use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{IntoIoError, JsonFileError};

/// Configuration for a specific instance.
///
/// Stored in `QuantumLauncher/instances/<instance_name>/config.json`.
///
/// See the documentation of each field for more information.
#[derive(Serialize, Deserialize, Clone)]
pub struct InstanceConfigJson {
    /// Can be one of:
    /// - `"Vanilla"`
    /// - `"Fabric"`
    /// - `"Forge"`
    /// - `"OptiFine"`
    /// - `"Quilt"`
    /// - `"NeoForge"` (coming soon)
    pub mod_type: String,
    /// If you want to use your own Java installation
    /// instead of the auto-installed one, specify
    /// the path to the `java` executable here.
    pub java_override: Option<String>,
    /// The amount of RAM in megabytes the instance should have.
    pub ram_in_mb: usize,
    /// - `true` (default): Show log output in launcher.
    ///   May not show all log output, especially during a crash.
    /// - `false`: Print raw, unformatted log output to the console.
    ///   This is useful for debugging, but may be hard to read.
    pub enable_logger: Option<bool>,
    /// This is an optional list of additional
    /// arguments to pass to Java.
    pub java_args: Option<Vec<String>>,
    /// This is an optional list of additional
    /// arguments to pass to the game.
    pub game_args: Option<Vec<String>>,
    /// If the instance was downloaded from Omniarchive,
    /// this field contains information about the entry.
    pub omniarchive: Option<OmniarchiveEntry>,
    /// - `true`: the instance is a classic server.
    /// - `false` (default): the instance is a client
    ///   or a non-classic server (alpha, beta, release).
    ///
    /// This is stored because classic servers:
    /// - Are downloaded differently (zip file to extract)
    /// - Cannot be stopped by sending a `stop` command.
    ///   (need to kill the process)
    pub is_classic_server: Option<bool>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct OmniarchiveEntry {
    pub name: String,
    pub url: String,
    pub category: String,
}

impl InstanceConfigJson {
    /// Returns a String containing the Java argument to
    /// allocate the configured amount of RAM.
    #[must_use]
    pub fn get_ram_argument(&self) -> String {
        format!("-Xmx{}M", self.ram_in_mb)
    }

    pub async fn read(dir: &Path) -> Result<Self, JsonFileError> {
        let config_json_path = dir.join("config.json");
        let config_json = tokio::fs::read_to_string(&config_json_path)
            .await
            .path(config_json_path)?;
        let config_json: InstanceConfigJson = serde_json::from_str(&config_json)?;
        Ok(config_json)
    }
}
