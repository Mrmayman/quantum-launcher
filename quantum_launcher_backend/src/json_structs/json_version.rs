use serde::{Deserialize, Serialize};
use serde_json::Value;

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize)]
pub struct VersionDetails {
    pub arguments: Arguments,
    pub assetIndex: AssetIndex,
    pub assets: String,
    pub complianceLevel: usize,
    pub downloads: Downloads,
    pub id: String,
    pub javaVersion: JavaVersion,
    pub libraries: Vec<Library>,
    pub logging: Logging,
    pub mainClass: String,
    pub minimumLauncherVersion: usize,
    pub releaseTime: String,
    pub time: String,
    pub r#type: String,
}

#[derive(Serialize, Deserialize)]
pub struct Arguments {
    pub game: Vec<Value>,
    pub jvm: Vec<Value>,
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize)]
pub struct AssetIndex {
    pub id: String,
    pub sha1: String,
    pub size: usize,
    pub totalSize: usize,
    pub url: String,
}

#[derive(Serialize, Deserialize)]
pub struct Downloads {
    pub client: Download,
    pub client_mappings: Download,
    pub server: Download,
    pub server_mappings: Download,
}

#[derive(Serialize, Deserialize)]
pub struct Download {
    pub sha1: String,
    pub size: usize,
    pub url: String,
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize)]
pub struct JavaVersion {
    pub component: String,
    pub majorVersion: usize,
}

#[derive(Serialize, Deserialize)]
pub struct Library {
    pub downloads: LibraryDownloads,
    pub rules: Option<Vec<LibraryRule>>,
}

#[derive(Serialize, Deserialize)]
pub struct LibraryDownloads {
    pub artifact: LibraryDownloadArtifact,
    pub name: String,
}

#[derive(Serialize, Deserialize)]
pub struct LibraryRule {
    pub action: String,
    pub os: LibraryRuleOS,
}

#[derive(Serialize, Deserialize)]
pub struct LibraryRuleOS {
    pub name: String,
}

#[derive(Serialize, Deserialize)]
pub struct LibraryDownloadArtifact {
    pub path: String,
    pub sha1: String,
    pub size: usize,
    pub url: String,
}

#[derive(Serialize, Deserialize)]
pub struct Logging {
    pub client: LoggingClient,
}

#[derive(Serialize, Deserialize)]
pub struct LoggingClient {
    pub argument: String,
    pub file: LoggingClientFile,
    pub r#type: String,
}

#[derive(Serialize, Deserialize)]
pub struct LoggingClientFile {
    pub id: String,
    pub sha1: String,
    pub size: usize,
    pub url: String,
}
