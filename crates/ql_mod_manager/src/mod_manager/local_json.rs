use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use ql_core::{file_utils, io_err, InstanceSelection};
use serde::{Deserialize, Serialize};

use super::{ModError, ModFile};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ModConfig {
    pub name: String,
    pub manually_installed: bool,
    pub installed_version: String,
    pub version_release_time: String,
    pub enabled: bool,
    pub description: String,
    pub icon_url: Option<String>,
    /// Source platform where the mod was downloaded from.
    /// Eg: "modrinth", "curseforge" (coming soon).
    pub project_source: String,
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
    pub is_server: Option<bool>,
}

impl ModIndex {
    pub fn get(selected_instance: &InstanceSelection) -> Result<Self, ModError> {
        let mods_dir = file_utils::get_dot_minecraft_dir(selected_instance)?.join("mods");

        get(mods_dir, selected_instance)
    }

    pub fn save(&self) -> Result<(), ModError> {
        let mods_dir = file_utils::get_dot_minecraft_dir(&InstanceSelection::new(
            &self.instance_name,
            self.is_server.unwrap_or(false),
        ))?
        .join("mods");

        if !mods_dir.exists() {
            std::fs::create_dir(&mods_dir).map_err(io_err!(mods_dir))?;
        }

        let index_dir = mods_dir.join("index.json");

        let index_str = serde_json::to_string(&self)?;
        std::fs::write(&index_dir, &index_str).map_err(io_err!(index_dir))?;
        Ok(())
    }

    fn with_name(instance_name: &InstanceSelection) -> Self {
        Self {
            mods: HashMap::new(),
            instance_name: instance_name.get_name().to_owned(),
            is_server: Some(instance_name.is_server()),
        }
    }
}

fn get(mods_dir: PathBuf, instance_name: &InstanceSelection) -> Result<ModIndex, ModError> {
    if !mods_dir.exists() {
        std::fs::create_dir(&mods_dir).map_err(io_err!(mods_dir))?;
    }

    let mod_index_path = mods_dir.join("index.json");

    if mod_index_path.exists() {
        let index = std::fs::read_to_string(&mod_index_path).map_err(io_err!(mod_index_path))?;
        Ok(serde_json::from_str(&index)?)
    } else {
        let index = ModIndex::with_name(instance_name);
        let index_str = serde_json::to_string(&index)?;
        std::fs::write(&mod_index_path, &index_str).map_err(io_err!(mod_index_path))?;
        Ok(index)
    }
}
