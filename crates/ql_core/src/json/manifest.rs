use crate::{err, file_utils, pt, JsonDownloadError};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Manifest {
    pub latest: Latest,
    pub versions: Vec<Version>,
}

impl Manifest {
    pub async fn download() -> Result<Manifest, JsonDownloadError> {
        let m = match Self::download_from_omniarchive().await {
            Ok(n) => n,
            Err(err) => {
                err!("Could not get version list from Omniarchive: {err}");
                pt!("Downloading the official mojang version list");
                Self::download_from_mojang().await?
            }
        };
        Ok(m)
    }

    async fn download_from_mojang() -> Result<Manifest, JsonDownloadError> {
        const VERSIONS_JSON: &str = "https://launchermeta.mojang.com/mc/game/version_manifest.json";

        let client = reqwest::Client::new();
        let manifest = file_utils::download_file_to_string(&client, VERSIONS_JSON, false).await?;
        Ok(serde_json::from_str(&manifest)?)
    }

    async fn download_from_omniarchive() -> Result<Manifest, JsonDownloadError> {
        // Here's a really funny story.
        // I implemented a full blown web scraper for downloading
        // omniarchive versions just for them to come up with this
        // API 2 WEEKS LATER!!! All my work is obsolete.
        const VERSIONS_JSON: &str = "https://meta.omniarchive.uk/v1/manifest.json";

        let client = reqwest::Client::new();
        let manifest = file_utils::download_file_to_string(&client, VERSIONS_JSON, false).await?;
        Ok(serde_json::from_str(&manifest)?)
    }

    pub fn find_name(&self, name: &str) -> Option<&Version> {
        self.versions.iter().find(|n| n.id == name)
    }
}

#[derive(Serialize, Deserialize)]
pub struct Latest {
    pub release: String,
    pub snapshot: String,
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize)]
pub struct Version {
    pub id: String,
    pub r#type: String,
    pub url: String,
    pub time: String,
    pub releaseTime: String,
    pub phase: Option<String>,
}
