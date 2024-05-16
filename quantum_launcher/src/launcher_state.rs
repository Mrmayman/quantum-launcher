use std::{
    path::PathBuf,
    process::Child,
    sync::{mpsc::Receiver, Arc},
};

use quantum_launcher_backend::{
    download::progress::DownloadProgress,
    error::LauncherResult,
    instance::{instance_launch::GameLaunchResult, instance_mod_installer::fabric::FabricVersion},
    io_err,
    json_structs::json_instance_config::InstanceConfigJson,
};

use crate::config::LauncherConfig;

#[derive(Debug, Clone)]
pub enum Message {
    OpenDir(PathBuf),
    InstallFabricEnd(Result<(), String>),
    InstallFabricVersionSelected(String),
    InstallFabricVersionsLoaded(Result<Vec<FabricVersion>, String>),
    LaunchInstanceSelected(String),
    LaunchUsernameSet(String),
    Launch,
    DeleteInstanceMenu,
    DeleteInstance,
    GoToLaunchScreen,
    LaunchEnd(GameLaunchResult),
    CreateInstanceScreen,
    CreateInstanceVersionsLoaded(Result<Arc<Vec<String>>, String>),
    CreateInstanceVersionSelected(String),
    CreateInstanceNameInput(String),
    CreateInstance,
    CreateInstanceEnd(Result<(), String>),
    CreateInstanceProgressUpdate,
    LocateJavaStart,
    LocateJavaEnd(Option<PathBuf>),
    EditInstance,
    EditInstanceJavaOverride(String),
    EditInstanceMemoryChanged(f32),
    EditInstanceSave,
    ManageMods,
    InstallFabricClicked,
    InstallFabric,
}

#[derive(Default)]
pub struct MenuLaunch {
    pub selected_instance: Option<String>,
}

pub struct MenuEditInstance {
    pub selected_instance: String,
    pub config: InstanceConfigJson,
    pub slider_value: f32,
    pub slider_text: String,
}

pub enum State {
    Launch(MenuLaunch),
    EditInstance(MenuEditInstance),
    EditMods {
        selected_instance: String,
        config: InstanceConfigJson,
    },
    Create {
        instance_name: String,
        selected_version: Option<String>,
        versions: Vec<String>,
        progress_reciever: Option<Receiver<DownloadProgress>>,
        progress_number: Option<f32>,
        progress_text: Option<String>,
    },
    FindJavaVersion {
        version: Option<PathBuf>,
        required_version: Option<usize>,
    },
    Error {
        error: String,
    },
    DeleteInstance {
        selected_instance: String,
    },
    InstallFabric {
        selected_instance: String,
        fabric_version: Option<String>,
        fabric_versions: Vec<String>,
    },
}

pub struct Launcher {
    pub state: State,
    pub instances: Option<Vec<String>>,
    pub config: Option<LauncherConfig>,
    pub spawned_process: Option<Arc<std::sync::Mutex<Child>>>,
}

impl Launcher {
    pub fn new() -> LauncherResult<Self> {
        // .config/QuantumLauncher/ OR AppData/Roaming/QuantumLauncher/
        let dir_path = quantum_launcher_backend::file_utils::get_launcher_dir()?;
        std::fs::create_dir_all(&dir_path).map_err(io_err!(dir_path))?;

        // QuantumLauncher/instances/
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

        Ok(Self {
            instances: Some(subdirectories),
            state: State::Launch(MenuLaunch::default()),
            spawned_process: None,
            config: Some(LauncherConfig::load()?),
        })
    }

    pub fn with_error(error: String) -> Self {
        Self {
            state: State::Error {
                error: format!("Error: {error}"),
            },
            instances: None,
            config: LauncherConfig::load().ok(),
            spawned_process: None,
        }
    }

    pub fn set_error(&mut self, error: String) {
        self.state = State::Error { error }
    }

    pub fn go_to_launch_screen(&mut self) {
        self.state = State::Launch(MenuLaunch::default())
    }
}
