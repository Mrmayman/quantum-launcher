use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{InstanceSelection, IntoIoError, IntoJsonError, JsonFileError};

/// Configuration for a specific instance.
/// Not to be confused with [`crate::json::VersionDetails`]. That one
/// is launcher agnostic data provided from mojang, this one is
/// Quantumlauncher-specific information.
///
/// Stored in:
/// - Client: `QuantumLauncher/instances/<instance_name>/config.json`
/// - Server: `QuantumLauncher/servers/<instance_name>/config.json`
///
/// See the documentation of each field for more information.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InstanceConfigJson {
    /// **Default: `"Vanilla"`**
    ///
    /// Can be one of:
    /// - `"Vanilla"` (unmodded)
    /// - `"Fabric"`
    /// - `"Forge"`
    /// - `"OptiFine"`
    /// - `"Quilt"`
    /// - `"NeoForge"`
    pub mod_type: String,
    /// If you want to use your own Java installation
    /// instead of the auto-installed one, specify
    /// the path to the `java` executable here.
    pub java_override: Option<String>,
    /// The amount of RAM in megabytes the instance should have.
    pub ram_in_mb: usize,
    /// **Default: `true`**
    ///
    /// - `true` (default): Show log output in launcher.
    ///   May not show all log output, especially during a crash.
    /// - `false`: Print raw, unformatted log output to the console (stdout).
    ///   This is useful for debugging, but may be hard to read.
    pub enable_logger: Option<bool>,
    /// This is an optional list of additional
    /// arguments to pass to Java.
    pub java_args: Option<Vec<String>>,
    /// This is an optional list of additional
    /// arguments to pass to the game.
    pub game_args: Option<Vec<String>>,
    /// DEPRECATED in v0.4.2
    ///
    /// This used to indicate whether a version
    /// was downloaded from Omniarchive instead
    /// of Mojang, in Quantum Launcher
    /// v0.3.1 - v0.4.1
    #[deprecated(since = "0.4.2", note = "migrated to BetterJSONs, so no longer needed")]
    pub omniarchive: Option<serde_json::Value>,
    /// **Default: `false`**
    ///
    /// - `true`: the instance is a classic server.
    /// - `false` (default): the instance is a client
    ///   or a non-classic server (alpha, beta, release).
    ///
    /// This is stored because classic servers:
    /// - Are downloaded differently (zip file to extract)
    /// - Cannot be stopped by sending a `stop` command.
    ///   (need to kill the process)
    pub is_classic_server: Option<bool>,
    /// **Client Only**
    ///
    /// If true, then the Java Garbage Collector
    /// will be modified through launch arguments,
    /// for *different* performance.
    ///
    /// **Default: `false`**
    ///
    /// This doesn't specifically improve performance,
    /// in fact from my testing it worsens them?:
    ///
    /// - Without these args I got 110-115 FPS average on vanilla
    ///   Minecraft 1.20 in a new world.
    ///
    /// - With these args I got 105-110 FPS. So... yeah they aren't
    ///   doing the job for me.
    ///
    /// But in different workloads this might improve performance.
    ///
    /// # Arguments
    ///
    /// The G1 garbage collector will be used.
    /// Here are the specific arguments.
    ///
    /// - `-XX:+UnlockExperimentalVMOptions`
    /// - `-XX:+UseG1GC`
    /// - `-XX:G1NewSizePercent=20`
    /// - `-XX:G1ReservePercent=20`
    /// - `-XX:MaxGCPauseMillis=50`
    /// - `-XX:G1HeapRegionSize=32M`
    pub do_gc_tuning: Option<bool>,
    /// **Client Only**
    ///
    /// Whether to close the launcher upon
    /// starting the game.
    ///
    /// **Default: `false`**
    ///
    /// This keeps *just the game* running
    /// after you open it. However:
    /// - The impact of keeping the launcher open
    ///   is downright **negligible**. Quantum Launcher
    ///   is **very** lightweight. You won't feel any
    ///   difference even on slow computers
    /// - By doing this you lose access to easy log viewing
    ///   and the ability to easily kill the game process if stuck
    ///
    /// Ultimately if you want one less icon in your taskbar then go ahead.
    pub close_on_start: Option<bool>,
}

impl InstanceConfigJson {
    /// Returns a String containing the Java argument to
    /// allocate the configured amount of RAM.
    #[must_use]
    pub fn get_ram_argument(&self) -> String {
        format!("-Xmx{}M", self.ram_in_mb)
    }

    /// Loads the launcher-specific instance configuration from disk,
    /// based on a path to the root of the instance directory.
    ///
    /// # Errors
    /// - `dir`/`config.json` doesn't exist or isn't a file
    /// - `config.json` file couldn't be loaded
    /// - `config.json` couldn't be parsed into valid JSON
    pub async fn read_from_path(dir: &Path) -> Result<Self, JsonFileError> {
        let config_json_path = dir.join("config.json");
        let config_json = tokio::fs::read_to_string(&config_json_path)
            .await
            .path(config_json_path)?;
        Ok(serde_json::from_str(&config_json).json(config_json)?)
    }

    /// Loads the launcher-specific instance configuration from disk,
    /// based on a specific `InstanceSelection`
    ///
    /// # Errors
    /// - `config.json` file couldn't be loaded
    /// - `config.json` couldn't be parsed into valid JSON
    pub async fn read(instance: &InstanceSelection) -> Result<Self, JsonFileError> {
        Self::read_from_path(&instance.get_instance_path()).await
    }
}
