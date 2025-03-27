use std::{
    collections::HashMap,
    sync::{atomic::AtomicI32, mpsc::Sender},
    time::Instant,
};

use ql_core::{GenericProgress, RequestError, CLIENT};
use reqwest::header::HeaderValue;
use serde::Deserialize;

use crate::{mod_manager::SearchMod, rate_limiter::RATE_LIMITER};

use super::{Backend, ModError};

const NOT_LOADED: i32 = -1;
lazy_static::lazy_static!(
    pub static ref MC_ID: AtomicI32 = AtomicI32::new(NOT_LOADED);
);

#[derive(Deserialize)]
#[allow(non_snake_case)]
struct Mod {
    name: String,
    slug: String,
    summary: String,
    downloadCount: usize,
    logo: Logo,
    id: i32,
}

#[derive(Deserialize)]
struct Logo {
    url: String,
}

pub struct CurseforgeBackend;

impl Backend for CurseforgeBackend {
    async fn search(
        query: super::Query,
    ) -> Result<(super::SearchResult, std::time::Instant), super::ModError> {
        let _lock = RATE_LIMITER.lock().await;
        let instant = Instant::now();

        #[derive(Deserialize)]
        struct SearchResult {
            data: Vec<Mod>,
        }

        const TOTAL_DOWNLOADS: &str = "6";

        let mut params = HashMap::from([
            ("gameId", get_mc_id().await?.to_string()),
            ("gameVersion", query.version.clone()),
            ("modLoaderType", query.loader.to_curseforge().to_owned()),
            ("sortField", TOTAL_DOWNLOADS.to_owned()),
            ("sortOrder", "desc".to_owned()),
        ]);

        if !query.name.is_empty() {
            params.insert("searchFilter", query.name.clone());
        }

        let response = send_request("mods/search", &params).await?;
        let response: SearchResult = serde_json::from_str(&response)?;

        Ok((
            super::SearchResult {
                mods: response
                    .data
                    .into_iter()
                    .map(|n| SearchMod {
                        title: n.name,
                        description: n.summary,
                        downloads: n.downloadCount,
                        internal_name: n.slug,
                        id: n.id.to_string(),
                        icon_url: n.logo.url,
                    })
                    .collect(),
                backend: ql_core::StoreBackendType::Curseforge,
            },
            instant,
        ))
    }

    async fn get_description(id: &str) -> Result<super::ModInformation, super::ModError> {
        #[derive(Deserialize)]
        struct Resp1 {
            data: Mod,
        }

        #[derive(Deserialize)]
        struct Resp2 {
            data: String,
        }

        let map = HashMap::new();

        let response = send_request(&format!("mods/{id}"), &map).await?;
        let response: Resp1 = serde_json::from_str(&response)?;

        let description = send_request(&format!("mods/{id}/description"), &map).await?;
        let description: Resp2 = serde_json::from_str(&description)?;

        Ok(crate::mod_manager::ModInformation {
            title: response.data.name,
            description: response.data.summary,
            icon_url: Some(response.data.logo.url),
            id: ql_core::ModId::Curseforge(response.data.id.to_string()),
            long_description: description.data,
        })
    }

    async fn get_latest_version_date(
        id: &str,
        version: &str,
        loader: Option<ql_core::Loader>,
    ) -> Option<(chrono::DateTime<chrono::FixedOffset>, String)> {
        todo!()
    }

    async fn download(
        id: &str,
        instance: &ql_core::InstanceSelection,
    ) -> Result<(), super::ModError> {
        todo!()
    }

    async fn download_bulk(
        ids: &[String],
        instance: &ql_core::InstanceSelection,
        ignore_incompatible: bool,
        set_manually_installed: bool,
        sender: Option<&Sender<GenericProgress>>,
    ) -> Result<(), super::ModError> {
        todo!()
    }
}

async fn send_request(api: &str, params: &HashMap<&str, String>) -> Result<String, RequestError> {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::ACCEPT,
        HeaderValue::from_static("application/json"),
    );
    headers.insert("x-api-key", HeaderValue::from_str(API_KEY)?);

    let url = format!("https://api.curseforge.com/v1/{api}");
    let response = CLIENT
        .get(&url)
        .headers(headers)
        .query(params)
        .send()
        .await?;

    if response.status().is_success() {
        Ok(response.text().await?)
    } else {
        Err(RequestError::DownloadError {
            code: response.status(),
            url: response.url().clone(),
        })
    }
}

// Please don't steal :)
const API_KEY: &str = "$2a$10$2SyApFh1oojq/d6z8axjRO6I8yrWI8.m0BTJ20vXNTWfy2O0X5Zsa";

pub async fn get_mc_id() -> Result<i32, ModError> {
    #[derive(Deserialize)]
    struct Response {
        data: Vec<Game>,
    }

    #[derive(Deserialize)]
    struct Game {
        id: i32,
        name: String,
    }

    let val = MC_ID.load(std::sync::atomic::Ordering::Acquire);

    if val == NOT_LOADED {
        let params = HashMap::new();

        let response = send_request("games", &params).await?;
        let response: Response = serde_json::from_str(&response)?;

        let Some(minecraft) = response
            .data
            .iter()
            .find(|n| n.name.eq_ignore_ascii_case("Minecraft"))
        else {
            return Err(ModError::NoMinecraftInCurseForge);
        };

        MC_ID.store(minecraft.id, std::sync::atomic::Ordering::Release);

        Ok(minecraft.id)
    } else {
        Ok(val)
    }
}
