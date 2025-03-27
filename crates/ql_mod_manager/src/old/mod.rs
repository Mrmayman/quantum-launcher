mod delete;
mod local_json;
mod modrinth;
mod search;
mod toggle;
mod update;

use std::{sync::mpsc::Sender, time::Instant};

pub use delete::delete_mods;
pub use local_json::{ModConfig, ModIndex};
use modrinth::ModVersion;
pub use modrinth::{ModDownloader, ModrinthBackend};
use ql_core::{
    info, pt, GenericProgress, InstanceSelection, IoError, JsonFileError, Loader, RequestError,
};
pub use search::{download_image, ImageResult};
use serde::{Deserialize, Serialize};
use thiserror::Error;
pub use toggle::toggle_mods;
pub use update::{apply_updates, check_for_updates};

pub async fn download_mod(id: &ModId, instance: &InstanceSelection) -> Result<(), ModError> {
    match id {
        ModId::Modrinth(id) => ModrinthBackend.download(id, instance).await,
        ModId::Curseforge(_) => todo!(),
    }
}

use zip_extract::ZipError;

pub trait StoreBackend {
    async fn search(&self, query: SearchQuery) -> Result<(SearchResult, Instant), ModError>;
    async fn download(&self, id: &str, instance: &InstanceSelection) -> Result<(), ModError>;
}

#[derive(Debug, Clone, Copy)]
pub enum StoreBackendType {
    Modrinth,
    Curseforge,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum ModId {
    Modrinth(String),
    Curseforge(String),
}

impl ModId {
    pub fn get_internal_id(&self) -> &str {
        match self {
            ModId::Modrinth(n) | ModId::Curseforge(n) => &n,
        }
    }

    fn get_index_str(&self) -> String {
        match self {
            ModId::Modrinth(n) => n.clone(),
            ModId::Curseforge(n) => format!("CF:{n}"),
        }
    }

    fn from_index_str(n: &str) -> Self {
        if n.starts_with("CF:") {
            ModId::Curseforge(n.strip_prefix("CF:").unwrap_or(n).to_owned())
        } else {
            ModId::Modrinth(n.to_owned())
        }
    }
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
    pub id: ModId,
    pub icon_url: String,
}

#[derive(Clone, Debug)]
pub struct SearchQuery {
    pub name: String,
    pub version: String,
    pub loader: Loader,
    pub server_side: bool,
}

#[derive(Clone, Debug)]
pub struct ProjectInformation {
    pub title: String,
    pub description: String,
    pub icon_url: Option<String>,
    pub id: ModId,
    pub long_description: String,
}

#[derive(Debug, Error)]
pub enum ModError {
    #[error(transparent)]
    RequestError(#[from] RequestError),
    #[error("json error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error(transparent)]
    Io(#[from] IoError),
    #[error("no compatible version found for mod")]
    NoCompatibleVersionFound,
    #[error("no files found for mod")]
    NoFilesFound,
    #[error("couldn't add entry {1} to zip: {0}")]
    ZipIoError(std::io::Error, String),
    #[error("zip error: {0}")]
    Zip(#[from] ZipError),
}

impl From<JsonFileError> for ModError {
    fn from(value: JsonFileError) -> Self {
        match value {
            JsonFileError::SerdeError(err) => err.into(),
            JsonFileError::Io(err) => err.into(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModFile {
    // pub hashes: ModHashes,
    pub url: String,
    pub filename: String,
    pub primary: bool,
    // pub size: usize,
    // pub file_type: Option<String>,
}

// #[derive(Serialize, Deserialize, Debug, Clone)]
// pub struct ModHashes {
//     pub sha512: String,
//     pub sha1: String,
// }

#[derive(Debug, Clone)]
pub struct RecommendedMod {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub enabled_by_default: bool,
}

impl RecommendedMod {
    pub async fn get_compatible_mods(
        ids: Vec<RecommendedMod>,
        version: String,
        loader: Loader,
        sender: Sender<GenericProgress>,
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

            let is_compatible = ModVersion::is_compatible(id.id, &version, &loader).await?;
            pt!("{} : {is_compatible}", id.name);
            if is_compatible {
                mods.push(id);
            }
        }
        Ok(mods)
    }
}

pub const RECOMMENDED_MODS: &[RecommendedMod] = &[
    RecommendedMod {
        id: "AANobbMI",
        name: "Sodium",
        description: "Optimizes the rendering engine",
        enabled_by_default: true,
    },
    RecommendedMod {
        id: "gvQqBUqZ",
        name: "Lithium",
        description: "Optimizes the integrated server",
        enabled_by_default: true,
    },
    RecommendedMod {
        id: "mOgUt4GM",
        name: "Mod Menu",
        description: "A mod menu for managing mods",
        enabled_by_default: true,
    },
    RecommendedMod {
        id: "NNAgCjsB",
        name: "Entity Culling",
        description: "Optimizes entity rendering",
        enabled_by_default: true,
    },
    RecommendedMod {
        id: "5ZwdcRci",
        name: "ImmediatelyFast",
        description: "Optimizes immediate mode rendering",
        enabled_by_default: true,
    },
    RecommendedMod {
        id: "qQyHxfxd",
        name: "No Chat Reports",
        description: "Disables chat reporting",
        enabled_by_default: true,
    },
    RecommendedMod {
        id: "kzwxhsjp",
        name: "Accurate Block Placement Reborn",
        description: "Makes placing blocks more accurate",
        enabled_by_default: true,
    },
    RecommendedMod {
        id: "aC3cM3Vq",
        name: "Mouse Tweaks",
        description: "Improves inventory controls",
        enabled_by_default: true,
    },
    RecommendedMod {
        id: "hvFnDODi",
        name: "LazyDFU",
        description: "Speeds up Minecraft start time",
        enabled_by_default: true,
    },
    RecommendedMod {
        id: "YL57xq9U",
        name: "Iris Shaders",
        description: "Adds Shaders to Minecraft",
        enabled_by_default: false,
    },
    RecommendedMod {
        id: "1IjD5062",
        name: "Continuity",
        description: "Adds connected textures",
        enabled_by_default: false,
    },
    RecommendedMod {
        id: "yBW8D80W",
        name: "LambDynamicLights",
        description: "Adds dynamic lights",
        enabled_by_default: false,
    },
    RecommendedMod {
        id: "bXX9h73M",
        name: "MidnightControls",
        description: "Adds controller (and touch) support",
        enabled_by_default: false,
    },
    RecommendedMod {
        id: "8shC1gFX",
        name: "BetterF3",
        description: "Cleans up the debug (F3) screen",
        enabled_by_default: false,
    },
    RecommendedMod {
        id: "EsAfCjCV",
        name: "AppleSkin",
        description: "Shows hunger and saturation values",
        enabled_by_default: false,
    },
    RecommendedMod {
        id: "1bokaNcj",
        name: "Xaero's Minimap",
        description: "Adds a minimap to the game",
        enabled_by_default: false,
    },
    RecommendedMod {
        id: "NcUtCpym",
        name: "Xaero's World Map",
        description: "Adds a world map to the game",
        enabled_by_default: false,
    },
];
