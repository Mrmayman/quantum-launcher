use std::{
    collections::HashMap,
    sync::{atomic::AtomicI32, mpsc::Sender},
    time::Instant,
};

use chrono::DateTime;
use ql_core::{
    file_utils, json::VersionDetails, pt, GenericProgress, JsonDownloadError, RequestError, CLIENT,
};
use reqwest::header::HeaderValue;
use serde::Deserialize;

use crate::{
    mod_manager::{get_loader, ModIndex, SearchMod},
    rate_limiter::RATE_LIMITER,
};

use super::{Backend, ModError, SearchResult};

mod download;

const NOT_LOADED: i32 = -1;
pub static MC_ID: AtomicI32 = AtomicI32::new(NOT_LOADED);

#[derive(Deserialize, Clone, Debug)]
struct ModQuery {
    data: Mod,
}

impl ModQuery {
    pub async fn load(id: &str) -> Result<Self, JsonDownloadError> {
        let response = send_request(&format!("mods/{id}"), &HashMap::new()).await?;
        let response: ModQuery = serde_json::from_str(&response)?;
        Ok(response)
    }

    async fn get_file(
        &self,
        id: &str,
        version: &str,
        loader: Option<&str>,
    ) -> Result<CurseforgeFileQuery, ModError> {
        let Some(file) = self.data.latestFilesIndexes.iter().find(|n| {
            let is_loader_compatible = loader == n.modLoader.map(|n| n.to_string()).as_deref();
            let is_version_compatible = n.gameVersion == version;
            is_version_compatible && is_loader_compatible
        }) else {
            return Err(ModError::NoCompatibleVersionFound);
        };

        let file_query = CurseforgeFileQuery::load(id, file.fileId).await?;

        Ok(file_query)
    }
}

#[derive(Deserialize, Clone, Debug)]
#[allow(non_snake_case)]
struct Mod {
    name: String,
    slug: String,
    summary: String,
    downloadCount: usize,
    logo: Option<Logo>,
    id: i32,
    latestFilesIndexes: Vec<CurseforgeFileIdx>,
    // latestFiles: Vec<CurseforgeFile>,
}

#[derive(Deserialize, Clone, Debug)]
#[allow(non_snake_case)]
struct CurseforgeFileIdx {
    // filename: String,
    gameVersion: String,
    fileId: i32,
    modLoader: Option<i32>,
}

#[derive(Deserialize, Clone, Debug)]
struct CurseforgeFileQuery {
    data: CurseforgeFile,
}

impl CurseforgeFileQuery {
    pub async fn load(mod_id: &str, file_id: i32) -> Result<Self, JsonDownloadError> {
        let response =
            send_request(&format!("mods/{mod_id}/files/{file_id}"), &HashMap::new()).await?;
        let response: Self = serde_json::from_str(&response)?;
        Ok(response)
    }
}

#[derive(Deserialize, Clone, Debug)]
#[allow(non_snake_case)]
struct CurseforgeFile {
    fileName: String,
    downloadUrl: Option<String>,
    gameVersions: Vec<String>,
    dependencies: Vec<Dependency>,
    fileDate: String,
    displayName: String,
}

#[derive(Deserialize, Clone, Debug)]
#[allow(non_snake_case)]
struct Dependency {
    modId: usize,
}

#[derive(Deserialize, Clone, Debug)]
struct Logo {
    url: String,
}

pub struct CurseforgeBackend;

