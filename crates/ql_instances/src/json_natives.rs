use ql_core::json::version::{LibraryDownloadArtifact, LibraryRule};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct JsonNatives {
    // pub name: String,
    // pub uid: String,
    pub version: String,
    pub libraries: Vec<NativeLibrary>,
}

#[derive(Clone, Copy)]
pub enum NativesEntry {
    Lwjgl,
    Log4J,
    Oshi,
    JavaObjCBridge,
    Slf4J,
}

impl NativesEntry {
    pub fn get_file(&self) -> &'static str {
        match self {
            NativesEntry::Lwjgl => include_str!("../../../assets/minecraft_arm/org.lwjgl3.json"),
            NativesEntry::Log4J => {
                include_str!("../../../assets/minecraft_arm/org.apache.logging.log4j.json")
            }
            NativesEntry::Oshi => {
                include_str!("../../../assets/minecraft_arm/com.github.oshi.json")
            }
            NativesEntry::JavaObjCBridge => {
                include_str!("../../../assets/minecraft_arm/ca.weblite.json")
            }
            NativesEntry::Slf4J => include_str!("../../../assets/minecraft_arm/org.slf4j.json"),
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
    pub fn get(entry: NativesEntry) -> Result<Self, serde_json::Error> {
        let json = entry.get_file();
        let json: Self = serde_json::from_str(json)?;
        Ok(json)
    }
}

#[derive(Deserialize)]
pub struct NativeLibrary {
    // pub name: String,
    pub downloads: NativeDownloads,
    pub rules: Option<Vec<LibraryRule>>,
}

#[derive(Deserialize)]
pub struct NativeDownloads {
    pub artifact: LibraryDownloadArtifact,
}
