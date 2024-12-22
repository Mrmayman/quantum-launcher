use std::fmt::Display;

use crate::MinecraftVersionCategory;

#[derive(Debug, Clone)]
pub enum ListEntry {
    Normal(String),
    Omniarchive {
        category: MinecraftVersionCategory,
        name: String,
        url: String,
    },
}

impl Display for ListEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ListEntry::Normal(name) | ListEntry::Omniarchive { name, .. } => write!(f, "{name}"),
        }
    }
}
