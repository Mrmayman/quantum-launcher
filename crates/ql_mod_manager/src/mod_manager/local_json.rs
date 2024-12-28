use std::collections::{HashMap, HashSet};

use ql_core::{file_utils, InstanceSelection, IntoIoError};
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
        let dot_mc_dir = file_utils::get_dot_minecraft_dir(selected_instance)?;

        let mods_dir = dot_mc_dir.join("mods");
        if !mods_dir.exists() {
            std::fs::create_dir(&mods_dir).path(&mods_dir)?;
        }

        let index_path = dot_mc_dir.join("mod_index.json");
        let old_index_path = mods_dir.join("index.json");

        if index_path.exists() {
            let index = std::fs::read_to_string(&index_path).path(index_path)?;
            Ok(serde_json::from_str(&index)?)
        } else if old_index_path.exists() {
            // Migrate old index to new location
            let index = std::fs::read_to_string(&old_index_path).path(&old_index_path)?;
            let mod_index = serde_json::from_str(&index)?;

            std::fs::remove_file(&old_index_path).path(old_index_path)?;
            std::fs::write(&index_path, &index).path(index_path)?;

            Ok(mod_index)
        } else {
            let index = ModIndex::with_name(selected_instance);
            let index_str = serde_json::to_string(&index)?;
            std::fs::write(&index_path, &index_str).path(index_path)?;
            Ok(index)
        }
    }

    pub fn save(&self) -> Result<(), ModError> {
        let dot_mc_dir = file_utils::get_dot_minecraft_dir(&InstanceSelection::new(
            &self.instance_name,
            self.is_server.unwrap_or(false),
        ))?;

        let index_dir = dot_mc_dir.join("mod_index.json");

        let index_str = serde_json::to_string(&self)?;
        std::fs::write(&index_dir, &index_str).path(index_dir)?;
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
