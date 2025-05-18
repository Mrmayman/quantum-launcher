use std::{collections::HashSet, fmt::Display, path::PathBuf, sync::mpsc::Sender, time::Instant};

use chrono::DateTime;
use ql_core::{
    err,
    json::{InstanceConfigJson, VersionDetails},
    GenericProgress, InstanceSelection, IntoIoError, Loader, ModId, StoreBackendType,
};

mod add_file;
mod curseforge;
mod delete;
mod error;
mod image;
mod local_json;
mod modpack;
mod modrinth;
mod recommended;
mod toggle;
mod update;

pub use add_file::add_files;
pub use curseforge::CurseforgeBackend;
pub use delete::delete_mods;
pub use error::ModError;
pub use image::{download_image, ImageResult};
pub use local_json::{ModConfig, ModFile, ModIndex};
pub use modpack::install_modpack;
pub use modrinth::ModrinthBackend;
pub use recommended::{RecommendedMod, RECOMMENDED_MODS};
pub use toggle::{flip_filename, toggle_mods, toggle_mods_local};
pub use update::{apply_updates, check_for_updates};

pub const SOURCE_ID_MODRINTH: &str = "modrinth";
pub const SOURCE_ID_CURSEFORGE: &str = "curseforge";

#[allow(async_fn_in_trait)]
pub trait Backend {
    async fn search(
        query: Query,
        offset: usize,
        query_type: QueryType,
    ) -> Result<SearchResult, ModError>;
    /// Gets the description of a mod based on its id.
    /// Returns the id and description `String`.
    ///
    /// This supports both Markdown and HTML.
    async fn get_description(id: &str) -> Result<(ModId, String), ModError>;
    async fn get_latest_version_date(
        id: &str,
        version: &str,
        loader: Option<Loader>,
    ) -> Result<(DateTime<chrono::FixedOffset>, String), ModError>;

    async fn download(
        id: &str,
        instance: &InstanceSelection,
        sender: Option<Sender<GenericProgress>>,
    ) -> Result<HashSet<CurseforgeNotAllowed>, ModError>;

    async fn download_bulk(
        ids: &[String],
        instance: &InstanceSelection,
        ignore_incompatible: bool,
        set_manually_installed: bool,
        sender: Option<&Sender<GenericProgress>>,
    ) -> Result<HashSet<CurseforgeNotAllowed>, ModError>;
}

pub async fn get_description(id: ModId) -> Result<(ModId, String), ModError> {
    match &id {
        ModId::Modrinth(n) => ModrinthBackend::get_description(n).await,
        ModId::Curseforge(n) => CurseforgeBackend::get_description(n).await,
    }
}

pub async fn search(
    query: Query,
    offset: usize,
    backend: StoreBackendType,
    query_type: QueryType,
) -> Result<SearchResult, ModError> {
    match backend {
        StoreBackendType::Modrinth => ModrinthBackend::search(query, offset, query_type).await,
        StoreBackendType::Curseforge => CurseforgeBackend::search(query, offset, query_type).await,
    }
}

pub async fn download_mod(
    id: &ModId,
    instance: &InstanceSelection,
    sender: Option<Sender<GenericProgress>>,
) -> Result<HashSet<CurseforgeNotAllowed>, ModError> {
    match id {
        ModId::Modrinth(n) => ModrinthBackend::download(n, instance, sender).await,
        ModId::Curseforge(n) => CurseforgeBackend::download(n, instance, sender).await,
    }
}

pub async fn download_mods_bulk(
    ids: Vec<ModId>,
    instance_name: InstanceSelection,
    sender: Option<Sender<GenericProgress>>,
) -> Result<HashSet<CurseforgeNotAllowed>, ModError> {
    let (modrinth, other): (Vec<ModId>, Vec<ModId>) = ids.into_iter().partition(|n| match n {
        ModId::Modrinth(_) => true,
        ModId::Curseforge(_) => false,
    });

    let modrinth: Vec<String> = modrinth
        .into_iter()
        .map(|n| n.get_internal_id().to_owned())
        .collect();

    let curseforge: Vec<String> = other
        .into_iter()
        .map(|n| n.get_internal_id().to_owned())
        .collect();

    // if !other.is_empty() {
    //     err!("Unimplemented downloading for mods: {other:#?}");
    // }

    let not_allowed =
        ModrinthBackend::download_bulk(&modrinth, &instance_name, true, true, sender.as_ref())
            .await?;
    debug_assert!(not_allowed.is_empty());

    let not_allowed =
        CurseforgeBackend::download_bulk(&curseforge, &instance_name, true, true, sender.as_ref())
            .await?;

    Ok(not_allowed)
}

