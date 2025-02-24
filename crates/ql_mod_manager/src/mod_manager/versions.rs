use std::sync::mpsc::Sender;

use ql_core::{file_utils, info, pt, GenericProgress};
use serde::{Deserialize, Serialize};

use crate::rate_limiter::RATE_LIMITER;

use super::{Loader, ModError, RecommendedMod};

#[derive(Deserialize, Debug, Clone)]
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
        let url = format!("https://api.modrinth.com/v2/project/{project_id}/version");
        let file = file_utils::download_file_to_string(&url, false).await?;
        let file = serde_json::from_str(&file)?;
        Ok(file)
    }

    pub async fn get_compatible_mods_w(
        ids: Vec<RecommendedMod>,
        version: String,
        loader: Loader,
        sender: Sender<GenericProgress>,
    ) -> Result<Vec<RecommendedMod>, String> {
        Self::get_compatible_mods(ids, &version, &loader, &sender)
            .await
            .map_err(|e| e.to_string())
    }
    pub async fn get_compatible_mods(
        ids: Vec<RecommendedMod>,
        version: &String,
        loader: &Loader,
        sender: &Sender<GenericProgress>,
    ) -> Result<Vec<RecommendedMod>, ModError> {
        info!("Checking compatibility");
        let mut mods = vec![];
        let len = ids.len();
        for (i, id) in ids.into_iter().enumerate() {
            if sender
                .send(GenericProgress {
                    done: i,
                    total: len,
                    message: Some(format!("Checking compatibility: {}", id.name)),
                    has_finished: false,
                })
                .is_err()
            {
                info!("Cancelled recommended mod check");
                return Ok(Vec::new());
            }

            let is_compatible = Self::is_compatible(id.id, version, loader).await?;
            pt!("{} : {is_compatible}", id.name);
            if is_compatible {
                mods.push(id);
            }
        }
        Ok(mods)
    }

    pub async fn is_compatible(
        project_id: &str,
        minecraft_version: &String,
        instance_loader: &Loader,
    ) -> Result<bool, ModError> {
        let versions = Self::download(project_id).await?;
        Ok(versions.iter().any(|n| {
            n.game_versions.contains(minecraft_version)
                && n.loaders.contains(&instance_loader.to_string())
        }))
    }
}

#[derive(Deserialize, Debug, Clone)]
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
