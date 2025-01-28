use std::collections::HashMap;

use ql_core::{file_utils, IntoIoError, IoError};

pub struct ServerProperties {
    pub entries: HashMap<String, String>,
}

impl ServerProperties {
    pub async fn load(server_name: &str) -> Option<Self> {
        let server_dir = file_utils::get_launcher_dir()
            .await
            .ok()?
            .join("servers")
            .join(server_name);
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

    pub async fn save(&self, server_name: &str) -> Result<(), IoError> {
        let server_dir = file_utils::get_launcher_dir()
            .await?
            .join("servers")
            .join(server_name);
        let properties_file = server_dir.join("server.properties");
        let mut properties_content = String::new();
        for (key, value) in &self.entries {
            properties_content.push_str(&format!("{key}={value}\n"));
        }
        tokio::fs::write(&properties_file, properties_content)
            .await
            .path(properties_file)?;
        Ok(())
    }
}
