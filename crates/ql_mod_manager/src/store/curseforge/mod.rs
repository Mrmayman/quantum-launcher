use std::{
    collections::HashMap,
    sync::{atomic::AtomicI32, mpsc::Sender},
    time::Instant,
};

use chrono::DateTime;
use ql_core::{
    json::VersionDetails, pt, GenericProgress, IntoJsonError, JsonDownloadError, ModId,
    RequestError, CLIENT,
};
use reqwest::header::HeaderValue;
use serde::Deserialize;

use crate::{
    rate_limiter::RATE_LIMITER,
    store::{get_loader, ModIndex, SearchMod},
};

use super::{get_mods_resourcepacks_shaderpacks_dir, Backend, ModError, QueryType, SearchResult};
use categories::get_categories;

mod categories;
mod download;

const NOT_LOADED: i32 = -1;
pub static MC_ID: AtomicI32 = AtomicI32::new(NOT_LOADED);

#[derive(Deserialize, Clone, Debug)]
pub struct ModQuery {
    pub data: Mod,
}

impl ModQuery {
    pub async fn load(id: &str) -> Result<Self, JsonDownloadError> {
        let response = send_request(&format!("mods/{id}"), &HashMap::new()).await?;
        let response: ModQuery = serde_json::from_str(&response).json(response)?;
        Ok(response)
    }
}

#[derive(Deserialize, Clone, Debug)]
#[allow(non_snake_case)]
pub struct Mod {
    pub name: String,
    pub slug: String,
    pub summary: String,
    pub downloadCount: usize,
    pub logo: Option<Logo>,
    pub id: i32,
    pub latestFilesIndexes: Vec<CurseforgeFileIdx>,
    pub classId: i32,
    // latestFiles: Vec<CurseforgeFile>,
}

impl Mod {
    async fn get_file(
        &self,
        title: String,
        id: &str,
        version: &str,
        loader: Option<&str>,
        query_type: QueryType,
    ) -> Result<CurseforgeFileQuery, ModError> {
        let Some(file) = self.latestFilesIndexes.iter().find(|n| {
            let is_loader_compatible = loader == n.modLoader.map(|n| n.to_string()).as_deref();
            let is_version_compatible = n.gameVersion == version;
            (query_type != QueryType::Mods) || (is_version_compatible && is_loader_compatible)
        }) else {
            return Err(ModError::NoCompatibleVersionFound(title));
        };

        let file_query = CurseforgeFileQuery::load(id, file.fileId).await?;

        Ok(file_query)
    }
}

