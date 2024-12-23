use std::{
    collections::{HashMap, HashSet},
    process::ExitStatus,
    sync::{mpsc::Receiver, Arc, Mutex},
    time::Instant,
};

use iced::{widget::image::Handle, Command};
use ql_core::{
    err, file_utils, io_err,
    json::{instance_config::InstanceConfigJson, version::VersionDetails},
    DownloadProgress, IoError, JavaInstallProgress, JsonFileError,
};
use ql_instances::{
    AssetRedownloadProgress, GameLaunchResult, ListEntry, LogLine, ScrapeProgress, UpdateCheckInfo,
    UpdateProgress,
};
use ql_mod_manager::{
    instance_mod_installer::{
        fabric::{FabricInstallProgress, FabricVersionListItem},
        forge::ForgeInstallProgress,
        optifine::OptifineInstallProgress,
    },
    mod_manager::{ApplyUpdateProgress, Loader, ModConfig, ModIndex, ProjectInfo, Search},
};
use ql_servers::ServerCreateProgress;
use tokio::process::{Child, ChildStdin};

use crate::{
    config::LauncherConfig,
    message_handler::get_locally_installed_mods,
    stylesheet::styles::{LauncherStyle, LauncherTheme, STYLE},
};

#[derive(Debug, Clone)]
pub enum InstallFabricMessage {
    End(Result<(), String>),
    VersionSelected(String),
    VersionsLoaded(Result<Vec<FabricVersionListItem>, String>),
    ButtonClicked,
    ScreenOpen,
}

#[derive(Debug, Clone)]
pub enum CreateInstanceMessage {
    ScreenOpen,
    VersionsLoaded(Result<Vec<ListEntry>, String>),
    VersionSelected(ListEntry),
    NameInput(String),
    Start,
    End(Result<(), String>),
    ChangeAssetToggle(bool),
}

#[derive(Debug, Clone)]
pub enum EditInstanceMessage {
    /// The `Option` represents the selected server, if any.
    /// - If `None` then you are editing the client instance.
    /// - If `Some` then you are editing the server instance.
    MenuOpen(Option<String>),
    JavaOverride(String),
    MemoryChanged(f32),
    LoggingToggle(bool),
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
}

