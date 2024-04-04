use std::{
    path::PathBuf,
    process::Child,
    sync::{mpsc::Receiver, Arc},
};

use quantum_launcher_backend::{
    download::DownloadProgress, error::LauncherResult, file_utils::create_dir_if_not_exists,
    instance::instance_launch::GameLaunchResult,
};

use crate::config::LauncherConfig;

#[derive(Debug, Clone)]
pub enum Message {
    LaunchInstanceSelected(String),
    LaunchUsernameSet(String),
    Launch,
    DeleteInstanceMenu,
    DeleteInstance,
    DeleteInstanceCancel,
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
}

pub enum State {
    Launch {
        selected_instance: String,
        spawned_process: Option<Arc<std::sync::Mutex<Child>>>,
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
}

impl Launcher {
    pub fn load() -> LauncherResult<Self> {
        let dir_path = quantum_launcher_backend::file_utils::get_launcher_dir()?;
        create_dir_if_not_exists(&dir_path)?;
        let dir_path = dir_path.join("instances");
        create_dir_if_not_exists(&dir_path)?;
        let dir = std::fs::read_dir(&dir_path)?;

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
                spawned_process: None,
            },
            config: Some(LauncherConfig::load()?),
        })
    }

    pub fn set_error(&mut self, error: String) {
        self.state = State::Error { error }
    }

    pub fn go_to_launch_screen(&mut self) {
        self.state = State::Launch {
            selected_instance: "".to_owned(),
            spawned_process: None,
        }
    }
}
