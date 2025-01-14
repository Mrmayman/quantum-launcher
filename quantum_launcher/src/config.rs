use ql_core::{err, file_utils, IntoIoError, JsonFileError, LAUNCHER_VERSION_NAME};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// The launcher configuration.
///
/// This is stored in the launcher directory as `config.json`.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LauncherConfig {
    pub username: String,
    pub theme: Option<String>,
    pub style: Option<String>,
    pub version: Option<String>,
}

impl Default for LauncherConfig {
    fn default() -> Self {
        Self {
            username: String::new(),
            theme: None,
            style: None,
            version: Some(LAUNCHER_VERSION_NAME.to_owned()),
        }
    }
}

impl LauncherConfig {
    /// Load the launcher configuration.
    pub fn load() -> Result<Self, JsonFileError> {
        let config_path = file_utils::get_launcher_dir()?.join("config.json");
        if !config_path.exists() {
            return LauncherConfig::create(&config_path);
        }

        let config = std::fs::read_to_string(&config_path).path(&config_path)?;
        let config = match serde_json::from_str(&config) {
            Ok(config) => config,
            Err(err) => {
                err!("Invalid launcher config! This may be a sign of corruption! Please report if this happens to you.\nError: {err}");
                return LauncherConfig::create(&config_path);
            }
        };
        Ok(config)
    }

    /// Saves the launcher configuration.
    pub async fn save(&self) -> Result<(), JsonFileError> {
        let config_path = file_utils::get_launcher_dir()?.join("config.json");
        let config = serde_json::to_string(&self)?;

        tokio::fs::write(&config_path, config.as_bytes())
            .await
            .path(config_path)?;
        Ok(())
    }

    /// [`save`] `_w` function
    pub async fn save_w(self) -> Result<(), String> {
        self.save().await.map_err(|err| err.to_string())
    }

    fn create(path: &Path) -> Result<Self, JsonFileError> {
        let config = LauncherConfig::default();

        std::fs::write(path, serde_json::to_string(&config)?.as_bytes()).path(path)?;

        Ok(config)
    }
}
