use std::collections::{HashMap, HashSet};

use ql_instances::{file_utils, io_err};
use serde::{Deserialize, Serialize};

use super::{ModDownloadError, ModFile};

#[derive(Serialize, Deserialize)]
pub struct ModConfig {
    pub name: String,
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
    pub fn get(instance_name: &str) -> Result<Self, ModDownloadError> {
        let launcher_dir = file_utils::get_launcher_dir()?;
        let mods_dir = launcher_dir
            .join("instances")
            .join(instance_name)
            .join("mods");

        if !mods_dir.exists() {
            std::fs::create_dir(&mods_dir).map_err(io_err!(mods_dir))?;
        }

        let index_dir = mods_dir.join("index.json");

        if index_dir.exists() {
            let index = std::fs::read_to_string(&index_dir).map_err(io_err!(index_dir))?;
            Ok(serde_json::from_str(&index)?)
        } else {
            let index = ModIndex::with_name(instance_name);
            let index_str = serde_json::to_string(&index)?;
            std::fs::write(&index_dir, &index_str).map_err(io_err!(index_dir))?;
            Ok(index)
        }
    }

    pub fn save(&self) -> Result<(), ModDownloadError> {
        let launcher_dir = file_utils::get_launcher_dir()?;
        let mods_dir = launcher_dir
            .join("instances")
            .join(&self.instance_name)
            .join("mods");

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
            mods: Default::default(),
            instance_name: instance_name.to_owned(),
        }
    }
}
