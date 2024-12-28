use std::collections::HashMap;

use ql_core::{file_utils, io_err, IoError};

pub struct ServerProperties {
    pub entries: HashMap<String, String>,
}

impl ServerProperties {
    pub fn load(server_name: &str) -> Option<Self> {
        let server_dir = file_utils::get_launcher_dir()
            .ok()?
            .join("servers")
            .join(server_name);
        let properties_file = server_dir.join("server.properties");
        let entries = std::fs::read_to_string(&properties_file).ok()?;

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

    pub fn save(&self, server_name: &str) -> Result<(), IoError> {
        let server_dir = file_utils::get_launcher_dir()?
            .join("servers")
            .join(server_name);
        let properties_file = server_dir.join("server.properties");
        let mut properties_content = String::new();
        for (key, value) in &self.entries {
            properties_content.push_str(&format!("{}={}\n", key, value));
        }
        std::fs::write(&properties_file, properties_content).map_err(io_err!(properties_file))?;
        Ok(())
    }
}
