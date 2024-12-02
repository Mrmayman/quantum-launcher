use std::{
    collections::{HashMap, HashSet},
    process::ExitStatus,
    sync::{mpsc::Receiver, Arc, Mutex},
    time::Instant,
};

use iced::widget::image::Handle;
use ql_instances::{
    err,
    error::{LauncherError, LauncherResult},
    file_utils, io_err,
    json_structs::{json_instance_config::InstanceConfigJson, json_version::VersionDetails},
    AssetRedownloadProgress, DownloadProgress, GameLaunchResult, JavaInstallProgress, LogLine,
    UpdateCheckInfo, UpdateProgress,
};
use ql_mod_manager::{
    instance_mod_installer::{
        fabric::{FabricInstallProgress, FabricVersion},
        forge::ForgeInstallProgress,
    },
    mod_manager::{ModConfig, ModIndex, ProjectInfo, Search},
};
use tokio::process::Child;

use crate::{
    config::LauncherConfig,
    stylesheet::styles::{LauncherStyle, LauncherTheme, STYLE},
};

#[derive(Debug, Clone)]
pub enum Message {
    OpenDir(String),
    InstallFabricEnd(Result<(), String>),
    InstallFabricVersionSelected(String),
    InstallFabricVersionsLoaded(Result<Vec<FabricVersion>, String>),
    LaunchInstanceSelected(String),
    LaunchUsernameSet(String),
    LaunchStart,
    DeleteInstanceMenu,
    DeleteInstance,
    LaunchScreenOpen(Option<String>),
    LaunchEnd(GameLaunchResult),
    LaunchKill,
    LaunchKillEnd(Result<(), String>),
    CreateInstanceScreenOpen,
    CreateInstanceVersionsLoaded(Result<Arc<Vec<String>>, String>),
    CreateInstanceVersionSelected(String),
    CreateInstanceNameInput(String),
    CreateInstanceStart,
    CreateInstanceEnd(Result<(), String>),
    CreateInstanceChangeAssetToggle(bool),
    EditInstance,
    EditInstanceJavaOverride(String),
    EditInstanceMemoryChanged(f32),
    EditInstanceLoggingToggle(bool),
    ManageModsScreenOpen,
    ManageModsToggleCheckbox((String, String), bool),
    ManageModsDeleteSelected,
    ManageModsDeleteFinished(Result<Vec<String>, String>),
    ManageModsToggleSelected,
    ManageModsToggleFinished(Result<(), String>),
    InstallFabricClicked,
    InstallFabricScreenOpen,
    InstallForgeStart,
    InstallForgeEnd(Result<(), String>),
    UninstallLoaderStart,
    UninstallLoaderEnd(Result<(), String>),
    ErrorCopy,
    Tick,
    TickConfigSaved(Result<(), String>),
    LaunchEndedLog(Result<ExitStatus, String>),
    LaunchCopyLog,
    UpdateCheckResult(Result<UpdateCheckInfo, String>),
    UpdateDownloadStart,
    UpdateDownloadEnd(Result<(), String>),
    InstallModsSearchResult(Result<(Search, Instant), String>),
    InstallModsOpen,
    InstallModsSearchInput(String),
    InstallModsImageDownloaded(Result<(String, Vec<u8>), String>),
    InstallModsClick(usize),
    InstallModsBackToMainScreen,
    InstallModsLoadData(Result<Box<ProjectInfo>, String>),
    InstallModsDownload(usize),
    InstallModsDownloadComplete(Result<String, String>),
    InstallOptifineScreenOpen,
    InstallOptifineSelectInstallerStart,
    InstallOptifineSelectInstallerEnd(Option<rfd::FileHandle>),
    LauncherSettingsThemePicked(String),
    LauncherSettingsStylePicked(String),
    LauncherSettingsOpen,
    ManageModsSelectAll,
    InstallOptifineEnd(Result<(), String>),
}

#[derive(Default)]
pub struct MenuLaunch {
    pub message: String,
    pub java_recv: Option<Receiver<JavaInstallProgress>>,
    pub asset_recv: Option<Receiver<AssetRedownloadProgress>>,
}

impl MenuLaunch {
    pub fn with_message(message: String) -> Self {
        Self {
            message,
            java_recv: None,
            asset_recv: None,
        }
    }
}