#[derive(Deserialize, Clone, Debug)]
#[allow(non_snake_case)]
pub struct CurseforgeFileIdx {
    // filename: String,
    gameVersion: String,
    fileId: i32,
    modLoader: Option<i32>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct CurseforgeFileQuery {
    pub data: CurseforgeFile,
}

impl CurseforgeFileQuery {
    pub async fn load(mod_id: &str, file_id: i32) -> Result<Self, JsonDownloadError> {
        let response =
            send_request(&format!("mods/{mod_id}/files/{file_id}"), &HashMap::new()).await?;
        let response: Self = serde_json::from_str(&response).json(response)?;
        Ok(response)
    }
}

#[derive(Deserialize, Clone, Debug)]
#[allow(non_snake_case)]
pub struct CurseforgeFile {
    pub fileName: String,
    pub downloadUrl: Option<String>,
    pub gameVersions: Vec<String>,
    pub dependencies: Vec<Dependency>,
    pub fileDate: String,
    pub displayName: String,
}

#[derive(Deserialize, Clone, Debug)]
#[allow(non_snake_case)]
pub struct Dependency {
    modId: usize,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Logo {
    url: String,
}

#[derive(Deserialize)]
struct CFSearchResult {
    data: Vec<Mod>,
}

impl CFSearchResult {
    async fn get_from_ids(ids: &[String]) -> Result<Self, super::ModError> {
        if ids.is_empty() {
            return Ok(Self { data: Vec::new() });
        }

        // Convert to JSON Array
        let ids: Vec<serde_json::Value> = ids
            .iter()
            .map(|s| s.parse::<u64>().map(serde_json::Value::from))
            .collect::<Result<_, _>>()?;

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::ACCEPT,
            HeaderValue::from_static("application/json"),
        );
        headers.insert(
            "x-api-key",
            HeaderValue::from_str(API_KEY).map_err(RequestError::from)?,
        );
        let response = CLIENT
            .post("https://api.curseforge.com/v1/mods")
            .headers(headers)
            .json(&serde_json::json!({"modIds" : ids}))
            .send()
            .await
            .map_err(RequestError::from)?;
        if response.status().is_success() {
            Ok(response.json().await.map_err(RequestError::from)?)
        } else {
            Err(RequestError::DownloadError {
                code: response.status(),
                url: response.url().clone(),
            }
            .into())
        }
    }
}

pub struct CurseforgeBackend;

impl Backend for CurseforgeBackend {
    async fn search(
        query: super::Query,
        offset: usize,
        query_type: QueryType,
    ) -> Result<SearchResult, super::ModError> {
        const TOTAL_DOWNLOADS: &str = "6";

        let _lock = RATE_LIMITER.lock().await;
        let instant = Instant::now();

        let mut params = HashMap::from([
            ("gameId", get_mc_id().await?.to_string()),
            ("sortField", TOTAL_DOWNLOADS.to_owned()),
            ("sortOrder", "desc".to_owned()),
            ("index", offset.to_string()),
        ]);

        if let QueryType::Mods = query_type {
            if let Some(loader) = query.loader {
                params.insert("modLoaderType", loader.to_curseforge().to_owned());
            }
            params.insert("gameVersion", query.version.clone());
        }

        let categories = get_categories().await?;
        let query_type_str = query_type.to_curseforge_str();
        if let Some(category) = categories.data.iter().find(|n| n.slug == query_type_str) {
            params.insert("classId", category.id.to_string());
        }

        if !query.name.is_empty() {
            params.insert("searchFilter", query.name.clone());
        }

        let response = send_request("mods/search", &params).await?;
        let response: CFSearchResult = serde_json::from_str(&response).json(response)?;

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
                    project_type: query_type_str.to_owned(),
                    icon_url: n.logo.map(|n| n.url).unwrap_or_default(),
                })
                .collect(),
            start_time: instant,
            backend: ql_core::StoreBackendType::Curseforge,
            offset,
        })
    }

    async fn get_description(id: &str) -> Result<(ModId, String), super::ModError> {
        #[derive(Deserialize)]
        struct Resp2 {
            data: String,
        }

        let map = HashMap::new();
        let description = send_request(&format!("mods/{id}/description"), &map).await?;
        let description: Resp2 = serde_json::from_str(&description).json(description)?;

        Ok((ModId::Curseforge(id.to_string()), description.data))
    }

    async fn get_latest_version_date(
        id: &str,
        version: &str,
        loader: Option<ql_core::Loader>,
    ) -> Result<(DateTime<chrono::FixedOffset>, String), ModError> {
        let response = ModQuery::load(id).await?;
        let loader = loader.map(|n| n.to_curseforge());

        let query_type = get_query_type(response.data.classId).await?;
        let file_query = response
            .data
            .get_file(response.data.name.clone(), id, version, loader, query_type)
            .await?;

        let download_version_time = DateTime::parse_from_rfc3339(&file_query.data.fileDate)?;

        Ok((download_version_time, response.data.name))
    }

    async fn download(
        id: &str,
        instance: &ql_core::InstanceSelection,
    ) -> Result<(), super::ModError> {
        let version_json = VersionDetails::load(instance).await?;
        let loader = get_loader(instance).await?.map(|n| n.to_curseforge());
        let mut index = ModIndex::get(instance).await?;

        let (mods_dir, resourcepacks_dir, shaderpacks_dir) =
            get_mods_resourcepacks_shaderpacks_dir(instance, &version_json).await?;

        let mut cache = HashMap::new();
        download::download(
            id,
            &version_json.id,
            loader,
            &mut index,
            (&mods_dir, &resourcepacks_dir, &shaderpacks_dir),
            None,
            &mut cache,
        )
        .await?;

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
        let version_json = VersionDetails::load(instance).await?;
        let loader = get_loader(instance).await?.map(|n| n.to_curseforge());
        let mut index = ModIndex::get(instance).await?;

        let (mods_dir, resourcepacks_dir, shaderpacks_dir) =
            get_mods_resourcepacks_shaderpacks_dir(instance, &version_json).await?;

        let mut cache = CFSearchResult::get_from_ids(ids)
            .await?
            .data
            .into_iter()
            .map(|n| (n.id.to_string(), n))
            .collect();

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

            let result = download::download(
                id,
                &version_json.id,
                loader,
                &mut index,
                (&mods_dir, &resourcepacks_dir, &shaderpacks_dir),
                None,
                &mut cache,
            )
            .await;

            if let Err(ModError::NoCompatibleVersionFound(name)) = &result {
                if ignore_incompatible {
                    pt!("No compatible version found for mod {name} ({id}), skipping...");
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

pub async fn send_request(
    api: &str,
    params: &HashMap<&str, String>,
) -> Result<String, RequestError> {
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
        let response: Response = serde_json::from_str(&response).json(response)?;

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

pub async fn get_query_type(class_id: i32) -> Result<QueryType, ModError> {
    let categories = get_categories().await?;
    Ok(
        if let Some(category) = categories.data.iter().find(|n| n.id == class_id) {
            QueryType::from_curseforge_str(&category.slug)
                .ok_or(ModError::UnknownProjectType(category.slug.clone()))?
        } else {
            QueryType::Mods
        },
    )
}
