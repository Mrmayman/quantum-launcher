use ql_core::{file_utils, JsonDownloadError};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Manifest {
    pub latest: Latest,
    pub versions: Vec<Version>,
}

impl Manifest {
    pub async fn download() -> Result<Manifest, JsonDownloadError> {
        const VERSIONS_JSON: &str = "https://launchermeta.mojang.com/mc/game/version_manifest.json";

        let client = reqwest::Client::new();
        let manifest = file_utils::download_file_to_string(&client, VERSIONS_JSON, false).await?;
        Ok(serde_json::from_str(&manifest)?)
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
}
