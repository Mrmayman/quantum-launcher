use std::collections::{HashMap, HashSet};

use ql_instances::{file_utils, io_err};
use serde::{Deserialize, Serialize};

use super::{ModFile, ModrinthError};

#[derive(Serialize, Deserialize, Clone)]
pub struct ModConfig {
    pub name: String,
    pub manually_installed: bool,
    pub installed_version: String,
    pub enabled: bool,
    pub description: String,
    pub icon_url: Option<String>,
    pub project_id: String,
    pub files: Vec<ModFile>,
    pub supported_versions: Vec<String>,
    pub dependencies: HashSet<String>,
    pub dependents: HashSet<String>,
}

#[derive(Serialize, Deserialize)]
pub struct ModIndex {
    pub mods: HashMap<String, ModConfig>,
    pub instance_name: String,
}

impl ModIndex {
    pub fn get(instance_name: &str) -> Result<Self, ModrinthError> {
        let launcher_dir = file_utils::get_launcher_dir()?;
        let mods_dir = launcher_dir
            .join("instances")
            .join(instance_name)
            .join(".minecraft/mods");

        if !mods_dir.exists() {
            std::fs::create_dir(&mods_dir).map_err(io_err!(mods_dir))?;
        }

        let mod_index_path = mods_dir.join("index.json");

        if mod_index_path.exists() {
            let index =
                std::fs::read_to_string(&mod_index_path).map_err(io_err!(mod_index_path))?;
            Ok(serde_json::from_str(&index)?)
        } else {
            let index = ModIndex::with_name(instance_name);
            let index_str = serde_json::to_string(&index)?;
            std::fs::write(&mod_index_path, &index_str).map_err(io_err!(mod_index_path))?;
            Ok(index)
        }
    }

    pub fn save(&self) -> Result<(), ModrinthError> {
        let launcher_dir = file_utils::get_launcher_dir()?;
        let mods_dir = launcher_dir
            .join("instances")
            .join(&self.instance_name)
            .join(".minecraft/mods");

        if !mods_dir.exists() {
            std::fs::create_dir(&mods_dir).map_err(io_err!(mods_dir))?;
        }

        let index_dir = mods_dir.join("index.json");

        let index_str = serde_json::to_string(&self)?;
        std::fs::write(&index_dir, &index_str).map_err(io_err!(index_dir))?;
        Ok(())
    }

    fn with_name(instance_name: &str) -> Self {
        Self {
            mods: HashMap::new(),
            instance_name: instance_name.to_owned(),
        }
    }
}