pub struct MenuEditInstance {
    pub config: InstanceConfigJson,
    pub slider_value: f32,
    pub slider_text: String,
}

#[derive(Hash, PartialEq, Eq)]
pub struct SelectedMod {
    pub name: String,
    pub id: String,
}

pub enum SelectedState {
    All,
    Some,
    None,
}

pub struct MenuEditMods {
    pub config: InstanceConfigJson,
    pub mods: ModIndex,
    pub selected_mods: HashSet<SelectedMod>,
    pub sorted_dependencies: Vec<(String, ModConfig)>,
    pub selected_state: SelectedState,
}

pub struct MenuCreateInstance {
    pub instance_name: String,
    pub selected_version: Option<String>,
    pub versions: Vec<String>,
    pub progress_receiver: Option<Receiver<DownloadProgress>>,
    pub progress_number: Option<f32>,
    pub progress_text: Option<String>,
    pub download_assets: bool,
    pub combo_state: iced::widget::combo_box::State<String>,
}

pub struct MenuDeleteInstance {}

pub struct MenuInstallFabric {
    pub fabric_version: Option<String>,
    pub fabric_versions: Vec<String>,
    pub progress_receiver: Option<Receiver<FabricInstallProgress>>,
    pub progress_num: f32,
}

pub struct MenuInstallForge {
    pub forge_progress_receiver: Receiver<ForgeInstallProgress>,
    pub forge_progress_num: f32,
    pub forge_message: String,
    pub java_progress_receiver: Receiver<JavaInstallProgress>,
    pub java_progress_num: f32,
    pub java_message: Option<String>,
    pub is_java_getting_installed: bool,
}

pub struct MenuLauncherUpdate {
    pub url: String,
    pub receiver: Option<Receiver<UpdateProgress>>,
    pub progress: f32,
    pub progress_message: Option<String>,
}

pub struct MenuInstallJava {
    pub num: f32,
    pub recv: Receiver<JavaInstallProgress>,
    pub message: String,
}

pub struct MenuRedownloadAssets {
    pub num: f32,
    pub recv: Receiver<AssetRedownloadProgress>,
    pub java_recv: Option<Receiver<JavaInstallProgress>>,
}
impl MenuRedownloadAssets {
    pub fn tick(&mut self) -> bool {
        while let Ok(progress) = self.recv.try_recv() {
            match progress {
                AssetRedownloadProgress::P1Start => {
                    self.num = 0.0;
                }
                AssetRedownloadProgress::P2Progress { done, out_of } => {
                    self.num = done as f32 / out_of as f32;
                }
                AssetRedownloadProgress::P3Done => {
                    return true;
                }
            }
        }
        false
    }
}

pub struct MenuModsDownload {
    pub query: String,
    pub results: Option<Search>,
    pub result_data: HashMap<String, ProjectInfo>,
    pub config: InstanceConfigJson,
    pub json: Box<VersionDetails>,
    pub opened_mod: Option<usize>,
    pub latest_load: Instant,
    pub is_loading_search: bool,
    pub mods_download_in_progress: HashSet<String>,
    pub mod_index: ModIndex,
}

pub struct MenuLauncherSettings;

pub enum State {
    Launch(MenuLaunch),
    EditInstance(MenuEditInstance),
    EditMods(MenuEditMods),
    Create(MenuCreateInstance),
    Error { error: String },
    DeleteInstance(MenuDeleteInstance),
    InstallFabric(MenuInstallFabric),
    InstallForge(MenuInstallForge),
    InstallOptifine(MenuInstallOptifine),
    InstallJava(MenuInstallJava),
    RedownloadAssets(MenuRedownloadAssets),
    UpdateFound(MenuLauncherUpdate),
    ModsDownload(MenuModsDownload),
    LauncherSettings,
}

pub struct MenuInstallOptifine {
    pub progress: Option<InstallOptifineProgress>,
}

pub struct InstallOptifineProgress {
    pub optifine_install_progress: Receiver<()>,
    pub optifine_install_num: f32,
    pub java_install_progress: Receiver<()>,
    pub java_install_num: f32,
}

pub struct InstanceLog {
    pub log: String,
    pub has_crashed: bool,
}

