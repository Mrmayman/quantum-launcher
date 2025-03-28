use std::{sync::mpsc::Sender, time::Instant};

use chrono::DateTime;
use ql_core::{
    err, file_utils, json::InstanceConfigJson, GenericProgress, InstanceSelection, Loader, ModId,
    StoreBackendType,
};

mod curseforge;
mod delete;
mod error;
mod image;
mod local_json;
mod modrinth;
mod recommended;
mod toggle;
mod update;

pub use curseforge::CurseforgeBackend;
pub use delete::delete_mods;
pub use error::ModError;
pub use image::{download_image, ImageResult};
pub use local_json::{ModConfig, ModFile, ModIndex};
pub use modrinth::ModrinthBackend;
pub use recommended::{RecommendedMod, RECOMMENDED_MODS};
pub use toggle::toggle_mods;
pub use update::{apply_updates, check_for_updates};

pub const SOURCE_ID_MODRINTH: &str = "modrinth";
pub const SOURCE_ID_CURSEFORGE: &str = "curseforge";

#[allow(async_fn_in_trait)]
pub trait Backend {
    async fn search(query: Query) -> Result<(SearchResult, Instant), ModError>;
    async fn get_description(id: &str) -> Result<ModInformation, ModError>;
    async fn get_latest_version_date(
        id: &str,
        version: &str,
        loader: Option<Loader>,
    ) -> Result<(DateTime<chrono::FixedOffset>, String), ModError>;

    async fn download(id: &str, instance: &InstanceSelection) -> Result<(), ModError>;
    async fn download_bulk(
        ids: &[String],
        instance: &InstanceSelection,
        ignore_incompatible: bool,
        set_manually_installed: bool,
        sender: Option<&Sender<GenericProgress>>,
    ) -> Result<(), ModError>;
}

pub async fn download_mod(id: &ModId, instance: &InstanceSelection) -> Result<(), ModError> {
    match id {
        ModId::Modrinth(n) => ModrinthBackend::download(n, instance).await,
        ModId::Curseforge(n) => CurseforgeBackend::download(n, instance).await,
    }
}

pub async fn download_mods_bulk(
    ids: Vec<ModId>,
    instance_name: InstanceSelection,
    sender: Option<Sender<GenericProgress>>,
) -> Result<(), ModError> {
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

    ModrinthBackend::download_bulk(&modrinth, &instance_name, true, true, sender.as_ref()).await?;

    CurseforgeBackend::download_bulk(&curseforge, &instance_name, true, true, sender.as_ref())
        .await?;

    Ok(())
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

#[derive(Clone, Debug)]
pub struct Query {
    pub name: String,
    pub version: String,
    pub loader: Loader,
    pub server_side: bool,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub mods: Vec<SearchMod>,
    pub backend: StoreBackendType,
}

#[derive(Debug, Clone)]
pub struct SearchMod {
    pub title: String,
    pub description: String,
    pub downloads: usize,
    pub internal_name: String,
    pub id: String,
    pub icon_url: String,
}

#[derive(Debug, Clone)]
pub struct ModInformation {
    pub title: String,
    pub description: String,
    pub icon_url: Option<String>,
    pub id: ModId,
    pub long_description: String,
}

async fn get_loader(instance: &InstanceSelection) -> Result<Option<Loader>, ModError> {
    let instance_dir = file_utils::get_instance_dir(instance).await?;
    let config_json = InstanceConfigJson::read_from_path(&instance_dir).await?;

    Ok(match config_json.mod_type.as_str() {
        "Fabric" => Some(Loader::Fabric),
        "Forge" => Some(Loader::Forge),
        "Quilt" => Some(Loader::Quilt),
        "NeoForge" => Some(Loader::Neoforge),
        "LiteLoader" => Some(Loader::Liteloader),
        "Rift" => Some(Loader::Rift),
        "OptiFine" => Some(Loader::OptiFine),
        _ => {
            err!("Unknown loader {}", config_json.mod_type);
            None
        } // TODO: Add more loaders
    })
}
