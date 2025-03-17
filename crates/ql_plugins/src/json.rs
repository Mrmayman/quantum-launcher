use std::{collections::HashMap, path::Path};

use ql_core::{IntoIoError, IoError};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct PluginJson {
    pub launcher_version: String,
    pub details: PluginDetails,
    pub files: Vec<PluginFile>,
    pub main_file: PluginFile,
    pub includes: Option<Vec<PluginFile>>,
    pub invoke: PluginInvoke,
    pub dependencies: Option<HashMap<String, PluginDependency>>,
    pub permissions: Vec<PluginPermission>,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct PluginDetails {
    pub name: String,
    pub version: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PluginDependency {
    pub version: String,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum PluginInvoke {
    LoaderInstaller { software: String },
    Library,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone)]
#[serde(tag = "type")]
pub enum PluginPermission {
    Java,
    Request { whitelist: Vec<String> },
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PluginFile {
    pub filename: String,
    pub import: String,
}

impl PluginFile {
    pub fn load(
        &self,
        root_dir: &Path,
        mod_map: &mut HashMap<String, String>,
    ) -> Result<(), IoError> {
        let lua_file = root_dir.join(&self.filename);
        let lua_file = std::fs::read_to_string(&lua_file).path(lua_file)?;
        mod_map.insert(self.import.clone(), lua_file);
        Ok(())
    }
}
