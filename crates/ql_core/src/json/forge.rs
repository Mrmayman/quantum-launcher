use std::collections::HashMap;

use crate::{file_utils, JsonDownloadError};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct JsonVersions {
    promos: HashMap<String, String>,
}

impl JsonVersions {
    /// Downloads the Forge versions JSON file from the Forge website.
    ///
    /// # Errors
    /// If the file cannot be:
    /// - Downloaded (maybe bad internet or server down).
    /// - Parsed into JSON.
    pub async fn download() -> Result<Self, JsonDownloadError> {
        const VERSIONS_JSON: &str =
            "https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json";

        let manifest = file_utils::download_file_to_string(VERSIONS_JSON, false).await?;
        Ok(serde_json::from_str(&manifest)?)
    }

    /// Returns the Forge version for the given Minecraft version.
    #[must_use]
    pub fn get_forge_version(&self, minecraft_version: &str) -> Option<String> {
        self.promos
            .iter()
            .find(|(version_mc, _)| *version_mc == &format!("{minecraft_version}-latest"))
            .map(|n| n.1.clone())
    }
}

#[allow(non_snake_case)]
#[derive(Deserialize)]
pub struct JsonInstallProfile {
    pub install: serde_json::Value,
    pub versionInfo: JsonDetails,
}

#[allow(non_snake_case)]
#[derive(Deserialize)]
pub struct JsonDetails {
    pub id: String,
    pub time: String,
    pub releaseTime: String,
    pub r#type: String,
    pub mainClass: String,
    pub inheritsFrom: Option<String>,
    pub logging: Option<serde_json::Value>,
    pub arguments: Option<JsonDetailsArguments>,
    pub libraries: Vec<JsonDetailsLibrary>,
    pub minecraftArguments: Option<String>,
}

#[derive(Deserialize)]
pub struct JsonDetailsArguments {
    pub game: Vec<String>,
    pub jvm: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub struct JsonDetailsLibrary {
    pub name: String,
    pub url: Option<String>,
    pub downloads: Option<JsonDetailsDownloads>,
    pub clientreq: Option<bool>,
}

#[derive(Deserialize)]
pub struct JsonDetailsDownloads {
    pub artifact: JsonDetailsArtifact,
}

#[derive(Deserialize)]
pub struct JsonDetailsArtifact {
    pub path: String,
    pub url: String,
    pub sha1: String,
    pub size: usize,
}