pub struct Launcher {
    pub state: State,
    pub selected_instance: Option<String>,
    pub instances: Option<Vec<String>>,
    pub config: Option<LauncherConfig>,
    pub processes: HashMap<String, GameProcess>,
    pub logs: HashMap<String, InstanceLog>,
    pub images: HashMap<String, Handle>,
    pub images_downloads_in_progress: HashSet<String>,
    pub images_to_load: Mutex<HashSet<String>>,
    pub theme: LauncherTheme,
    pub style: Arc<Mutex<LauncherStyle>>,
}

pub struct GameProcess {
    pub child: Arc<Mutex<Child>>,
    pub receiver: Receiver<LogLine>,
}

impl Launcher {
    pub fn new(message: Option<String>) -> LauncherResult<Self> {
        let subdirectories = reload_instances()?;

        let (config, theme, style) = load_config_and_theme()?;
        *STYLE.lock().unwrap() = style;

        Ok(Self {
            instances: Some(subdirectories),
            state: State::Launch(if let Some(message) = message {
                MenuLaunch::with_message(message)
            } else {
                MenuLaunch::default()
            }),
            processes: HashMap::new(),
            config,
            logs: HashMap::new(),
            selected_instance: None,
            images: HashMap::new(),
            images_downloads_in_progress: HashSet::new(),
            images_to_load: Mutex::new(HashSet::new()),
            theme,
            style: STYLE.clone(),
        })
    }

    pub fn with_error(error: &str) -> Self {
        let (config, theme, style) = load_config_and_theme().unwrap_or((
            None,
            LauncherTheme::default(),
            LauncherStyle::default(),
        ));
        *STYLE.lock().unwrap() = style;

        Self {
            state: State::Error {
                error: format!("Error: {error}"),
            },
            instances: None,
            config,
            processes: HashMap::new(),
            logs: HashMap::new(),
            selected_instance: None,
            images: HashMap::new(),
            images_downloads_in_progress: HashSet::new(),
            images_to_load: Mutex::new(HashSet::new()),
            theme,
            style: STYLE.clone(),
        }
    }

    pub fn set_error(&mut self, error: String) {
        self.state = State::Error { error }
    }

    pub fn go_to_launch_screen(&mut self) {
        self.state = State::Launch(MenuLaunch::default());
        if let Ok(list) = reload_instances() {
            self.instances = Some(list);
        } else {
            err!("Failed to reload instances list.");
        }
    }

    pub fn go_to_launch_screen_with_message(&mut self, message: String) {
        self.state = State::Launch(MenuLaunch::with_message(message));
        if let Ok(list) = reload_instances() {
            self.instances = Some(list);
        } else {
            err!("Failed to reload instances list.");
        }
    }

    pub fn edit_instance_wrapped(&mut self) {
        match self.edit_instance(self.selected_instance.clone().unwrap()) {
            Ok(()) => {}
            Err(err) => self.set_error(err.to_string()),
        }
    }
}

fn load_config_and_theme(
) -> Result<(Option<LauncherConfig>, LauncherTheme, LauncherStyle), LauncherError> {
    let config = LauncherConfig::load()?;
    let theme = match config.theme.as_deref() {
        Some("Dark") => LauncherTheme::Dark,
        Some("Light") => LauncherTheme::Light,
        None => LauncherTheme::default(),
        _ => {
            err!("Unknown style: {:?}", config.theme);
            LauncherTheme::default()
        }
    };
    let style = match config.style.as_deref() {
        Some("Brown") => LauncherStyle::Brown,
        Some("Purple") => LauncherStyle::Purple,
        None => LauncherStyle::default(),
        _ => {
            err!("Unknown style: {:?}", config.style);
            LauncherStyle::default()
        }
    };
    Ok((Some(config), theme, style))
}

pub fn reload_instances() -> Result<Vec<String>, LauncherError> {
    let dir_path = file_utils::get_launcher_dir()?;
    std::fs::create_dir_all(&dir_path).map_err(io_err!(dir_path))?;

    let dir_path = dir_path.join("instances");
    std::fs::create_dir_all(&dir_path).map_err(io_err!(dir_path))?;

    let dir = std::fs::read_dir(&dir_path).map_err(io_err!(dir_path))?;

    let subdirectories: Vec<String> = dir
        .filter_map(|entry| {
            if let Ok(entry) = entry {
                if entry.path().is_dir() {
                    if let Some(file_name) = entry.file_name().to_str() {
                        return Some(file_name.to_owned());
                    }
                }
            }
            None
        })
        .collect();
    Ok(subdirectories)
}
