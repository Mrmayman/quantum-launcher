use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct PluginJson {
    pub launcher_version: String,
    pub details: PluginDetails,
    pub files: Vec<PluginFile>,
    pub main_file: PluginFile,
    pub invoke: PluginInvoke,
    pub dependencies: Option<HashMap<String, PluginDependency>>,
    pub permissions: Vec<PluginPermission>,
}

#[derive(Serialize, Deserialize)]
pub struct PluginDetails {
    pub name: String,
    pub version: String,
}

#[derive(Serialize, Deserialize)]
pub struct PluginDependency {
    pub version: String,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PluginInvoke {
    LoaderInstaller { software: String },
    Library,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PluginPermission {
    Java,
}

#[derive(Serialize, Deserialize)]
pub struct PluginFile {
    pub filename: String,
    pub import: String,
}
