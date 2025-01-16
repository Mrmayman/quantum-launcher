mod delete;
mod download;
mod get_project;
mod local_json;
mod search;
mod toggle;
mod update;
mod versions;

pub use delete::{delete_mods, delete_mods_w};
pub use download::{download_mod, download_mod_w, download_mods_w};
pub use get_project::{DonationLink, GalleryItem, License, ProjectInfo};
pub use local_json::{ModConfig, ModIndex};
pub use search::{Entry, ImageResult, Loader, ModError, Query, Search};
pub use toggle::toggle_mods_w;
pub use update::{apply_updates_w, check_for_updates};
pub use versions::{ModFile, ModHashes, ModVersion};

#[derive(Debug, Clone)]
pub struct RecommendedMod {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub enabled_by_default: bool,
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
        id: "kzwxhsjp",
        name: "Accurate Block Placement Reborn",
        description: "Makes placing blocks more accurate",
        enabled_by_default: false,
    },
    RecommendedMod {
        id: "aC3cM3Vq",
        name: "Mouse Tweaks",
        description: "Improves inventory controls",
        enabled_by_default: false,
    },
    RecommendedMod {
        id: "8shC1gFX",
        name: "BetterF3",
        description: "Cleans up the debug screen",
        enabled_by_default: false,
    },
    RecommendedMod {
        id: "EsAfCjCV",
        name: "AppleSkin",
        description: "Shows food saturation values",
        enabled_by_default: false,
    },
    RecommendedMod {
        id: "1bokaNcj",
        name: "Xaero's Minimap",
        description: "Adds a minimap to the game",
        enabled_by_default: false,
    },
];
