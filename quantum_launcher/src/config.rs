use std::{fs::File, io::Write};

use quantum_launcher_backend::{error::LauncherError, file_utils};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct LauncherConfig {
    pub java_installs: Vec<String>,
    pub username: String,
}

impl LauncherConfig {
    pub fn load() -> Result<Self, LauncherError> {
        let config_path = file_utils::get_launcher_dir()?.join("config.json");
        if !config_path.exists() {
            let config = LauncherConfig {
                java_installs: Default::default(),
                username: Default::default(),
            };

            let mut file = File::create(config_path)?;
            file.write_all(serde_json::to_string(&config)?.as_bytes())?;

            return Ok(config);
        }

        let config = std::fs::read_to_string(&config_path)?;
        Ok(serde_json::from_str(&config)?)
    }

    pub fn save(&self) -> Result<(), LauncherError> {
        let config_path = file_utils::get_launcher_dir()?.join("config.json");
        let mut file = File::create(config_path)?;

        let config = serde_json::to_string(&self)?;
        file.write_all(config.as_bytes())?;
        Ok(())
    }
}
