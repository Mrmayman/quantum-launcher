use ql_core::{
    file_utils,
    json::version::{LibraryDownloadArtifact, LibraryRule},
    JsonDownloadError,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct JsonNatives {
    pub name: String,
    pub uid: String,
    pub version: String,
    pub libraries: Vec<NativeLibrary>,
}

pub enum NativesEntry {
    Lwjgl,
    Log4J,
    Oshi,
    JavaObjCBridge,
    Slf4J,
}

impl NativesEntry {
    pub fn get_url(&self) -> &'static str {
        match self {
            NativesEntry::Lwjgl => "https://raw.githubusercontent.com/Kichura/Minecraft_ARM/refs/heads/Canary/patches/org.lwjgl3.json",
            NativesEntry::Log4J => "https://raw.githubusercontent.com/Kichura/Minecraft_ARM/refs/heads/Canary/patches/org.apache.logging.log4j.json",
            NativesEntry::Oshi => "https://raw.githubusercontent.com/Kichura/Minecraft_ARM/refs/heads/Canary/patches/com.github.oshi.json",
            NativesEntry::JavaObjCBridge => "https://raw.githubusercontent.com/Kichura/Minecraft_ARM/refs/heads/Canary/patches/ca.weblite.json",
            NativesEntry::Slf4J => "https://raw.githubusercontent.com/Kichura/Minecraft_ARM/refs/heads/Canary/patches/org.slf4j.json",
        }
    }

    pub fn get(name: &str) -> Option<Self> {
        if name.starts_with("org.lwjgl") {
            Some(NativesEntry::Lwjgl)
        } else if name.starts_with("org.apache.logging.log4j") {
            Some(NativesEntry::Log4J)
        } else if name.starts_with("com.github.oshi") {
            Some(NativesEntry::Oshi)
        } else if name.starts_with("ca.weblite") {
            Some(NativesEntry::JavaObjCBridge)
        } else if name.starts_with("org.slf4j") {
            Some(NativesEntry::Slf4J)
        } else {
            None
        }
    }
}

impl JsonNatives {
    pub async fn download(entry: NativesEntry) -> Result<Self, JsonDownloadError> {
        let url = entry.get_url();
        let client = reqwest::Client::new();
        let json = file_utils::download_file_to_string(&client, url, false).await?;
        let json: Self = serde_json::from_str(&json)?;
        Ok(json)
    }
}

#[derive(Serialize, Deserialize)]
pub struct NativeLibrary {
    pub name: String,
    pub downloads: NativeDownloads,
    pub rules: Option<Vec<LibraryRule>>,
}

#[derive(Serialize, Deserialize)]
pub struct NativeDownloads {
    pub artifact: LibraryDownloadArtifact,
}
