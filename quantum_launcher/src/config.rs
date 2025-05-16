use ql_core::{
    err, IntoIoError, IntoJsonError, JsonFileError, LAUNCHER_DIR, LAUNCHER_VERSION_NAME,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path};

pub const SIDEBAR_WIDTH_DEFAULT: u32 = 190;

/// The global launcher configuration.
///
/// This is stored in the launcher directory
/// (`QuantumLauncher/`) as `config.json`.
///
/// For more info on the launcher directory see
/// <https://mrmayman.github.io/quantumlauncher#files-location>
///
/// # Why `Option`?
///
/// Note: many fields here are `Option`s. This is for
/// backwards-compatibility, as if you upgrade from an older
/// version without these fields, `serde` will safely serialize
/// them as `None`.
///
/// So generally `None` is interpreted as a default value
/// put there when migrating from a version without the feature.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LauncherConfig {
    /// The offline username set by the player when playing Minecraft.
    pub username: String,

    #[deprecated(
        since = "0.2.0",
        note = "removed feature, field left here for backwards compatibility"
    )]
    pub java_installs: Option<Vec<String>>,

    /// The theme (Light/Dark) set by the user.
    ///
    /// Implemented in v0.3
    pub theme: Option<String>,
    /// The color scheme set by the user.
    ///
    /// Implemented in v0.3
    ///
    /// Valid options are:
    /// - Purple
    /// - Brown
    /// - Sky Blue
    /// - Catppuccin
    pub style: Option<String>,

    /// The version that the launcher was last time
    /// you opened it.
    ///
    /// Implemented in v0.3, so if it's missing then
    /// it was last opened in v0.1 or v0.2
    pub version: Option<String>,

    /// The width of the sidebar in the main menu
    /// (which shows the list of instances). You can
    /// drag it around to resize it.
    ///
    /// Implemented in v0.4
    pub sidebar_width: Option<u32>,
    /// A list of Minecraft accounts logged into the launcher.
    ///
    /// Implemented in v0.4
    ///
    /// `String (username) : ConfigAccount { uuid: String, skin: None (unimplemented) }`
    ///
    /// Upon opening the launcher,
    /// [`ql_instances::read_refresh_token`]`(username)`
    /// is called on each account's key value (username)
    /// to get the refresh token (stored securely on disk).
    pub accounts: Option<HashMap<String, ConfigAccount>>,
    /// The scale of the UI, ie. how big everything is.
    ///
    /// Implemented in v0.4
    ///
    /// - `(1.0-*)` A higher number means more zoomed in buttons, text
    ///   and everything else (useful if you are on a high DPI display
    ///   or have bad eyesight),
    /// - `1.0` is the default value.
    /// - `(0.x-1.0)` A lower number means zoomed out UI elements.
    pub ui_scale: Option<f64>,
}

impl Default for LauncherConfig {
    fn default() -> Self {
        #[allow(deprecated)]
        Self {
            username: String::new(),
            theme: None,
            style: None,
            version: Some(LAUNCHER_VERSION_NAME.to_owned()),
            sidebar_width: Some(SIDEBAR_WIDTH_DEFAULT),
            accounts: None,
            ui_scale: None,
            java_installs: Some(Vec::new()),
        }
    }
}

impl LauncherConfig {
    /// Load the launcher configuration. You must supply the launcher
    /// directory in the `launcher_dir` argument. It can be obtained from
    /// [`file_utils::get_launcher_dir`].
    ///
    /// # Errors
    /// - if the user doesn't have permission to access launcher directory
    ///
    /// This function is designed to *not* fail fast,
    /// resetting the config if it's nonexistent or corrupted
    /// (with an error log message).
    pub fn load(launcher_dir: &Path) -> Result<Self, JsonFileError> {
        let config_path = launcher_dir.join("config.json");
        if !config_path.exists() {
            return LauncherConfig::create(&config_path);
        }

        let config = std::fs::read_to_string(&config_path).path(&config_path)?;
        let mut config: Self = match serde_json::from_str(&config) {
            Ok(config) => config,
            Err(err) => {
                err!("Invalid launcher config! This may be a sign of corruption! Please report if this happens to you.\nError: {err}");
                return LauncherConfig::create(&config_path);
            }
        };

        #[allow(deprecated)]
        {
            if config.java_installs.is_none() {
                config.java_installs = Some(Vec::new());
            }
        }

        Ok(config)
    }

    pub async fn save(&self) -> Result<(), JsonFileError> {
        let config_path = LAUNCHER_DIR.join("config.json");
        let config = serde_json::to_string(&self).json_to()?;

        tokio::fs::write(&config_path, config.as_bytes())
            .await
            .path(config_path)?;
        Ok(())
    }

    fn create(path: &Path) -> Result<Self, JsonFileError> {
        let config = LauncherConfig::default();
        std::fs::write(path, serde_json::to_string(&config).json_to()?.as_bytes()).path(path)?;
        Ok(config)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ConfigAccount {
    /// UUID of the Minecraft account. Stored as a string without dashes.
    ///
    /// Example: `2553495fc9094d40a82646cfc92cd7a5`
    ///
    /// A UUID is like an alternate username that can be used to identify
    /// an account. Unlike a username it can't be changed, so it's useful for
    /// dealing with account data in a stable manner.
    ///
    /// You can find someone's UUID through many online services where you
    /// input their username.
    pub uuid: String,
    /// Currently unimplemented, does nothing.
    pub skin: Option<String>, // TODO: Add skin visualization?
}
