use std::path::Path;

use ql_instances::{error::LauncherError, file_utils, io_err};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct LauncherConfig {
    pub username: String,
    pub theme: Option<String>,
    pub style: Option<String>,
    /// Filler implementation, to not break older versions of the launcher.
    #[deprecated]
    pub java_installs: Vec<String>,
}

impl LauncherConfig {
    pub fn load() -> Result<Self, LauncherError> {
        let config_path = file_utils::get_launcher_dir()?.join("config.json");
        if !config_path.exists() {
            return LauncherConfig::create(&config_path);
        }

        let config = std::fs::read_to_string(&config_path).map_err(io_err!(config_path))?;
        let config = match serde_json::from_str(&config) {
            Ok(config) => config,
            Err(err) => {
                eprintln!("[error] Invalid launcher config! This may be a sign of corruption! Please report if this happens to you. Error: {err}");
                return LauncherConfig::create(&config_path);
            }
        };
        Ok(config)
    }

    pub async fn save(&self) -> Result<(), LauncherError> {
        let config_path = file_utils::get_launcher_dir()?.join("config.json");
        let config = serde_json::to_string(&self)?;

        tokio::fs::write(&config_path, config.as_bytes())
            .await
            .map_err(io_err!(config_path))?;
        Ok(())
    }

    pub async fn save_wrapped(self) -> Result<(), String> {
        self.save().await.map_err(|err| err.to_string())
    }

    fn create(path: &Path) -> Result<Self, LauncherError> {
        let config = LauncherConfig::default();

        std::fs::write(path, serde_json::to_string(&config)?.as_bytes()).map_err(io_err!(path))?;

        Ok(config)
    }
}
