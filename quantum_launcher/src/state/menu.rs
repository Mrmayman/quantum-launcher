use std::{
    collections::{HashMap, HashSet},
    sync::Mutex,
    time::Instant,
};

use iced::{widget::scrollable::AbsoluteOffset, Task};
use ql_core::{
    file_utils::DirItem,
    jarmod::JarMods,
    json::{InstanceConfigJson, VersionDetails},
    DownloadProgress, GenericProgress, InstanceSelection, ListEntry, ModId, OptifineUniqueVersion,
    SelectedMod, StoreBackendType,
};
use ql_mod_manager::{
    loaders::{forge::ForgeInstallProgress, optifine::OptifineInstallProgress},
    store::{CurseforgeNotAllowed, ModConfig, ModIndex, QueryType, RecommendedMod, SearchResult},
};

use crate::{config::SIDEBAR_WIDTH_DEFAULT, message_handler::get_locally_installed_mods};

use super::{ManageModsMessage, Message, ProgressBar};

#[derive(Clone, PartialEq, Eq, Debug, Default, Copy)]
pub enum LaunchTabId {
    #[default]
    Buttons,
    Log,
    Edit,
}

impl std::fmt::Display for LaunchTabId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                LaunchTabId::Buttons => "Play",
                LaunchTabId::Log => "Log",
                LaunchTabId::Edit => "Edit",
            }
        )
    }
}

/// The home screen of the launcher.
pub struct MenuLaunch {
    pub message: String,
    pub login_progress: Option<ProgressBar<GenericProgress>>,
    pub tab: LaunchTabId,
    pub edit_instance: Option<MenuEditInstance>,

    pub sidebar_width: u16,
    pub sidebar_height: f32,
    pub sidebar_dragging: bool,

    pub is_viewing_server: bool,
    pub log_scroll: isize,
}

impl Default for MenuLaunch {
    fn default() -> Self {
        Self::with_message(String::new())
    }
}

impl MenuLaunch {
    pub fn with_message(message: String) -> Self {
        Self {
            message,
            tab: LaunchTabId::default(),
            edit_instance: None,
            login_progress: None,
            sidebar_width: SIDEBAR_WIDTH_DEFAULT as u16,
            sidebar_height: 100.0,
            sidebar_dragging: false,
            is_viewing_server: false,
            log_scroll: 0,
        }
    }
}

/// The screen where you can edit an instance/server.
pub struct MenuEditInstance {
    pub config: InstanceConfigJson,
    pub instance_name: String,
    pub old_instance_name: String,
    pub slider_value: f32,
    pub slider_text: String,
}

pub enum SelectedState {
    All,
    Some,
    None,
}

#[derive(Debug, Clone)]
pub enum ModListEntry {
    Downloaded { id: ModId, config: Box<ModConfig> },
    Local { file_name: String },
}

impl ModListEntry {
    pub fn is_manually_installed(&self) -> bool {
        match self {
            ModListEntry::Local { .. } => true,
            ModListEntry::Downloaded { config, .. } => config.manually_installed,
        }
    }

    pub fn name(&self) -> String {
        match self {
            ModListEntry::Local { file_name } => file_name.clone(),
            ModListEntry::Downloaded { config, .. } => config.name.clone(),
        }
    }

    pub fn id(&self) -> SelectedMod {
        match self {
            ModListEntry::Local { file_name } => SelectedMod::Local {
                file_name: file_name.clone(),
            },
            ModListEntry::Downloaded { id, config } => SelectedMod::Downloaded {
                name: config.name.clone(),
                id: id.clone(),
            },
        }
    }
}

pub struct MenuEditMods {
    pub mod_update_progress: Option<ProgressBar<GenericProgress>>,

    pub config: InstanceConfigJson,
    pub mods: ModIndex,

    pub locally_installed_mods: HashSet<String>,
    pub sorted_mods_list: Vec<ModListEntry>,

    pub selected_mods: HashSet<SelectedMod>,
    pub selected_state: SelectedState,

