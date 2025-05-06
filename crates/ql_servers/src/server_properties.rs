use std::{collections::HashMap, fmt::Write};

use ql_core::{IntoIoError, IoError, LAUNCHER_DIR};

pub struct ServerProperties {
    pub entries: HashMap<String, String>,
}

impl ServerProperties {
    #[must_use]
    pub async fn load(server_name: &str) -> Option<Self> {
        let server_dir = LAUNCHER_DIR.join("servers").join(server_name);
        let properties_file = server_dir.join("server.properties");
        let entries = tokio::fs::read_to_string(&properties_file).await.ok()?;

        let entries_map: HashMap<String, String> = entries
            .lines()
            .filter(|n| !n.starts_with('#'))
            .filter_map(|n| n.split_once('='))
            .map(|(a, b)| (a.to_owned(), b.to_owned()))
            .collect();

        Some(Self {
            entries: entries_map,
        })
    }

    /// Saves the configuration to a server with name `server_name`,
    /// as a `server.properties` file.
    ///
    /// # Errors
    /// - if config dir (~/.config on linux or AppData/Roaming on windows) is not found
    /// - if you’re on an unsupported platform (other than Windows, Linux, macOS, Redox, any linux-like unix)
    /// - if the directory could not be accessed (permissions)
    pub async fn save(&self, server_name: &str) -> Result<(), IoError> {
        let server_dir = LAUNCHER_DIR.join("servers").join(server_name);
        let properties_file = server_dir.join("server.properties");
        let mut properties_content = String::new();
        for (key, value) in &self.entries {
            _ = writeln!(properties_content, "{key}={value}");
        }
        tokio::fs::write(&properties_file, properties_content)
            .await
            .path(properties_file)?;
        Ok(())
    }
}