#[derive(Debug, Clone)]
pub enum Message {
    InstallFabric(InstallFabricMessage),
    CreateInstance(CreateInstanceMessage),
    EditInstance(EditInstanceMessage),
    CoreOpenDir(String),
    LaunchInstanceSelected(String),
    LaunchUsernameSet(String),
    LaunchStart,
    LaunchScreenOpen(Option<String>),
    LaunchEnd(GameLaunchResult),
    LaunchKill,
    LaunchKillEnd(Result<(), String>),
    DeleteInstanceMenu,
    DeleteInstance,
    ManageModsScreenOpen,
    ManageModsToggleCheckbox((String, String), bool),
    ManageModsToggleCheckboxLocal(String, bool),
    ManageModsDeleteSelected,
    ManageModsDeleteFinished(Result<Vec<String>, String>),
    ManageModsLocalDeleteFinished(Result<(), String>),
    ManageModsLocalIndexLoaded(HashSet<String>),
    ManageModsToggleSelected,
    ManageModsToggleFinished(Result<(), String>),
    ManageModsUpdateMods,
    ManageModsUpdateModsFinished(Result<(), String>),
    InstallForgeStart,
    InstallForgeEnd(Result<(), String>),
    UninstallLoaderFabricStart,
    UninstallLoaderForgeStart,
    UninstallLoaderOptiFineStart,
    UninstallLoaderEnd(Result<Loader, String>),
    CoreErrorCopy,
    CoreTick,
    CoreTickConfigSaved(Result<(), String>),
    LaunchEndedLog(Result<(ExitStatus, String), String>),
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
    ManageModsUpdateCheckResult(Option<Vec<(String, String)>>),
    ManageModsUpdateCheckToggle(usize, bool),
    ManageModsSelectAll,
    InstallOptifineScreenOpen,
    InstallOptifineSelectInstallerStart,
    InstallOptifineSelectInstallerEnd(Option<rfd::FileHandle>),
    InstallOptifineEnd(Result<(), String>),
    LauncherSettingsThemePicked(String),
    LauncherSettingsStylePicked(String),
    LauncherSettingsOpen,
    // The `Option` represents the selected server, if any.
    ServerManageOpen(Option<String>),
    ServerManageSelectedServer(String),
    ServerManageStartServer(String),
    ServerManageStartServerFinish(Result<(Arc<Mutex<Child>>, bool), String>),
    ServerManageEndedLog(Result<(ExitStatus, String), String>),
    ServerManageKillServer(String),
    ServerManageEditCommand(String, String),
    ServerManageSubmitCommand(String),
    ServerCreateScreenOpen,
    ServerCreateVersionsLoaded(Result<Vec<ListEntry>, String>),
    ServerCreateNameInput(String),
    ServerCreateVersionSelected(ListEntry),
    ServerCreateStart,
    ServerCreateEnd(Result<String, String>),
    ServerDeleteOpen(String),
    ServerDeleteConfirm,
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

impl MenuEditInstance {
    pub fn save_server_config(&self, selected_server: &str) -> Result<(), JsonFileError> {
        let mut config = self.config.clone();
        if config.enable_logger.is_none() {
            config.enable_logger = Some(true);
        }
        let launcher_dir = file_utils::get_launcher_dir()?;
        let config_path = launcher_dir
            .join("servers")
            .join(selected_server)
            .join("config.json");

        let config_json = serde_json::to_string(&config)?;
        std::fs::write(&config_path, config_json).map_err(io_err!(config_path))?;
        Ok(())
    }
}

#[derive(Hash, PartialEq, Eq)]
pub enum SelectedMod {
    Downloaded { name: String, id: String },
    Local { file_name: String },
}

pub enum SelectedState {
    All,
    Some,
    None,
}

#[derive(Debug)]
pub enum ModListEntry {
    Downloaded { id: String, config: Box<ModConfig> },
    Local { file_name: String },
}

pub struct MenuEditMods {
    pub config: InstanceConfigJson,
    pub mods: ModIndex,
    pub locally_installed_mods: HashSet<String>,
    pub selected_mods: HashSet<SelectedMod>,
    pub sorted_mods_list: Vec<ModListEntry>,
    pub selected_state: SelectedState,
    pub available_updates: Vec<(String, String, bool)>,
    pub mod_update_progress: Option<UpdateModsProgress>,
}

impl MenuEditMods {
    pub fn update_locally_installed_mods(
        idx: &ModIndex,
        selected_instance: String,
    ) -> Command<Message> {
        let mut blacklist = Vec::new();
        for mod_info in idx.mods.values() {
            for file in &mod_info.files {
                blacklist.push(file.filename.clone());
            }
        }
        Command::perform(
            get_locally_installed_mods(selected_instance, blacklist),
            Message::ManageModsLocalIndexLoaded,
        )
    }
}

pub enum MenuCreateInstance {
    Loading {
        progress_receiver: Receiver<ScrapeProgress>,
        progress_number: f32,
    },
    Loaded {
        instance_name: String,
        selected_version: Option<ListEntry>,
        progress_receiver: Option<Receiver<DownloadProgress>>,
        progress_number: Option<f32>,
        progress_text: Option<String>,
        download_assets: bool,
        combo_state: Box<iced::widget::combo_box::State<ListEntry>>,
    },
}

pub enum MenuInstallFabric {
    Loading,
    Loaded {
        fabric_version: Option<String>,
        fabric_versions: Vec<String>,
        progress_receiver: Option<Receiver<FabricInstallProgress>>,
        progress_num: f32,
        progress_message: String,
    },
    Unsupported,
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
    DeleteInstance,
    InstallFabric(MenuInstallFabric),
    InstallForge(MenuInstallForge),
    InstallOptifine(MenuInstallOptifine),
    InstallJava(MenuInstallJava),
    RedownloadAssets(MenuRedownloadAssets),
    UpdateFound(MenuLauncherUpdate),
    ModsDownload(Box<MenuModsDownload>),
    LauncherSettings,
    ServerManage(MenuServerManage),
    ServerCreate(MenuServerCreate),
    ServerDelete { selected_server: String },
}

pub struct MenuServerManage {
    pub server_list: Vec<String>,
    pub java_install_recv: Option<Receiver<JavaInstallProgress>>,
}

pub enum MenuServerCreate {
    Loading {
        progress_receiver: Receiver<ScrapeProgress>,
        progress_number: f32,
    },
    Loaded {
        name: String,
        versions: iced::widget::combo_box::State<ListEntry>,
        selected_version: Option<ListEntry>,
        progress_receiver: Option<Receiver<ServerCreateProgress>>,
        progress_number: f32,
    },
}

pub struct UpdateModsProgress {
    pub recv: Receiver<ApplyUpdateProgress>,
    pub num: f32,
    pub message: String,
}

pub struct MenuInstallOptifine {
    pub progress: Option<OptifineInstallProgressData>,
}

pub struct OptifineInstallProgressData {
    pub optifine_install_progress: Receiver<OptifineInstallProgress>,
    pub optifine_install_num: f32,
    pub optifine_install_message: String,
    pub java_install_progress: Receiver<JavaInstallProgress>,
    pub java_install_num: f32,
    pub java_install_message: String,
    pub is_java_being_installed: bool,
}

pub struct InstanceLog {
    pub log: String,
    pub has_crashed: bool,
    pub command: String,
}

pub struct Launcher {
    pub state: State,
    pub selected_instance: Option<String>,
    pub selected_server: Option<String>,
    pub client_version_list_cache: Option<Vec<ListEntry>>,
    pub server_version_list_cache: Option<Vec<ListEntry>>,
    pub instances: Option<Vec<String>>,
    pub config: Option<LauncherConfig>,
    pub client_processes: HashMap<String, ClientProcess>,
    pub server_processes: HashMap<String, ServerProcess>,
    pub client_logs: HashMap<String, InstanceLog>,
    pub server_logs: HashMap<String, InstanceLog>,
    pub images: HashMap<String, Handle>,
    pub images_downloads_in_progress: HashSet<String>,
    pub images_to_load: Mutex<HashSet<String>>,
    pub theme: LauncherTheme,
    pub style: Arc<Mutex<LauncherStyle>>,
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
}

impl Launcher {
    pub fn new(message: Option<String>) -> Result<Self, JsonFileError> {
        let subdirectories = get_entries("instances")?;

        let (config, theme, style) = load_config_and_theme()?;
        *STYLE.lock().unwrap() = style;

        Ok(Self {
            instances: Some(subdirectories),
            state: State::Launch(if let Some(message) = message {
                MenuLaunch::with_message(message)
            } else {
                MenuLaunch::default()
            }),
            client_processes: HashMap::new(),
            config,
            client_logs: HashMap::new(),
            selected_instance: None,
            images: HashMap::new(),
            images_downloads_in_progress: HashSet::new(),
            images_to_load: Mutex::new(HashSet::new()),
            theme,
            style: STYLE.clone(),
            client_version_list_cache: None,
            server_version_list_cache: None,
            selected_server: None,
            server_processes: HashMap::new(),
            server_logs: HashMap::new(),
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
            client_processes: HashMap::new(),
            client_logs: HashMap::new(),
            selected_instance: None,
            images: HashMap::new(),
            images_downloads_in_progress: HashSet::new(),
            images_to_load: Mutex::new(HashSet::new()),
            theme,
            style: STYLE.clone(),
            client_version_list_cache: None,
            selected_server: None,
            server_processes: HashMap::new(),
            server_logs: HashMap::new(),
            server_version_list_cache: None,
        }
    }

    pub fn set_error(&mut self, error: String) {
        self.state = State::Error { error }
    }

    pub fn go_to_launch_screen(&mut self) {
        self.state = State::Launch(MenuLaunch::default());
        if let Ok(list) = get_entries("instances") {
            self.instances = Some(list);
        } else {
            err!("Failed to reload instances list.");
        }
    }

    pub fn go_to_launch_screen_with_message(&mut self, message: String) {
        self.state = State::Launch(MenuLaunch::with_message(message));
        if let Ok(list) = get_entries("instances") {
            self.instances = Some(list);
        } else {
            err!("Failed to reload instances list.");
        }
    }

    pub fn edit_instance_wrapped(&mut self, selected_server: Option<String>) {
        match self.edit_instance(self.selected_instance.clone(), selected_server) {
            Ok(()) => {}
            Err(err) => self.set_error(err.to_string()),
        }
    }
}

fn load_config_and_theme(
) -> Result<(Option<LauncherConfig>, LauncherTheme, LauncherStyle), JsonFileError> {
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

pub fn get_entries(path: &str) -> Result<Vec<String>, IoError> {
    let dir_path = file_utils::get_launcher_dir()?;
    std::fs::create_dir_all(&dir_path).map_err(io_err!(dir_path))?;

    let dir_path = dir_path.join(path);
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
