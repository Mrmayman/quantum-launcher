use quantum_launcher_backend::{error::LauncherError, file_utils, io_err};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
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

            std::fs::write(&config_path, serde_json::to_string(&config)?.as_bytes())
                .map_err(io_err!(config_path))?;

            return Ok(config);
        }

        let config = std::fs::read_to_string(&config_path).map_err(io_err!(config_path))?;
        Ok(serde_json::from_str(&config)?)
    }

    pub fn save(&self) -> Result<(), LauncherError> {
        let config_path = file_utils::get_launcher_dir()?.join("config.json");
        let config = serde_json::to_string(&self)?;

        std::fs::write(&config_path, config.as_bytes()).map_err(io_err!(config_path))?;
        Ok(())
    }

    pub async fn save_wrapped(self) -> Result<(), String> {
        self.save().map_err(|err| err.to_string())
    }
}
