use std::fmt::Display;

use crate::MinecraftVersionCategory;

/// An enum representing a Minecraft version.
///
/// # Variants
/// - `Normal` - A Minecraft version officially
///   from Mojang. Release versions (1.0+).
/// - `Omniarchive` - A Minecraft version from
///   Omniarchive (an archive dedicated to old)
///   versions of Minecraft). Includes all old
///   versions of Minecraft (before 1.0).
/// - `OmniarchiveClassicZipServer` - A Minecraft
///   classic server in a Zip file from Omniarchive.
#[derive(Debug, Clone)]
pub enum ListEntry {
    Normal(String),
    Omniarchive {
        category: MinecraftVersionCategory,
        name: String,
        url: String,
    },
    OmniarchiveClassicZipServer {
        name: String,
        url: String,
    },
}

impl Display for ListEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ListEntry::Normal(name)
            | ListEntry::Omniarchive { name, .. }
            | ListEntry::OmniarchiveClassicZipServer { name, .. } => write!(f, "{name}"),
        }
    }
}
