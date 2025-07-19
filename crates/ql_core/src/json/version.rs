use std::{collections::BTreeMap, fmt::Debug, path::Path};

use chrono::DateTime;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{err, pt, InstanceSelection, IntoIoError, IntoJsonError, JsonFileError};

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VersionDetails {
    /// An index/list of assets (music/sounds) to be downloaded.
    pub assetIndex: AssetIndex,
    /// Which version of the assets to be downloaded.
    pub assets: String,
    /// Where to download the client/server jar.
    pub downloads: Downloads,
    /// Name of the version.
    pub id: String,
    /// Version of java required.
    pub javaVersion: Option<JavaVersionJson>,
    /// Library dependencies of the version that need to be downloaded.
    pub libraries: Vec<Library>,
    /// Details regarding console logging with log4j.
    pub logging: Option<Logging>,
    /// Which is the main class in the jar that has the main function.
    pub mainClass: String,

    /// The list of command line arguments.
    ///
    /// This one is used in Minecraft 1.12.2 and below,
    /// whereas `arguments` is used in 1.13 and above
    pub minecraftArguments: Option<String>,
    /// The list of command line arguments.
    ///
    /// This is used in Minecraft 1.13 and above,
    /// whereas `minecraftArguments` is used in 1.12.2 and below.
    pub arguments: Option<Arguments>,

    /// Minimum version of the official launcher that is supported.
    ///
    /// Unused field.
    pub minimumLauncherVersion: Option<usize>,

    // TODO: Find difference between `releaseTime` and `time`
    pub releaseTime: String,
    pub time: String,

    /// Type of version, such as alpha, beta or release.
    pub r#type: String,

    /// Quantum Launcher-specific field added here
    /// to cache the [`VersionDetails::is_legacy_version`] calculation.
    pub ql_is_legacy_version: Option<bool>,
}

impl VersionDetails {
    /// Loads a Minecraft instance JSON from disk,
    /// based on a specific `InstanceSelection`
    ///
    /// # Errors
    /// - `details.json` file couldn't be loaded
    /// - `details.json` couldn't be parsed into valid JSON
    pub async fn load(instance: &InstanceSelection) -> Result<Self, JsonFileError> {
        Self::load_from_path(&instance.get_instance_path()).await
    }

    /// Loads a Minecraft instance JSON from disk,
    /// based on a path to the root of the instance directory.
    ///
    /// This is the `async` function, for the sync function
    /// see [`VersionDetails::load_s`]
    ///
    /// # Errors
    /// - `dir`/`details.json` doesn't exist or isn't a file
    /// - `details.json` file couldn't be loaded
    /// - `details.json` couldn't be parsed into valid JSON
    pub async fn load_from_path(path: &Path) -> Result<Self, JsonFileError> {
        let path = path.join("details.json");
        let file = tokio::fs::read_to_string(&path).await.path(path)?;
        let version_json: VersionDetails = serde_json::from_str(&file).json(file)?;

        Ok(version_json)
    }

    /// Loads a Minecraft instance JSON from disk,
    /// based on a path to the root of the instance directory.
    ///
    /// This is the sync function, for the `async` function
    /// see [`VersionDetails::load_from_path`]
    ///
    /// # Errors
    /// - `dir`/`details.json` doesn't exist or isn't a file
    /// - `details.json` file couldn't be loaded
    /// - `details.json` couldn't be parsed into valid JSON
    #[must_use]
    pub fn load_s(instance_dir: &Path) -> Option<Self> {
        let path = instance_dir.join("details.json");

        let file = match std::fs::read_to_string(&path) {
            Ok(n) => n,
            Err(err) => {
                err!("Couldn't read details.json: {err}");
                return None;
            }
        };

        let details: VersionDetails = match serde_json::from_str(&file) {
            Ok(n) => n,
            Err(err) => {
                err!("Couldn't parse details.json: {err}");
                return None;
            }
        };

        Some(details)
    }

    pub async fn apply_tweaks(
        &mut self,
        instance: &InstanceSelection,
    ) -> Result<(), JsonFileError> {
        let patches_path = instance.get_instance_path().join("patches");
        if !patches_path.is_dir() {
            return Ok(());
        }

        let mut dir = tokio::fs::read_dir(&patches_path)
            .await
            .path(patches_path)?;

        while let Ok(Some(dir)) = dir.next_entry().await {
            let path = dir.path();
            if !path.is_file() {
                continue;
            }
            let name = path.file_name().unwrap_or(path.as_os_str());
            pt!("JSON: applying patch: {name:?}");

            let data = tokio::fs::read_to_string(&path).await.path(&path)?;
            let json: VersionDetailsPatch = match serde_json::from_str(&data) {
                Ok(n) => n,
                Err(err) => {
                    err!("Couldn't parse VersionDetails patch: {name:?}, skipping...\n{err}");
                    continue;
                }
            };

            self.patch(json);
        }

        Ok(())
    }

    fn patch(&mut self, json: VersionDetailsPatch) {
        if let Some(args) = json.minecraftArguments {
            self.minecraftArguments = Some(args);
        }
        if let Some(libraries) = json.libraries {
            self.libraries.extend(libraries);
        }
        // TODO: More fields in the future
    }