    pub update_check_handle: Option<iced::task::Handle>,
    pub available_updates: Vec<(ModId, String, bool)>,
    pub drag_and_drop_hovered: bool,
}

impl MenuEditMods {
    pub fn update_locally_installed_mods(
        idx: &ModIndex,
        selected_instance: &InstanceSelection,
    ) -> Task<Message> {
        let mut blacklist = Vec::new();
        for mod_info in idx.mods.values() {
            for file in &mod_info.files {
                blacklist.push(file.filename.clone());
                blacklist.push(format!("{}.disabled", file.filename));
            }
        }
        Task::perform(
            get_locally_installed_mods(selected_instance.get_dot_minecraft_path(), blacklist),
            |n| Message::ManageMods(ManageModsMessage::LocalIndexLoaded(n)),
        )
    }

    /// Returns two `Vec`s that are:
    /// - The IDs of downloaded mods
    /// - The filenames of local mods
    ///
    /// ...respectively, from the mods selected in the mod menu.
    pub fn get_kinds_of_ids(&self) -> (Vec<String>, Vec<String>) {
        let ids_downloaded = self
            .selected_mods
            .iter()
            .filter_map(|s_mod| {
                if let SelectedMod::Downloaded { id, .. } = s_mod {
                    Some(id.get_index_str())
                } else {
                    None
                }
            })
            .collect();

        let ids_local: Vec<String> = self
            .selected_mods
            .iter()
            .filter_map(|s_mod| {
                if let SelectedMod::Local { file_name } = s_mod {
                    Some(file_name.clone())
                } else {
                    None
                }
            })
            .collect();
        (ids_downloaded, ids_local)
    }
}

pub struct MenuEditJarMods {
    pub jarmods: JarMods,
    pub selected_state: SelectedState,
    pub selected_mods: HashSet<String>,
    pub drag_and_drop_hovered: bool,
    pub free_for_autosave: bool,
}

pub enum MenuCreateInstance {
    Loading {
        _handle: iced::task::Handle,
    },
    Loaded {
        instance_name: String,
        selected_version: Option<ListEntry>,
        progress: Option<ProgressBar<DownloadProgress>>,
        download_assets: bool,
        combo_state: Box<iced::widget::combo_box::State<ListEntry>>,
    },
}

pub enum MenuInstallFabric {
    Loading {
        is_quilt: bool,
        _loading_handle: iced::task::Handle,
    },
    Loaded {
        is_quilt: bool,
        fabric_version: String,
        fabric_versions: Vec<String>,
        progress: Option<ProgressBar<GenericProgress>>,
    },
    Unsupported(bool),
}

impl MenuInstallFabric {
    pub fn is_quilt(&self) -> bool {
        match self {
            MenuInstallFabric::Loading { is_quilt, .. }
            | MenuInstallFabric::Loaded { is_quilt, .. }
            | MenuInstallFabric::Unsupported(is_quilt) => *is_quilt,
        }
    }
}

pub struct MenuInstallForge {
    pub forge_progress: ProgressBar<ForgeInstallProgress>,
    pub java_progress: ProgressBar<GenericProgress>,
    pub is_java_getting_installed: bool,
}

pub struct MenuLauncherUpdate {
    pub url: String,
    pub progress: Option<ProgressBar<GenericProgress>>,
}

pub struct MenuModsDownload {
    pub query: String,
    pub results: Option<SearchResult>,
    pub mod_descriptions: HashMap<ModId, String>,
    pub json: Mutex<VersionDetails>,
    pub opened_mod: Option<usize>,
    pub latest_load: Instant,
    pub mods_download_in_progress: HashSet<ModId>,
    pub scroll_offset: AbsoluteOffset,

    pub config: InstanceConfigJson,
    pub mod_index: ModIndex,

    pub backend: StoreBackendType,
    pub query_type: QueryType,

    /// This is for the loading of continuation of the search,
    /// ie. when you scroll down and more stuff appears
    pub is_loading_continuation: bool,
}

