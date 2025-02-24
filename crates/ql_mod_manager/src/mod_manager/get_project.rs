use ql_core::{err, file_utils};
use serde::Deserialize;

use crate::rate_limiter::RATE_LIMITER;

use super::ModError;

#[derive(Deserialize, Debug, Clone)]
pub struct ProjectInfo {
    pub slug: String,
    pub title: String,
    pub description: String,
    pub categories: Vec<String>,
    pub client_side: String,
    pub server_side: String,
    pub body: String,
    pub status: String,
    pub requested_status: Option<String>,
    pub additional_categories: Vec<String>,
    pub issues_url: Option<String>,
    pub source_url: Option<String>,
    pub wiki_url: Option<String>,
    pub discord_url: Option<String>,
    pub donation_urls: Vec<DonationLink>,
    pub project_type: String,
    pub downloads: usize,
    pub icon_url: Option<String>,
    pub color: Option<usize>,
    pub thread_id: Option<String>,
    pub monetization_status: Option<String>,
    pub id: String,
    pub team: String,
    pub published: String,
    pub updated: String,
    pub approved: Option<String>,
    pub followers: usize,
    pub license: License,
    pub versions: Vec<String>,
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
    pub gallery: Vec<GalleryItem>,
}

impl ProjectInfo {
    pub async fn download(id: String) -> Result<Self, ModError> {
        let _lock = RATE_LIMITER.lock().await;
        let url = format!("https://api.modrinth.com/v2/project/{id}");
        let file = file_utils::download_file_to_string(&url, true).await?;
        let file: Self = match serde_json::from_str(&file) {
            Ok(file) => file,
            Err(err) => {
                err!("Could not parse mod project json from url: {url}");
                return Err(err.into());
            }
        };
        Ok(file)
    }

    pub async fn download_w(id: String) -> Result<Box<Self>, String> {
        Self::download(id)
            .await
            .map_err(|err| err.to_string())
            .map(Box::new)
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct DonationLink {
    pub id: String,
    pub platform: String,
    pub url: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct License {
    pub id: String,
    pub name: String,
    pub url: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GalleryItem {
    pub url: String,
    pub featured: bool,
    pub title: Option<String>,
    pub description: Option<String>,
    pub created: String,
    pub ordering: i64,
}
