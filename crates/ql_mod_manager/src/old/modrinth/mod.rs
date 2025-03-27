use std::time::Instant;

use ql_core::{file_utils, InstanceSelection};

use crate::rate_limiter::RATE_LIMITER;

use super::{
    ModError, ModId, SearchMod, SearchQuery, SearchResult, StoreBackend, StoreBackendType,
};

mod download;
mod info;
mod search;
mod versions;

pub use download::{get_loader_type, version_sort, ModDownloader};
pub use versions::ModVersion;

pub struct ModrinthBackend;

impl StoreBackend for ModrinthBackend {
    async fn search(&self, query: SearchQuery) -> Result<(SearchResult, Instant), ModError> {
        let _lock = RATE_LIMITER.lock().await;
        let instant = Instant::now();
        let url = search::get_url(&query);

        let json = file_utils::download_file_to_string(&url, true).await?;
        let json: search::Search = serde_json::from_str(&json)?;

        let res = SearchResult {
            mods: json
                .hits
                .into_iter()
                .map(|n| SearchMod {
                    title: n.title,
                    description: n.description,
                    downloads: n.downloads,
                    internal_name: n.slug,
                    id: ModId::Modrinth(n.project_id),
                    icon_url: n.icon_url,
                })
                .collect(),
            backend: StoreBackendType::Modrinth,
        };

        Ok((res, instant))
    }

    async fn download(&self, id: &str, instance: &InstanceSelection) -> Result<(), ModError> {
        download::download_mod(id, instance).await?;

        Ok(())
    }
}
