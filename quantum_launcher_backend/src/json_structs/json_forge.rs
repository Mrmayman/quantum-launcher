use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{file_utils, json_structs::JsonDownloadError};

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
