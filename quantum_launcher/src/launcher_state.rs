use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    path::{Path, PathBuf},
    process::ExitStatus,
    sync::{mpsc::Receiver, Arc, Mutex},
    time::Instant,
};

use iced::{
    widget::{self, image::Handle},
    Task,
};
use ql_core::{
    err, file_utils,
    json::{instance_config::InstanceConfigJson, version::VersionDetails},
    DownloadProgress, GenericProgress, InstanceSelection, IntoIoError, IntoStringError,
    JsonFileError, Loader, ModId, Progress, SelectedMod, StoreBackendType, LAUNCHER_VERSION_NAME,
};
use ql_instances::{
    AccountData, AuthCodeResponse, AuthTokenResponse, ListEntry, LogLine, UpdateCheckInfo,
    CLIENT_ID,
};
use ql_mod_manager::{
    loaders::{
        fabric::FabricVersionListItem, forge::ForgeInstallProgress,
        optifine::OptifineInstallProgress,
    },
    mod_manager::{ImageResult, ModConfig, ModIndex, ModDescription, RecommendedMod, SearchResult},
};
use tokio::process::{Child, ChildStdin};

use crate::{
    config::{LauncherConfig, SIDEBAR_WIDTH_DEFAULT},
    message_handler::get_locally_installed_mods,
    stylesheet::styles::{LauncherTheme, LauncherThemeColor, LauncherThemeLightness},
    WINDOW_HEIGHT, WINDOW_WIDTH,
};

pub const OFFLINE_ACCOUNT_NAME: &str = "(Offline)";
pub const NEW_ACCOUNT_NAME: &str = "+ Add Account";

#[derive(Debug, Clone)]
pub enum InstallFabricMessage {
    End(Result<bool, String>),
    VersionSelected(String),
    VersionsLoaded(Result<Vec<FabricVersionListItem>, String>),
    ButtonClicked,
    ScreenOpen { is_quilt: bool },
}

#[derive(Debug, Clone)]
pub enum CreateInstanceMessage {
    ScreenOpen,
    VersionsLoaded(Result<Vec<ListEntry>, String>),
    VersionSelected(ListEntry),
    NameInput(String),
    Start,
    End(Result<String, String>),
    ChangeAssetToggle(bool),
    Cancel,
}

#[derive(Debug, Clone)]
pub enum EditInstanceMessage {
    ConfigSaved(Result<(), String>),
    JavaOverride(String),
    MemoryChanged(f32),
    LoggingToggle(bool),
    CloseLauncherToggle(bool),
    JavaArgsAdd,
    JavaArgEdit(String, usize),
    JavaArgDelete(usize),
    JavaArgShiftUp(usize),
    JavaArgShiftDown(usize),
    GameArgsAdd,
    GameArgEdit(String, usize),
    GameArgDelete(usize),
    GameArgShiftUp(usize),
    GameArgShiftDown(usize),
    RenameEdit(String),
    RenameApply,
}

#[derive(Debug, Clone)]
pub enum ManageModsMessage {
    ScreenOpen,
    ToggleCheckbox((String, ModId), bool),
    ToggleCheckboxLocal(String, bool),
    DeleteSelected,
    DeleteFinished(Result<Vec<ModId>, String>),
    LocalDeleteFinished(Result<(), String>),
    LocalIndexLoaded(HashSet<String>),
    ToggleSelected,
    ToggleFinished(Result<(), String>),
    UpdateMods,
    UpdateModsFinished(Result<(), String>),
    UpdateCheckResult(Result<Vec<(ModId, String)>, String>),
    UpdateCheckToggle(usize, bool),
    SelectAll,
}

#[derive(Debug, Clone)]
pub enum InstallModsMessage {
    SearchResult(Result<SearchResult, String>),
    Open,
    SearchInput(String),
    ImageDownloaded(Result<ImageResult, String>),
    Click(usize),
    BackToMainScreen,
    LoadData(Result<Box<ModDescription>, String>),
    Download(usize),
    DownloadComplete(Result<ModId, String>),
    IndexUpdated(Result<ModIndex, String>),
    ChangeBackend(StoreBackendType),
    Scrolled(widget::scrollable::Viewport),
}

#[derive(Debug, Clone)]
pub enum InstallOptifineMessage {
    ScreenOpen,
    SelectInstallerStart,
    SelectInstallerEnd(Option<rfd::FileHandle>),
    End(Result<(), String>),
}

