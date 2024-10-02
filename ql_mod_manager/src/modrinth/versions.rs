use ql_instances::file_utils;
use serde::{Deserialize, Serialize};

use super::ModDownloadError;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModVersion {
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
    pub id: String,
    pub project_id: String,
    pub author_id: String,
    pub featured: bool,
    pub name: String,
    pub version_number: String,
    pub changelog: String,
    // pub changelog_url: Option<String>,
    pub date_published: String,
    pub downloads: usize,
    pub version_type: String,
    pub status: String,
    // pub requested_status: Option<String>,
    pub files: Vec<ModFile>,
}

impl ModVersion {
    pub async fn download(project_id: &str) -> Result<Vec<Self>, ModDownloadError> {
        let _lock = ql_instances::RATE_LIMITER.lock().await;
        let client = reqwest::Client::new();
        let url = format!("https://api.modrinth.com/v2/project/{project_id}/version");
        let file = file_utils::download_file_to_string(&client, &url, false).await?;
        let file = serde_json::from_str(&file)?;
        Ok(file)
    }

    pub async fn download_wrapped(project_id: String) -> Result<(Vec<Self>, String), String> {
        Self::download(&project_id)
            .await
            .map_err(|err| err.to_string())
            .map(|n| (n, project_id))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModFile {
    pub hashes: ModHashes,
    pub url: String,
    pub filename: String,
    pub primary: bool,
    pub size: usize,
    // pub file_type: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModHashes {
    pub sha512: String,
    pub sha1: String,
}