    #[allow(clippy::missing_panics_doc)]
    pub fn is_legacy_version(&mut self) -> bool {
        if let Some(n) = self.ql_is_legacy_version {
            n
        } else {
            let v1_5_2 = DateTime::parse_from_rfc3339("2013-04-25T15:45:00+00:00").unwrap();
            let res = match DateTime::parse_from_rfc3339(&self.releaseTime) {
                Ok(dt) => dt <= v1_5_2,
                Err(e) => {
                    err!("Could not parse instance date/time: {e}");
                    false
                }
            };
            self.ql_is_legacy_version = Some(res);
            res
        }
    }

    #[must_use]
    pub fn needs_launchwrapper_fix(&self) -> bool {
        self.libraries
            .iter()
            .filter_map(|n| n.downloads.as_ref())
            .filter_map(|n| n.artifact.as_ref())
            .any(|n| {
                n.path
                    .as_ref()
                    .is_some_and(|n| n.contains("mcphackers/launchwrapper/1.1.2"))
            })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[allow(non_snake_case)]
pub struct VersionDetailsPatch {
    pub libraries: Option<Vec<Library>>,
    pub minecraftArguments: Option<String>,
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
    // pub client_mappings: Option<Download>,
    pub server: Option<Download>,
    // pub server_mappings: Option<Download>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Download {
    pub sha1: String,
    pub size: usize,
    pub url: String,
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JavaVersionJson {
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
    // pub sha1: Option<String>,
    // pub sha256: Option<String>,
    // pub size: Option<usize>,
    // pub sha512: Option<String>,
    // pub md5: Option<String>,
    pub url: Option<String>,
}

impl Debug for Library {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = f.debug_struct(&if let Some(name) = &self.name {
            format!("Library ({name})")
        } else {
            "Library".to_owned()
        });
        let mut s = &mut s;
        if let Some(downloads) = &self.downloads {
            s = s.field("downloads", &downloads);
        }
        if let Some(extract) = &self.extract {
            s = s.field("extract", &extract);
        }
        if let Some(rules) = &self.rules {
            s = s.field("rules", &rules);
        }
        if let Some(natives) = &self.natives {
            s = s.field("natives", &natives);
        }
        if let Some(url) = &self.url {
            s = s.field("url", &url);
        }
        s.finish()
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct LibraryExtract {
    pub exclude: Vec<String>,
    pub name: Option<String>,
}

impl Debug for LibraryExtract {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(name) = &self.name {
            write!(f, "({name}), exclude: {:?}", self.exclude)
        } else {
            write!(f, "exclude: {:?}", self.exclude)
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct LibraryDownloads {
    pub artifact: Option<LibraryDownloadArtifact>,
    // pub name: Option<String>,
    pub classifiers: Option<BTreeMap<String, LibraryClassifier>>,
}

impl Debug for LibraryDownloads {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (&self.artifact, &self.classifiers) {
            (None, None) => write!(f, "LibraryDownloads: None {{}}"),
            (None, Some(classifiers)) => {
                if f.alternate() {
                    write!(f, "classifiers: {classifiers:#?}")
                } else {
                    write!(f, "classifiers: {classifiers:?}")
                }
            }
            (Some(artifact), None) => {
                if f.alternate() {
                    write!(f, "artifact: {artifact:#?}")
                } else {
                    write!(f, "artifact: {artifact:?}")
                }
            }
            (Some(artifact), Some(classifiers)) => f
                .debug_struct("LibraryDownloads")
                .field("artifact", artifact)
                .field("classifiers", classifiers)
                .finish(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LibraryClassifier {
    // pub path: Option<String>,
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
            write!(f, "Rule: {} for {os:?}", self.action)
        } else {
            write!(f, "Rule: {}", self.action)
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct LibraryRuleOS {
    pub name: String,
    // pub version: Option<String>, // Regex for OS version. TODO: Use this
}

impl Debug for LibraryRuleOS {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LibraryDownloadArtifact {
    pub path: Option<String>,
    pub sha1: String,
    pub size: usize,
    pub url: String,
}

impl LibraryDownloadArtifact {
    #[must_use]
    pub fn get_path(&self) -> String {
        self.path.clone().unwrap_or_else(|| {
            // https://libraries.minecraft.net/net/java/jinput/jinput/2.0.5/jinput-2.0.5.jar
            // -> libraries.minecraft.net/net/java/jinput/jinput/2.0.5/jinput-2.0.5.jar
            let url = self
                .url
                .strip_prefix("https://")
                .or(self.url.strip_prefix("http://"))
                .unwrap_or(&self.url);

            // libraries.minecraft.net/net/java/jinput/jinput/2.0.5/jinput-2.0.5.jar
            // -> net/java/jinput/jinput/2.0.5/jinput-2.0.5.jar
            if let Some(pos) = url.find('/') {
                url[pos + 1..].to_string()
            } else {
                url.to_string()
            }
        })
    }
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