#[derive(Debug, Clone)]
pub enum EditPresetsMessage {
    Open,
    TabChange(String),
    ToggleCheckbox((String, ModId), bool),
    ToggleCheckboxLocal(String, bool),
    SelectAll,
    BuildYourOwn,
    BuildYourOwnEnd(Result<Vec<u8>, String>),
    Load,
    LoadComplete(Result<(), String>),
    RecommendedModCheck(Result<Vec<RecommendedMod>, String>),
    RecommendedToggle(usize, bool),
    RecommendedDownload,
    RecommendedDownloadEnd(Result<(), String>),
}

#[derive(Debug, Clone)]
pub enum AccountMessage {
    Selected(String),
    Response1(Result<AuthCodeResponse, String>),
    Response2(Result<AuthTokenResponse, String>),
    Response3(Result<AccountData, String>),
    LogoutCheck,
    LogoutConfirm,
    RefreshComplete(Result<AccountData, String>),
}

#[derive(Debug, Clone)]
pub enum Message {
    #[allow(unused)]
    Nothing,

    Account(AccountMessage),
    CreateInstance(CreateInstanceMessage),
    EditInstance(EditInstanceMessage),
    ManageMods(ManageModsMessage),
    InstallMods(InstallModsMessage),
    InstallOptifine(InstallOptifineMessage),
    InstallFabric(InstallFabricMessage),
    EditPresets(EditPresetsMessage),
    CoreOpenDir(String),
    LaunchInstanceSelected {
        name: String,
        is_server: bool,
    },
    LaunchUsernameSet(String),
    LaunchStart,
    LaunchScreenOpen {
        message: Option<String>,
        clear_selection: bool,
    },
    LaunchEnd(Result<Arc<Mutex<Child>>, String>),
    LaunchKill,
    LaunchKillEnd(Result<(), String>),
    DeleteInstanceMenu,
    DeleteInstance,
    InstallForgeStart {
        is_neoforge: bool,
    },
    InstallForgeEnd(Result<(), String>),
    InstallPaperStart,
    InstallPaperEnd(Result<(), String>),
    UninstallLoaderConfirm(Box<Message>, String),
    UninstallLoaderFabricStart,
    UninstallLoaderForgeStart,
    UninstallLoaderOptiFineStart,
    UninstallLoaderPaperStart,
    UninstallLoaderEnd(Result<Loader, String>),

    CoreErrorCopy,
    CoreCopyText(String),
    CoreTick,
    CoreTickConfigSaved(Result<(), String>),
    CoreListLoaded(Result<(Vec<String>, bool), String>),
    CoreOpenChangeLog,
    CoreOpenIntro,
    CoreEvent(iced::Event, iced::event::Status),

    CoreLogToggle,
    CoreLogScroll(i64),
    CoreLogScrollAbsolute(i64),

    LaunchLogScroll(i64),
    LaunchLogScrollAbsolute(i64),

    LaunchEndedLog(Result<(ExitStatus, String), String>),
    LaunchCopyLog,
    LaunchChangeTab(LaunchTabId),

    UpdateCheckResult(Result<UpdateCheckInfo, String>),
    UpdateDownloadStart,
    UpdateDownloadEnd(Result<(), String>),

    LauncherSettingsOpen,
    LauncherSettingsThemePicked(String),
    LauncherSettingsStylePicked(String),
    LauncherSettingsUiScale(f64),
    LauncherSettingsUiScaleApply,

    ServerManageOpen {
        selected_server: Option<String>,
        message: Option<String>,
    },
    ServerManageStartServer(String),
    ServerManageStartServerFinish(Result<(Arc<Mutex<Child>>, bool), String>),
    ServerManageEndedLog(Result<(ExitStatus, String), String>),
    ServerManageKillServer(String),
    ServerManageEditCommand(String, String),
    ServerManageCopyLog,
    ServerManageSubmitCommand(String),

    ServerCreateScreenOpen,
    ServerCreateVersionsLoaded(Result<Vec<ListEntry>, String>),
    ServerCreateNameInput(String),
    ServerCreateVersionSelected(ListEntry),
    ServerCreateStart,
    ServerCreateEnd(Result<String, String>),
}

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
    pub asset_recv: Option<Receiver<GenericProgress>>,
    pub login_progress: Option<ProgressBar<GenericProgress>>,
    pub tab: LaunchTabId,
    pub edit_instance: Option<MenuEditInstance>,
    pub sidebar_width: u16,
    pub sidebar_dragging: bool,
    pub is_viewing_server: bool,
    pub log_scroll: i64,
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
            asset_recv: None,
            tab: LaunchTabId::default(),
            edit_instance: None,
            login_progress: None,
            sidebar_width: SIDEBAR_WIDTH_DEFAULT as u16,
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
    pub config: InstanceConfigJson,
    pub mods: ModIndex,
    pub locally_installed_mods: HashSet<String>,
    pub selected_mods: HashSet<SelectedMod>,
    pub sorted_mods_list: Vec<ModListEntry>,
    pub selected_state: SelectedState,
    pub available_updates: Vec<(ModId, String, bool)>,
    pub mod_update_progress: Option<ProgressBar<GenericProgress>>,
    pub drag_and_drop_hovered: bool,
}

