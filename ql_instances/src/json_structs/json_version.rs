use std::{collections::BTreeMap, fmt::Debug};

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VersionDetails {
    /// The list of command line arguments.
    ///
    /// Used in new Minecraft versions, compared to minecraftArguments used in old versions.
    pub arguments: Option<Arguments>,
    /// An index/list of assets (music/sounds) to be downloaded.
    pub assetIndex: AssetIndex,
    /// Which version of the assets to be downloaded.
    pub assets: String,
    pub complianceLevel: Option<usize>,
    /// Where to download the client/server jar.
    pub downloads: Downloads,
    /// Name of the version.
    pub id: String,
    /// Version of java required.
    pub javaVersion: Option<JavaVersion>,
    /// Library dependencies of the version that need to be downloaded.
    pub libraries: Vec<Library>,
    /// Details regarding console logging with log4j.
    pub logging: Option<Logging>,
    /// Which is the main class in the jar that has the main function.
    pub mainClass: String,
    /// The list of command line arguments.
    ///
    /// Used in old Minecraft versions, compared to arguments used in new versions.
    pub minecraftArguments: Option<String>,
    /// Minimum version of the official launcher that is supported. Not applicable here.
    pub minimumLauncherVersion: usize,
    /// When was this version released. Idk the difference between time and releaseTime.
    pub releaseTime: String,
    /// When was this version released. Idk the difference between time and releaseTime.
    pub time: String,
    /// Type of version, such as alpha, beta or release.
    pub r#type: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Arguments {
    pub game: Vec<Value>,
    pub jvm: Vec<Value>,
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AssetIndex {
    pub id: String,
    pub sha1: String,
    pub size: usize,
    pub totalSize: usize,
    pub url: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Downloads {
    pub client: Download,
    pub client_mappings: Option<Download>,
    pub server: Option<Download>,
    pub server_mappings: Option<Download>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Download {
    pub sha1: String,
    pub size: usize,
    pub url: String,
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JavaVersion {
    pub component: String,
    pub majorVersion: usize,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Library {
    pub downloads: Option<LibraryDownloads>,
    pub extract: Option<LibraryExtract>,
    pub name: Option<String>,
    pub rules: Option<Vec<LibraryRule>>,
    pub natives: Option<BTreeMap<String, String>>,
    // Fabric:
    pub sha1: Option<String>,
    pub sha256: Option<String>,
    // name: Option<String>
    pub size: Option<usize>,
    pub sha512: Option<String>,
    pub md5: Option<String>,
    pub url: Option<String>,
}

impl Debug for Library {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = f.debug_struct("Library");
        let mut s_ref = &mut s;
        if let Some(downloads) = &self.downloads {
            s_ref = s_ref.field("downloads", &downloads);
        }
        if let Some(extract) = &self.extract {
            s_ref = s_ref.field("extract", &extract);
        }
        if let Some(name) = &self.name {
            s_ref = s_ref.field("name", &name);
        }
        if let Some(rules) = &self.rules {
            s_ref = s_ref.field("rules", &rules);
        }
        if let Some(natives) = &self.natives {
            s_ref = s_ref.field("natives", &natives);
        }
        if let Some(sha1) = &self.sha1 {
            s_ref = s_ref.field("sha1", &sha1);
        }
        if let Some(sha256) = &self.sha256 {
            s_ref = s_ref.field("sha256", &sha256);
        }
        if let Some(size) = &self.size {
            s_ref = s_ref.field("size", &size);
        }
        if let Some(sha512) = &self.sha512 {
            s_ref = s_ref.field("sha512", &sha512);
        }
        if let Some(md5) = &self.md5 {
            s_ref = s_ref.field("md5", &md5);
        }
        if let Some(url) = &self.url {
            s_ref = s_ref.field("url", &url);
        }
        s_ref.finish()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LibraryExtract {
    pub exclude: Vec<String>,
    pub name: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum LibraryDownloads {
    Normal {
        artifact: LibraryDownloadArtifact,
        name: Option<String>,
    },
    Native {
        classifiers: BTreeMap<String, LibraryClassifier>,
    },
}

impl Debug for LibraryDownloads {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LibraryDownloads::Normal {
                artifact,
                name: None,
            } => {
                write!(f, "Normal: {artifact:#?}")
            }
            LibraryDownloads::Normal {
                artifact,
                name: Some(name),
            } => write!(f, "Normal ({name}): {artifact:?}"),
            LibraryDownloads::Native { classifiers } => {
                write!(f, "Native: {classifiers:?}")
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LibraryClassifier {
    pub path: String,
    pub sha1: String,
    pub size: usize,
    pub url: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct LibraryRule {
    pub action: String,
    pub os: Option<LibraryRuleOS>,
}

impl Debug for LibraryRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(os) = &self.os {
            write!(f, "LibraryRule: {} for {os:?}", self.action)
        } else {
            write!(f, "LibraryRule: {}", self.action)
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct LibraryRuleOS {
    pub name: String,
    pub version: Option<String>, // Regex
}

impl Debug for LibraryRuleOS {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(version) = &self.version {
            write!(f, "{} {version}", self.name)
        } else {
            write!(f, "{}", self.name)
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LibraryDownloadArtifact {
    pub path: String,
    pub sha1: String,
    pub size: usize,
    pub url: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Logging {
    pub client: LoggingClient,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LoggingClient {
    pub argument: String,
    pub file: LoggingClientFile,
    pub r#type: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LoggingClientFile {
    pub id: String,
    pub sha1: String,
    pub size: usize,
    pub url: String,
}
