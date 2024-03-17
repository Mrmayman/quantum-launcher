use serde::{Deserialize, Serialize};
use serde_json::Value;

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug)]
pub struct VersionDetails {
    /// The list of command line arguments.
    ///
    /// Used in new Minecraft versions, compared to minecraftArguments used in old versions.
    pub arguments: Option<Arguments>,
    /// An index/list of assets (music/sounds) to be downloaded.
    pub assetIndex: AssetIndex,
    /// Which version of the assets to be downloaded.
    pub assets: String,
    pub complianceLevel: usize,
    /// Where to download the client/server jar.
    pub downloads: Downloads,
    /// Name of the version.
    pub id: String,
    /// Version of java required.
    pub javaVersion: JavaVersion,
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

#[derive(Serialize, Deserialize, Debug)]
pub struct Arguments {
    pub game: Vec<Value>,
    pub jvm: Vec<Value>,
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug)]
pub struct AssetIndex {
    pub id: String,
    pub sha1: String,
    pub size: usize,
    pub totalSize: usize,
    pub url: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Downloads {
    pub client: Download,
    pub client_mappings: Download,
    pub server: Download,
    pub server_mappings: Download,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Download {
    pub sha1: String,
    pub size: usize,
    pub url: String,
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug)]
pub struct JavaVersion {
    pub component: String,
    pub majorVersion: usize,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Library {
    pub downloads: LibraryDownloads,
    pub name: Option<String>,
    pub rules: Option<Vec<LibraryRule>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LibraryDownloads {
    pub artifact: LibraryDownloadArtifact,
    pub name: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LibraryRule {
    pub action: String,
    pub os: LibraryRuleOS,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LibraryRuleOS {
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LibraryDownloadArtifact {
    pub path: String,
    pub sha1: String,
    pub size: usize,
    pub url: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Logging {
    pub client: LoggingClient,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LoggingClient {
    pub argument: String,
    pub file: LoggingClientFile,
    pub r#type: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LoggingClientFile {
    pub id: String,
    pub sha1: String,
    pub size: usize,
    pub url: String,
}