pub struct MenuLauncherSettings {
    pub temp_scale: f64,
}

pub struct MenuEditPresets {
    pub inner: MenuEditPresetsInner,
    pub recommended_mods: Option<Vec<(bool, RecommendedMod)>>,
    pub progress: Option<ProgressBar<GenericProgress>>,
    pub config: InstanceConfigJson,
    pub sorted_mods_list: Vec<ModListEntry>,
    pub drag_and_drop_hovered: bool,
}

pub enum MenuEditPresetsInner {
    Build {
        selected_mods: HashSet<SelectedMod>,
        selected_state: SelectedState,
        is_building: bool,
    },
    Recommended {
        error: Option<String>,
        progress: ProgressBar<GenericProgress>,
    },
}

pub const PRESET_INNER_BUILD: &str = "Create";
pub const PRESET_INNER_RECOMMENDED: &str = "Recommended";

impl MenuEditPresetsInner {
    #[must_use]
    pub const fn id(&self) -> &'static str {
        match self {
            MenuEditPresetsInner::Build { .. } => PRESET_INNER_BUILD,
            MenuEditPresetsInner::Recommended { .. } => PRESET_INNER_RECOMMENDED,
        }
    }
}

pub enum MenuWelcome {
    P1InitialScreen,
    P2Theme,
    P3Auth,
}

pub struct MenuCurseforgeManualDownload {
    pub unsupported: HashSet<CurseforgeNotAllowed>,
    pub is_store: bool,
}

pub struct MenuExportInstance {
    pub entries: Vec<(DirItem, bool)>,
}

/// The enum that represents which menu is opened currently.
pub enum State {
    /// Default home screen
    Launch(MenuLaunch),
    Create(MenuCreateInstance),
    /// Screen to guide new users to the launcher
    Welcome(MenuWelcome),
    ChangeLog,
    UpdateFound(MenuLauncherUpdate),

    EditMods(MenuEditMods),
    EditJarMods(MenuEditJarMods),
    ImportModpack(ProgressBar<GenericProgress>),
    CurseforgeManualDownload(MenuCurseforgeManualDownload),
    ExportInstance(MenuExportInstance),

    Error {
        error: String,
    },
    ConfirmAction {
        msg1: String,
        msg2: String,
        yes: Message,
        no: Message,
    },
    GenericMessage(String),

    AccountLoginProgress(ProgressBar<GenericProgress>),
    AccountLogin {
        url: String,
        code: String,
        _cancel_handle: iced::task::Handle,
    },

    InstallPaper,
    InstallFabric(MenuInstallFabric),
    InstallForge(MenuInstallForge),
    InstallOptifine(MenuInstallOptifine),

    InstallJava,

    ModsDownload(Box<MenuModsDownload>),
    LauncherSettings(MenuLauncherSettings),
    ServerCreate(MenuServerCreate),
    ManagePresets(MenuEditPresets),
}

pub enum MenuServerCreate {
    LoadingList,
    Loaded {
        name: String,
        versions: Box<iced::widget::combo_box::State<ListEntry>>,
        selected_version: Option<ListEntry>,
    },
    Downloading {
        progress: ProgressBar<GenericProgress>,
    },
}

pub struct MenuInstallOptifine {
    pub optifine_install_progress: Option<ProgressBar<OptifineInstallProgress>>,
    pub java_install_progress: Option<ProgressBar<GenericProgress>>,
    pub is_java_being_installed: bool,
    pub is_b173_being_installed: bool,
    pub optifine_unique_version: Option<OptifineUniqueVersion>,
}

impl MenuInstallOptifine {
    pub fn get_url(&self) -> &'static str {
        const OPTIFINE_DOWNLOADS: &str = "https://optifine.net/downloads";

        self.optifine_unique_version
            .as_ref()
            .map_or(OPTIFINE_DOWNLOADS, |n| n.get_url().0)
    }
}
