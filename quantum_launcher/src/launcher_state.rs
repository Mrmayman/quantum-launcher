use std::{
    process::Child,
    sync::{Arc, Mutex},
};

use quantum_launcher_backend::error::LauncherResult;

#[derive(Debug, Clone)]
pub enum Message {
    InstanceSelected(String),
    UsernameSet(String),
    LaunchGame,
    GameOpened(Result<Arc<Mutex<Child>>, String>),
}

pub enum State {
    Launch {
        instances: Vec<String>,
        selected_instance: String,
        username: String,
    },
    Create {
        instance_name: String,
        version: String,
    },
    Error {
        error: String,
    },
}

pub struct Launcher {
    pub state: State,
}

impl Launcher {
    pub fn load() -> LauncherResult<Self> {
        let dir = quantum_launcher_backend::file_utils::get_launcher_dir()?;
        let dir = std::fs::read_dir(dir.join("instances"))?;

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
            state: State::Launch {
                instances: subdirectories,
                selected_instance: Default::default(),
                username: Default::default(),
            },
        })
    }
}
