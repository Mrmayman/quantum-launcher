use ql_core::file_utils;
use serde::Deserialize;

use crate::{rate_limiter::RATE_LIMITER, store::local_json::ModFile};

use super::ModError;

#[derive(Deserialize, Debug, Clone)]
pub struct ModVersion {
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
    // pub id: String,
    // pub project_id: String,
    // pub author_id: String,
    // pub featured: bool,
    pub name: String,
    pub version_number: String,
    // pub changelog: String,
    // pub changelog_url: Option<String>,
    pub date_published: String,
    // pub downloads: usize,
    // pub version_type: String,
    // pub status: String,
    // pub requested_status: Option<String>,
    pub files: Vec<ModFile>,
    pub dependencies: Vec<Dependency>,
}

impl ModVersion {
    pub async fn download(project_id: &str) -> Result<Vec<Self>, ModError> {
        let _lock = RATE_LIMITER.lock().await;
        let url = format!("https://api.modrinth.com/v2/project/{project_id}/version");
        let file = file_utils::download_file_to_string(&url, false).await?;
        let file = serde_json::from_str(&file)?;
        Ok(file)
    }

    // pub async fn is_compatible(
    //     project_id: &str,
    //     minecraft_version: &String,
    //     instance_loader: &Loader,
    // ) -> Result<bool, ModError> {
    //     let versions = Self::download(project_id).await?;
    //     Ok(versions.iter().any(|n| {
    //         n.game_versions.contains(minecraft_version)
    //             && n.loaders
    //                 .contains(&instance_loader.to_modrinth_str().to_owned())
    //     }))
    // }
}

#[derive(Deserialize, Debug, Clone)]
#[allow(clippy::struct_field_names)]
pub struct Dependency {
    // pub version_id: Option<serde_json::Value>,
    pub project_id: Option<String>,
    // pub file_name: Option<serde_json::Value>,
    pub dependency_type: String,
}
