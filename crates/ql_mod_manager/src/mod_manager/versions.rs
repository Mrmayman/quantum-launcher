use ql_core::file_utils;
use serde::{Deserialize, Serialize};

use crate::rate_limiter::RATE_LIMITER;

use super::ModError;

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
    pub dependencies: Vec<Dependency>,
}

impl ModVersion {
    pub async fn download(project_id: &str) -> Result<Vec<Self>, ModError> {
        let _lock = RATE_LIMITER.lock().await;
        let client = reqwest::Client::new();
        let url = format!("https://api.modrinth.com/v2/project/{project_id}/version");
        let file = file_utils::download_file_to_string(&client, &url, false).await?;
        let file = serde_json::from_str(&file)?;
        Ok(file)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[allow(clippy::struct_field_names)]
pub struct Dependency {
    pub version_id: Option<serde_json::Value>,
    pub project_id: String,
    pub file_name: Option<serde_json::Value>,
    pub dependency_type: String,
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