impl MenuEditMods {
    pub fn update_locally_installed_mods(
        idx: &ModIndex,
        selected_instance: &InstanceSelection,
        dir: &Path,
    ) -> Task<Message> {
        let mut blacklist = Vec::new();
        for mod_info in idx.mods.values() {
            for file in &mod_info.files {
                blacklist.push(file.filename.clone());
                blacklist.push(format!("{}.disabled", file.filename));
            }
        }
        Task::perform(
            get_locally_installed_mods(selected_instance.get_dot_minecraft_path(dir), blacklist),
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

pub enum MenuCreateInstance {
    Loading {
        receiver: Receiver<()>,
        number: f32,
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
    Loading(bool),
    Loaded {
        is_quilt: bool,
        fabric_version: Option<String>,
        fabric_versions: Vec<String>,
        progress: Option<ProgressBar<GenericProgress>>,
    },
    Unsupported(bool),
}

impl MenuInstallFabric {
    pub fn is_quilt(&self) -> bool {
        match self {
            MenuInstallFabric::Loading(is_quilt)
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
    pub result_data: HashMap<ModId, ModDescription>,
    pub config: InstanceConfigJson,
    pub json: VersionDetails,
    pub opened_mod: Option<usize>,
    pub latest_load: Instant,
    pub mods_download_in_progress: HashSet<ModId>,
    pub mod_index: ModIndex,
    pub backend: StoreBackendType,

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

/// The enum that represents which menu is opened currently.
pub enum State {
    Welcome,
    Launch(MenuLaunch),
    ChangeLog,
    EditMods(MenuEditMods),
    Create(MenuCreateInstance),
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
        cancel_handle: iced::task::Handle,
    },
    InstallPaper,
    InstallFabric(MenuInstallFabric),
    InstallForge(MenuInstallForge),
    InstallOptifine(MenuInstallOptifine),
    InstallJava,
    RedownloadAssets {
        progress: ProgressBar<GenericProgress>,
    },
    UpdateFound(MenuLauncherUpdate),
    ModsDownload(Box<MenuModsDownload>),
    LauncherSettings(MenuLauncherSettings),
    ServerCreate(MenuServerCreate),
    ManagePresets(MenuEditPresets),
}

pub enum MenuServerCreate {
    LoadingList {
        progress_receiver: Receiver<()>,
        progress_number: f32,
    },
    Loaded {
        name: String,
        versions: iced::widget::combo_box::State<ListEntry>,
        selected_version: Option<ListEntry>,
    },
    Downloading {
        progress: ProgressBar<GenericProgress>,
    },
}

#[derive(Default)]
pub struct MenuInstallOptifine {
    pub optifine_install_progress: Option<ProgressBar<OptifineInstallProgress>>,
    pub java_install_progress: Option<ProgressBar<GenericProgress>>,
    pub is_java_being_installed: bool,
}

pub struct InstanceLog {
    pub log: Vec<String>,
    pub has_crashed: bool,
    pub command: String,
}

pub struct Launcher {
    pub state: State,
    pub dir: PathBuf,
    pub selected_instance: Option<InstanceSelection>,
    pub config: LauncherConfig,
    pub theme: LauncherTheme,
    pub images: ImageState,

    pub is_log_open: bool,
    pub log_scroll: i64,

    pub java_recv: Option<ProgressBar<GenericProgress>>,

    pub accounts: HashMap<String, AccountData>,
    pub accounts_dropdown: Vec<String>,
    pub accounts_selected: Option<String>,

    pub client_version_list_cache: Option<Vec<ListEntry>>,
    pub server_version_list_cache: Option<Vec<ListEntry>>,
    pub client_list: Option<Vec<String>>,
    pub server_list: Option<Vec<String>>,
    pub client_processes: HashMap<String, ClientProcess>,
    pub server_processes: HashMap<String, ServerProcess>,
    pub client_logs: HashMap<String, InstanceLog>,
    pub server_logs: HashMap<String, InstanceLog>,

    pub window_size: (f32, f32),
    pub mouse_pos: (f32, f32),
    pub keys_pressed: HashSet<iced::keyboard::Key>,
}

#[derive(Default)]
pub struct ImageState {
    pub bitmap: HashMap<String, Handle>,
    pub svg: HashMap<String, iced::widget::svg::Handle>,
    pub downloads_in_progress: HashSet<String>,
    pub to_load: Mutex<HashSet<String>>,
}

pub struct ClientProcess {
    pub child: Arc<Mutex<Child>>,
    pub receiver: Option<Receiver<LogLine>>,
}

pub struct ServerProcess {
    pub child: Arc<Mutex<Child>>,
    pub receiver: Option<Receiver<String>>,
    pub stdin: Option<ChildStdin>,
    pub is_classic_server: bool,
    pub has_issued_stop_command: bool,
}

impl Launcher {
    pub fn load_new(message: Option<String>, is_new_user: bool) -> Result<Self, JsonFileError> {
        let launcher_dir = match file_utils::get_launcher_dir_s() {
            Ok(n) => n,
            Err(err) => {
                err!("Could not get launcher dir (This is a bug): {err}");
                return Ok(Self::with_error(format!(
                    "Could not get launcher dir: {err}"
                )));
            }
        };

        let (mut config, theme) = load_config_and_theme(&launcher_dir)?;

        let mut launch = if let Some(message) = message {
            MenuLaunch::with_message(message)
        } else {
            MenuLaunch::default()
        };

        if let Some(sidebar_width) = config.sidebar_width {
            launch.sidebar_width = sidebar_width as u16;
        }

        let launch = State::Launch(launch);

        // The version field was added in 0.3
        let version = config.version.as_deref().unwrap_or("0.3.0");

        let state = if is_new_user {
            State::Welcome
        } else if version == LAUNCHER_VERSION_NAME {
            launch
        } else {
            config.version = Some(LAUNCHER_VERSION_NAME.to_owned());
            State::ChangeLog
        };

        let mut accounts = HashMap::new();

        let mut accounts_dropdown =
            vec![OFFLINE_ACCOUNT_NAME.to_owned(), NEW_ACCOUNT_NAME.to_owned()];

        if let Some(config_accounts) = &config.accounts {
            for (username, account) in config_accounts {
                match ql_instances::read_refresh_token(username) {
                    Ok(refresh_token) => {
                        accounts_dropdown.insert(0, username.clone());
                        accounts.insert(
                            username.clone(),
                            AccountData {
                                access_token: None,
                                uuid: account.uuid.clone(),
                                username: username.clone(),
                                refresh_token,
                                needs_refresh: true,
                            },
                        );
                    }
                    Err(err) => {
                        err!("Could not load account: {err}");
                    }
                }
            }
        }

        let selected_account = accounts_dropdown
            .first()
            .cloned()
            .unwrap_or_else(|| OFFLINE_ACCOUNT_NAME.to_owned());

        Ok(Self {
            dir: launcher_dir,
            client_list: None,
            server_list: None,
            java_recv: None,
            is_log_open: false,
            log_scroll: 0,
            state,
            client_processes: HashMap::new(),
            config,
            client_logs: HashMap::new(),
            selected_instance: None,
            images: ImageState::default(),
            theme,
            client_version_list_cache: None,
            server_version_list_cache: None,
            server_processes: HashMap::new(),
            server_logs: HashMap::new(),
            mouse_pos: (0.0, 0.0),
            window_size: (WINDOW_WIDTH, WINDOW_HEIGHT),
            accounts,
            accounts_dropdown,
            accounts_selected: Some(selected_account),
            keys_pressed: HashSet::new(),
        })
    }

    pub fn with_error(error: impl std::fmt::Display) -> Self {
        let mut error = error.to_string();
        let launcher_dir = if error.contains("Could not get launcher dir") {
            None
        } else {
            match file_utils::get_launcher_dir_s() {
                Ok(n) => Some(n),
                Err(err) => {
                    err!("Could not get launcher dir! This is a bug! {err}");
                    error.push_str(&format!("\n\n{err}"));
                    None
                }
            }
        };

        let (config, theme) = launcher_dir
            .as_ref()
            .and_then(|n| match load_config_and_theme(n) {
                Ok(n) => Some(n),
                Err(err) => {
                    err!("Error loading config: {err}");
                    None
                }
            })
            .unwrap_or((LauncherConfig::default(), LauncherTheme::default()));

        Self {
            dir: launcher_dir.unwrap_or_default(),
            state: State::Error { error },
            is_log_open: false,
            log_scroll: 0,
            java_recv: None,
            client_list: None,
            server_list: None,
            config,
            client_processes: HashMap::new(),
            client_logs: HashMap::new(),
            selected_instance: None,
            images: ImageState::default(),
            theme,
            client_version_list_cache: None,
            server_processes: HashMap::new(),
            server_logs: HashMap::new(),
            server_version_list_cache: None,
            mouse_pos: (0.0, 0.0),
            window_size: (WINDOW_WIDTH, WINDOW_HEIGHT),
            accounts: HashMap::new(),
            accounts_dropdown: vec![OFFLINE_ACCOUNT_NAME.to_owned(), NEW_ACCOUNT_NAME.to_owned()],
            accounts_selected: Some(OFFLINE_ACCOUNT_NAME.to_owned()),
            keys_pressed: HashSet::new(),
        }
    }

    pub fn set_error(&mut self, error: impl ToString) {
        self.state = State::Error {
            error: error.to_string().replace(CLIENT_ID, "[CLIENT ID]"),
        }
    }

    pub fn go_to_launch_screen<T: Display>(&mut self, message: Option<T>) -> Task<Message> {
        let mut menu_launch = match message {
            Some(message) => MenuLaunch::with_message(message.to_string()),
            None => MenuLaunch::default(),
        };
        if let Some(width) = self.config.sidebar_width {
            menu_launch.sidebar_width = width as u16;
        }
        self.state = State::Launch(menu_launch);
        Task::perform(
            get_entries("instances".to_owned(), false),
            Message::CoreListLoaded,
        )
    }
}

fn load_config_and_theme(
    launcher_dir: &Path,
) -> Result<(LauncherConfig, LauncherTheme), JsonFileError> {
    let config = LauncherConfig::load(launcher_dir)?;
    let theme = match config.theme.as_deref() {
        Some("Dark") => LauncherThemeLightness::Dark,
        Some("Light") => LauncherThemeLightness::Light,
        None => LauncherThemeLightness::default(),
        _ => {
            err!("Unknown style: {:?}", config.theme);
            LauncherThemeLightness::default()
        }
    };
    let style = match config.style.as_deref() {
        Some("Brown") => LauncherThemeColor::Brown,
        Some("Purple") => LauncherThemeColor::Purple,
        Some("Sky Blue") => LauncherThemeColor::SkyBlue,
        None => LauncherThemeColor::default(),
        _ => {
            err!("Unknown style: {:?}", config.style);
            LauncherThemeColor::default()
        }
    };
    let theme = LauncherTheme::from_vals(style, theme);
    Ok((config, theme))
}

pub async fn get_entries(path: String, is_server: bool) -> Result<(Vec<String>, bool), String> {
    let dir_path = file_utils::get_launcher_dir().await.strerr()?.join(path);
    if !dir_path.exists() {
        tokio::fs::create_dir_all(&dir_path)
            .await
            .path(&dir_path)
            .strerr()?;
        return Ok((Vec::new(), is_server));
    }

    let mut dir = tokio::fs::read_dir(&dir_path)
        .await
        .path(dir_path)
        .strerr()?;

    let mut subdirectories = Vec::new();

    while let Ok(Some(entry)) = dir.next_entry().await {
        if entry.path().is_dir() {
            if let Some(file_name) = entry.file_name().to_str() {
                subdirectories.push(file_name.to_owned());
            }
        }
    }

    Ok((subdirectories, is_server))
}

pub struct ProgressBar<T: Progress> {
    pub num: f32,
    pub message: Option<String>,
    pub receiver: Receiver<T>,
    pub progress: T,
}

impl<T: Default + Progress> ProgressBar<T> {
    pub fn with_recv(receiver: Receiver<T>) -> Self {
        Self {
            num: 0.0,
            message: None,
            receiver,
            progress: T::default(),
        }
    }

    pub fn with_recv_and_msg(receiver: Receiver<T>, msg: String) -> Self {
        Self {
            num: 0.0,
            message: Some(msg),
            receiver,
            progress: T::default(),
        }
    }
}

impl<T: Progress> ProgressBar<T> {
    pub fn tick(&mut self) -> bool {
        let mut has_ticked = false;
        while let Ok(progress) = self.receiver.try_recv() {
            self.num = progress.get_num();
            self.message = progress.get_message();
            self.progress = progress;
            has_ticked = true;
        }
        has_ticked
    }
}
