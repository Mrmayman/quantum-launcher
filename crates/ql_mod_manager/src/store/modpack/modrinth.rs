use std::collections::HashMap;

use serde::Deserialize;

#[derive(Deserialize)]
pub struct PackIndex {
    pub name: String,
    pub files: Vec<PackFile>,

    /// Info about which Minecraft version
    /// and Loader version is required. May contain:
    ///
    /// - `minecraft` (always present)
    /// - `forge`
    /// - `neoforge`
    /// - `fabric-loader`
    /// - `quilt-loader`
    pub dependencies: HashMap<String, String>,
}

#[derive(Deserialize)]
pub struct PackFile {
    pub path: String,
    pub env: PackEnv,
    pub downloads: Vec<String>,
}

#[derive(Deserialize)]
pub struct PackEnv {
    pub client: String,
    pub server: String,
}