impl Backend for CurseforgeBackend {
    async fn search(query: super::Query, offset: usize) -> Result<SearchResult, super::ModError> {
        #[derive(Deserialize)]
        struct SearchResult {
            data: Vec<Mod>,
        }

        const TOTAL_DOWNLOADS: &str = "6";

        let _lock = RATE_LIMITER.lock().await;
        let instant = Instant::now();

        let mut params = HashMap::from([
            ("gameId", get_mc_id().await?.to_string()),
            ("gameVersion", query.version.clone()),
            ("modLoaderType", query.loader.to_curseforge().to_owned()),
            ("sortField", TOTAL_DOWNLOADS.to_owned()),
            ("sortOrder", "desc".to_owned()),
            ("index", offset.to_string()),
        ]);

        if !query.name.is_empty() {
            params.insert("searchFilter", query.name.clone());
        }

        let response = send_request("mods/search", &params).await?;
        let response: SearchResult = serde_json::from_str(&response)?;

        Ok(super::SearchResult {
            mods: response
                .data
                .into_iter()
                .map(|n| SearchMod {
                    title: n.name,
                    description: n.summary,
                    downloads: n.downloadCount,
                    internal_name: n.slug,
                    id: n.id.to_string(),
                    icon_url: n.logo.map(|n| n.url).unwrap_or_default(),
                })
                .collect(),
            start_time: instant,
            backend: ql_core::StoreBackendType::Curseforge,
            offset,
        })
    }

    async fn get_description(id: &str) -> Result<super::ModInformation, super::ModError> {
        #[derive(Deserialize)]
        struct Resp2 {
            data: String,
        }

        let map = HashMap::new();

        let response = ModQuery::load(id).await?;

        let description = send_request(&format!("mods/{id}/description"), &map).await?;
        let description: Resp2 = serde_json::from_str(&description)?;

        Ok(crate::mod_manager::ModInformation {
            title: response.data.name,
            description: response.data.summary,
            icon_url: response.data.logo.map(|n| n.url),
            id: ql_core::ModId::Curseforge(response.data.id.to_string()),
            long_description: description.data,
        })
    }

    async fn get_latest_version_date(
        id: &str,
        version: &str,
        loader: Option<ql_core::Loader>,
    ) -> Result<(DateTime<chrono::FixedOffset>, String), ModError> {
        let response = ModQuery::load(id).await?;
        let loader = loader.map(|n| n.to_curseforge());

        let file_query = response.get_file(id, version, loader).await?;

        let download_version_time = DateTime::parse_from_rfc3339(&file_query.data.fileDate)?;

        Ok((download_version_time, response.data.name))
    }

    async fn download(
        id: &str,
        instance: &ql_core::InstanceSelection,
    ) -> Result<(), super::ModError> {
        let version = {
            let version_json = VersionDetails::load(instance).await?;
            version_json.id
        };
        let loader = get_loader(instance).await?.map(|n| n.to_curseforge());
        let mut index = ModIndex::get(instance).await?;

        let mods_dir = file_utils::get_dot_minecraft_dir(instance)
            .await?
            .join("mods");

        download::download(id, &version, loader, &mut index, &mods_dir, None).await?;

        index.save(instance).await?;

        Ok(())
    }

    async fn download_bulk(
        ids: &[String],
        instance: &ql_core::InstanceSelection,
        ignore_incompatible: bool,
        set_manually_installed: bool,
        sender: Option<&Sender<GenericProgress>>,
    ) -> Result<(), super::ModError> {
        let version = {
            let version_json = VersionDetails::load(instance).await?;
            version_json.id
        };
        let loader = get_loader(instance).await?.map(|n| n.to_curseforge());
        let mut index = ModIndex::get(instance).await?;

        let mods_dir = file_utils::get_dot_minecraft_dir(instance)
            .await?
            .join("mods");

        let len = ids.len();
        for (i, id) in ids.iter().enumerate() {
            if let Some(sender) = &sender {
                _ = sender.send(GenericProgress {
                    done: i,
                    total: len,
                    message: None,
                    has_finished: false,
                });
            }

            let result =
                download::download(id, &version, loader, &mut index, &mods_dir, None).await;

            if let Err(ModError::NoCompatibleVersionFound) = result {
                if ignore_incompatible {
                    pt!("No compatible version found for mod {id}, skipping...");
                    continue;
                }
            }
            result?;

            if set_manually_installed {
                if let Some(config) = index.mods.get_mut(id) {
                    config.manually_installed = true;
                }
            }
        }

        index.save(instance).await?;
        pt!("Finished");
        if let Some(sender) = &sender {
            _ = sender.send(GenericProgress::finished());
        }

        Ok(())
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
