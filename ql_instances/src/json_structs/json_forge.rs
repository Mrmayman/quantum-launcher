use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::file_utils;

use super::JsonDownloadError;

#[derive(Serialize, Deserialize)]
pub struct JsonForgeVersions {
    homepage: String,
    promos: HashMap<String, String>,
}

impl JsonForgeVersions {
    pub async fn download() -> Result<Self, JsonDownloadError> {
        const VERSIONS_JSON: &str =
            "https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json";

        let client = reqwest::Client::new();
        let manifest = file_utils::download_file_to_string(&client, VERSIONS_JSON).await?;
        Ok(serde_json::from_str(&manifest)?)
    }

    pub fn get_forge_version(&self, minecraft_version: &str) -> Option<String> {
        self.promos
            .iter()
            .find(|(version_mc, _)| *version_mc == &format!("{minecraft_version}-latest"))
            .map(|n| n.1.to_owned())
    }
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize)]
pub struct JsonForgeDetails {
    pub id: String,
    pub time: String,
    pub releaseTime: String,
    pub r#type: String,
    pub mainClass: String,
    pub inheritsFrom: String,
    pub logging: serde_json::Value,
    pub arguments: JsonForgeDetailsArguments,
    pub libraries: Vec<JsonForgeDetailsLibrary>,
}

#[derive(Serialize, Deserialize)]
pub struct JsonForgeDetailsArguments {
    pub game: Vec<String>,
    pub jvm: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct JsonForgeDetailsLibrary {
    pub name: String,
    pub url: Option<String>,
    pub downloads: JsonForgeDetailsDownloads,
    pub clientreq: Option<bool>,
}

#[derive(Serialize, Deserialize)]
pub struct JsonForgeDetailsDownloads {
    pub artifact: JsonForgeDetailsArtifact,
}

#[derive(Serialize, Deserialize)]
pub struct JsonForgeDetailsArtifact {
    pub path: Option<String>,
    pub url: Option<String>,
    pub sha1: String,
    pub size: usize,
}
