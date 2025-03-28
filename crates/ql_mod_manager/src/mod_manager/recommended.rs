use std::sync::mpsc::Sender;

use ql_core::{info, pt, GenericProgress, Loader, ModId, StoreBackendType};

use crate::mod_manager::get_latest_version_date;

use super::ModError;

#[derive(Debug, Clone)]
pub struct RecommendedMod {
    pub id: &'static str,
    pub name: &'static str,
    pub backend: StoreBackendType,
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

            let mod_id = ModId::from_pair(id.id, id.backend);
            let is_compatible = get_latest_version_date(Some(loader), &mod_id, &version)
                .await
                .is_ok();
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
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "gvQqBUqZ",
        name: "Lithium",
        description: "Optimizes the integrated server",
        enabled_by_default: true,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "mOgUt4GM",
        name: "Mod Menu",
        description: "A mod menu for managing mods",
        enabled_by_default: true,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "NNAgCjsB",
        name: "Entity Culling",
        description: "Optimizes entity rendering",
        enabled_by_default: true,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "5ZwdcRci",
        name: "ImmediatelyFast",
        description: "Optimizes immediate mode rendering",
        enabled_by_default: true,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "qQyHxfxd",
        name: "No Chat Reports",
        description: "Disables chat reporting",
        enabled_by_default: true,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "kzwxhsjp",
        name: "Accurate Block Placement Reborn",
        description: "Makes placing blocks more accurate",
        enabled_by_default: true,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "aC3cM3Vq",
        name: "Mouse Tweaks",
        description: "Improves inventory controls",
        enabled_by_default: true,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "hvFnDODi",
        name: "LazyDFU",
        description: "Speeds up Minecraft start time",
        enabled_by_default: true,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "YL57xq9U",
        name: "Iris Shaders",
        description: "Adds Shaders to Minecraft",
        enabled_by_default: false,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "1IjD5062",
        name: "Continuity",
        description: "Adds connected textures",
        enabled_by_default: false,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "yBW8D80W",
        name: "LambDynamicLights",
        description: "Adds dynamic lights",
        enabled_by_default: false,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "bXX9h73M",
        name: "MidnightControls",
        description: "Adds controller (and touch) support",
        enabled_by_default: false,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "8shC1gFX",
        name: "BetterF3",
        description: "Cleans up the debug (F3) screen",
        enabled_by_default: false,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "EsAfCjCV",
        name: "AppleSkin",
        description: "Shows hunger and saturation values",
        enabled_by_default: false,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "1bokaNcj",
        name: "Xaero's Minimap",
        description: "Adds a minimap to the game",
        enabled_by_default: false,
        backend: StoreBackendType::Modrinth,
    },
    RecommendedMod {
        id: "NcUtCpym",
        name: "Xaero's World Map",
        description: "Adds a world map to the game",
        enabled_by_default: false,
        backend: StoreBackendType::Modrinth,
    },
];