pub async fn get_latest_version_date(
    loader: Option<Loader>,
    mod_id: &ModId,
    version: &str,
) -> Result<(DateTime<chrono::FixedOffset>, String), ModError> {
    Ok(match mod_id {
        ModId::Modrinth(n) => ModrinthBackend::get_latest_version_date(n, version, loader).await?,
        ModId::Curseforge(n) => {
            CurseforgeBackend::get_latest_version_date(n, version, loader).await?
        }
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueryType {
    Mods,
    ResourcePacks,
    Shaders,
    ModPacks,
    // TODO:
    // DataPacks,
    // Plugins,
}

impl Display for QueryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                QueryType::Mods => "Mods",
                QueryType::ResourcePacks => "Resource Packs",
                QueryType::Shaders => "Shaders",
                QueryType::ModPacks => "Modpacks",
            }
        )
    }
}

impl QueryType {
    pub const ALL: &[Self] = &[
        Self::Mods,
        Self::ResourcePacks,
        Self::Shaders,
        Self::ModPacks,
    ];

    #[must_use]
    pub fn to_modrinth_str(&self) -> &'static str {
        match self {
            QueryType::Mods => "mod",
            QueryType::ResourcePacks => "resourcepack",
            QueryType::Shaders => "shader",
            QueryType::ModPacks => "modpack",
        }
    }

    #[must_use]
    pub fn from_modrinth_str(s: &str) -> Option<Self> {
        match s {
            "mod" => Some(QueryType::Mods),
            "resourcepack" => Some(QueryType::ResourcePacks),
            "shader" => Some(QueryType::Shaders),
            "modpack" => Some(QueryType::ModPacks),
            _ => None,
        }
    }

    #[must_use]
    pub fn to_curseforge_str(&self) -> &'static str {
        match self {
            QueryType::Mods => "mc-mods",
            QueryType::ResourcePacks => "texture-packs",
            QueryType::Shaders => "shaders",
            QueryType::ModPacks => "modpacks",
        }
    }

    #[must_use]
    pub fn from_curseforge_str(s: &str) -> Option<Self> {
        match s {
            "mc-mods" => Some(QueryType::Mods),
            "texture-packs" => Some(QueryType::ResourcePacks),
            "shaders" => Some(QueryType::Shaders),
            "modpacks" => Some(QueryType::ModPacks),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Query {
    pub name: String,
    pub version: String,
    pub loader: Option<Loader>,
    pub server_side: bool,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub mods: Vec<SearchMod>,
    pub backend: StoreBackendType,
    pub start_time: Instant,
    pub offset: usize,
}

#[derive(Debug, Clone)]
pub struct SearchMod {
    pub title: String,
    pub description: String,
    pub downloads: usize,
    pub internal_name: String,
    pub project_type: String,
    pub id: String,
    pub icon_url: String,
}

async fn get_loader(instance: &InstanceSelection) -> Result<Option<Loader>, ModError> {
    let instance_dir = instance.get_instance_path();
    let config_json = InstanceConfigJson::read_from_path(&instance_dir).await?;

    Ok(match config_json.mod_type.as_str() {
        "Fabric" => Some(Loader::Fabric),
        "Forge" => Some(Loader::Forge),
        "Quilt" => Some(Loader::Quilt),
        "NeoForge" => Some(Loader::Neoforge),
        "LiteLoader" => Some(Loader::Liteloader),
        "Rift" => Some(Loader::Rift),
        "OptiFine" => Some(Loader::OptiFine),
        loader => {
            if loader != "Vanilla" {
                err!("Unknown loader {loader}");
            }
            None
        } // TODO: Add more loaders
    })
}

async fn get_mods_resourcepacks_shaderpacks_dir(
    instance_name: &InstanceSelection,
    version_json: &VersionDetails,
) -> Result<(PathBuf, PathBuf, PathBuf), ModError> {
    let dot_minecraft_dir = instance_name.get_dot_minecraft_path();
    let mods_dir = dot_minecraft_dir.join("mods");
    tokio::fs::create_dir_all(&mods_dir).await.path(&mods_dir)?;

    // Minecraft 13w24a release date (1.6.1 snapshot)
    // Switched from Texture Packs to Resource Packs
    let v1_6_1 = DateTime::parse_from_rfc3339("2013-06-13T15:32:23+00:00").unwrap();
    let resource_packs = match DateTime::parse_from_rfc3339(&version_json.releaseTime) {
        Ok(dt) => {
            if dt >= v1_6_1 {
                "resourcepacks"
            } else {
                "texturepacks"
            }
        }
        Err(e) => {
            err!("Could not parse instance date/time: {e}");
            "resourcepacks"
        }
    };

    let resource_packs_dir = dot_minecraft_dir.join(resource_packs);
    tokio::fs::create_dir_all(&resource_packs_dir)
        .await
        .path(&resource_packs_dir)?;

    let shader_packs_dir = dot_minecraft_dir.join("shaderpacks");
    tokio::fs::create_dir_all(&shader_packs_dir)
        .await
        .path(&shader_packs_dir)?;

    Ok((mods_dir, resource_packs_dir, shader_packs_dir))
}

#[must_use]
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct CurseforgeNotAllowed {
    pub name: String,
    pub slug: String,
    pub filename: String,
    pub project_type: String,
    pub file_id: usize,
}
