use std::{
    path::PathBuf,
    process::Child,
    sync::{mpsc::Receiver, Arc},
};

use quantum_launcher_backend::{
    download::DownloadProgress,
    error::{LauncherError, LauncherResult},
    file_utils::create_dir_if_not_exists,
    instance::instance_launch::GameLaunchResult,
    json_structs::json_instance_config::InstanceConfigJson,
};

use crate::config::LauncherConfig;

#[derive(Debug, Clone)]
pub enum Message {
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
}

pub enum State {
    Launch {
        selected_instance: String,
    },
    EditInstance {
        selected_instance: String,
        config: InstanceConfigJson,
        slider_value: f32,
        slider_text: String,
    },
    Create {
        instance_name: String,
        version: String,
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
}

pub struct Launcher {
    pub state: State,
    pub instances: Option<Vec<String>>,
    pub config: Option<LauncherConfig>,
    pub spawned_process: Option<Arc<std::sync::Mutex<Child>>>,
}

impl Launcher {
    pub fn load() -> LauncherResult<Self> {
        let dir_path = quantum_launcher_backend::file_utils::get_launcher_dir()?;
        create_dir_if_not_exists(&dir_path)
            .map_err(|err| LauncherError::IoError(err, dir_path.clone()))?;
        let dir_path = dir_path.join("instances");
        create_dir_if_not_exists(&dir_path)
            .map_err(|err| LauncherError::IoError(err, dir_path.clone()))?;
        let dir =
            std::fs::read_dir(&dir_path).map_err(|err| LauncherError::IoError(err, dir_path))?;

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
            state: State::Launch {
                selected_instance: Default::default(),
            },
            spawned_process: None,
            config: Some(LauncherConfig::load()?),
        })
    }

    pub fn set_error(&mut self, error: String) {
        self.state = State::Error { error }
    }

    pub fn go_to_launch_screen(&mut self) {
        self.state = State::Launch {
            selected_instance: "".to_owned(),
        }
    }
}
