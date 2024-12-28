use std::{
    collections::{HashMap, HashSet},
    process::ExitStatus,
    sync::{mpsc::Receiver, Arc, Mutex},
    time::Instant,
};

use iced::{widget::image::Handle, Command};
use ql_core::{
    err, file_utils, info,
    json::{instance_config::InstanceConfigJson, version::VersionDetails},
    DownloadProgress, InstanceSelection, IntoIoError, IoError, JavaInstallProgress, JsonFileError,
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
}

#[derive(Debug, Clone)]
pub enum EditInstanceMessage {
    MenuOpen,
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
pub enum ManageModsMessage {
    ScreenOpen,
    ToggleCheckbox((String, String), bool),
    ToggleCheckboxLocal(String, bool),
    DeleteSelected,
    DeleteFinished(Result<Vec<String>, String>),
    LocalDeleteFinished(Result<(), String>),
    LocalIndexLoaded(HashSet<String>),
    ToggleSelected,
    ToggleFinished(Result<(), String>),
    UpdateMods,
    UpdateModsFinished(Result<(), String>),
}

#[derive(Debug, Clone)]
pub enum Message {
    InstallFabric(InstallFabricMessage),
    CreateInstance(CreateInstanceMessage),
    EditInstance(EditInstanceMessage),
    ManageMods(ManageModsMessage),
    CoreOpenDir(String),
    LaunchInstanceSelected(String),
    LaunchUsernameSet(String),
    LaunchStart,
    LaunchScreenOpen {
        message: Option<String>,
        clear_selection: bool,
    },
    LaunchEnd(GameLaunchResult),
    LaunchKill,
    LaunchKillEnd(Result<(), String>),
    DeleteInstanceMenu,
    DeleteInstance,
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
    ServerManageOpen {
        selected_server: Option<String>,
        message: Option<String>,
    },
    ServerManageSelectedServer(String),
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
    ServerDeleteOpen,
    ServerDeleteConfirm,
    ServerEditModsOpen,
    InstallPaperStart,
    InstallPaperEnd(Result<(), String>),
    UninstallLoaderPaperStart,
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
        selected_instance: InstanceSelection,
    ) -> Command<Message> {
        let mut blacklist = Vec::new();
        for mod_info in idx.mods.values() {
            for file in &mod_info.files {
                blacklist.push(file.filename.clone());
            }
        }
        Command::perform(
            get_locally_installed_mods(selected_instance, blacklist),
            |n| Message::ManageMods(ManageModsMessage::LocalIndexLoaded(n)),
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
    Loading(bool),
    Loaded {
        is_quilt: bool,
        fabric_version: Option<String>,
        fabric_versions: Vec<String>,
        progress_receiver: Option<Receiver<FabricInstallProgress>>,
        progress_num: f32,
        progress_message: String,
    },
    Unsupported(bool),
}

impl MenuInstallFabric {
    pub fn is_quilt(&self) -> bool {
        match self {
            MenuInstallFabric::Loading(is_quilt) => *is_quilt,
            MenuInstallFabric::Loaded { is_quilt, .. } => *is_quilt,
            MenuInstallFabric::Unsupported(is_quilt) => *is_quilt,
        }
    }
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
    InstallPaper,
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
    ServerDelete,
}

pub struct MenuServerManage {
    pub server_list: Vec<String>,
    pub java_install_recv: Option<Receiver<JavaInstallProgress>>,
    pub message: Option<String>,
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
    pub selected_instance: Option<InstanceSelection>,
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
    pub name: String,
    pub has_issued_stop_command: bool,
}

impl Drop for ServerProcess {
    fn drop(&mut self) {
        if !self.has_issued_stop_command {
            info!("Force-Killing server {}\n       You should be a bit more careful before closing the launcher window", self.name);
            let mut lock = self.child.lock().unwrap();
            let _ = lock.start_kill();
        }
    }
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
            server_processes: HashMap::new(),
            server_logs: HashMap::new(),
            server_version_list_cache: None,
        }
    }

    pub fn set_error<T: ToString>(&mut self, error: T) {
        self.state = State::Error {
            error: error.to_string(),
        }
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

    pub fn edit_instance_w(&mut self) {
        let selected_instance = self.selected_instance.clone().unwrap();
        match self.edit_instance(&selected_instance) {
            Ok(()) => {}
            Err(err) => self.set_error(err),
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
    std::fs::create_dir_all(&dir_path).path(&dir_path)?;

    let dir_path = dir_path.join(path);
    std::fs::create_dir_all(&dir_path).path(&dir_path)?;

    let dir = std::fs::read_dir(&dir_path).path(dir_path)?;

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
