use std::{
    collections::{HashMap, HashSet},
    process::ExitStatus,
    sync::{mpsc::Receiver, Arc, Mutex},
    time::Instant,
};

use iced::widget::image::Handle;
use ql_instances::{
    error::{LauncherError, LauncherResult},
    file_utils, io_err,
    json_structs::{json_instance_config::InstanceConfigJson, json_version::VersionDetails},
    DownloadProgress, GameLaunchResult, JavaInstallProgress, LogLine, UpdateCheckInfo,
    UpdateProgress,
};
use ql_mod_manager::{
    instance_mod_installer::{
        fabric::{FabricInstallProgress, FabricVersion},
        forge::ForgeInstallProgress,
    },
    modrinth::{ProjectInfo, Search},
};
use tokio::process::Child;

use crate::config::LauncherConfig;

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
    ManageModsScreenOpen,
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
}

#[derive(Default)]
pub struct MenuLaunch {
    pub message: String,
    pub recv: Option<Receiver<JavaInstallProgress>>,
}

impl MenuLaunch {
    pub fn with_message(message: String) -> Self {
        Self {
            message,
            recv: None,
        }
    }
}

pub struct MenuEditInstance {
    pub config: InstanceConfigJson,
    pub slider_value: f32,
    pub slider_text: String,
}

pub struct MenuEditMods {
    pub config: InstanceConfigJson,
}

pub struct MenuCreateInstance {
    pub instance_name: String,
    pub selected_version: Option<String>,
    pub versions: Vec<String>,
    pub progress_receiver: Option<Receiver<DownloadProgress>>,
    pub progress_number: Option<f32>,
    pub progress_text: Option<String>,
    pub download_assets: bool,
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

pub struct MenuModsDownload {
    pub query: String,
    pub results: Option<Search>,
    pub result_data: HashMap<String, ProjectInfo>,
    pub config: InstanceConfigJson,
    pub json: VersionDetails,
    pub opened_mod: Option<usize>,
    pub latest_load: Instant,
    pub is_loading_search: bool,
}

pub enum State {
    Launch(MenuLaunch),
    EditInstance(MenuEditInstance),
    EditMods(MenuEditMods),
    Create(MenuCreateInstance),
    Error { error: String },
    DeleteInstance(MenuDeleteInstance),
    InstallFabric(MenuInstallFabric),
    InstallForge(MenuInstallForge),
    InstallJava(MenuInstallJava),
    UpdateFound(MenuLauncherUpdate),
    ModsDownload(MenuModsDownload),
}

pub struct Launcher {
    pub state: State,
    pub selected_instance: Option<String>,
    pub instances: Option<Vec<String>>,
    pub config: Option<LauncherConfig>,
    pub processes: HashMap<String, GameProcess>,
    pub logs: HashMap<String, String>,
    pub images: HashMap<String, Handle>,
    pub images_downloads_in_progress: HashSet<String>,
    pub images_to_load: Mutex<HashSet<String>>,
}

pub struct GameProcess {
    pub child: Arc<Mutex<Child>>,
    pub receiver: Receiver<LogLine>,
}

impl Launcher {
    pub fn new(message: Option<String>) -> LauncherResult<Self> {
        let subdirectories = reload_instances()?;

        Ok(Self {
            instances: Some(subdirectories),
            state: State::Launch(if let Some(message) = message {
                MenuLaunch::with_message(message)
            } else {
                MenuLaunch::default()
            }),
            processes: HashMap::new(),
            config: Some(LauncherConfig::load()?),
            logs: HashMap::new(),
            selected_instance: None,
            images: Default::default(),
            images_downloads_in_progress: Default::default(),
            images_to_load: Default::default(),
        })
    }

    pub fn with_error(error: String) -> Self {
        Self {
            state: State::Error {
                error: format!("Error: {error}"),
            },
            instances: None,
            config: LauncherConfig::load().ok(),
            processes: HashMap::new(),
            logs: HashMap::new(),
            selected_instance: None,
            images: Default::default(),
            images_downloads_in_progress: Default::default(),
            images_to_load: Default::default(),
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
            eprintln!("[error] Failed to reload instances list.")
        }
    }

    pub fn go_to_launch_screen_with_message(&mut self, message: String) {
        self.state = State::Launch(MenuLaunch::with_message(message));
        if let Ok(list) = reload_instances() {
            self.instances = Some(list);
        } else {
            eprintln!("[error] Failed to reload instances list.")
        }
    }

    pub fn edit_instance_wrapped(&mut self) {
        match self.edit_instance(self.selected_instance.clone().unwrap()) {
            Ok(_) => {}
            Err(err) => self.set_error(err.to_string()),
        }
    }
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
