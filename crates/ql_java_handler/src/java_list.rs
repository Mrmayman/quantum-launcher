use std::fmt::Display;

use crate::file_utils;
use ql_core::json::version::JavaVersionJson;
use serde::Deserialize;

use crate::JsonDownloadError;

#[derive(Clone, Copy, Debug)]
pub enum JavaVersion {
    Java16,
    Java17,
    Java21,
    Java8,
}

impl JavaVersion {
    #[must_use]
    pub(crate) fn get_corretto_url(self) -> &'static str {
        // https://aws.amazon.com/corretto/
        // for more info

        if cfg!(target_arch = "aarch64") && cfg!(target_os = "linux") {
            match self {
                JavaVersion::Java16 | JavaVersion::Java17 => {
                    "https://corretto.aws/downloads/latest/amazon-corretto-17-aarch64-linux-jdk.tar.gz"
                }
                JavaVersion::Java21 => {
                    "https://corretto.aws/downloads/latest/amazon-corretto-21-aarch64-linux-jdk.tar.gz"
                }
                JavaVersion::Java8 => {
                    "https://corretto.aws/downloads/latest/amazon-corretto-8-aarch64-linux-jdk.tar.gz"
                }
            }
        } else if cfg!(target_arch = "aarch64") && cfg!(target_os = "macos") {
            match self {
                JavaVersion::Java16 | JavaVersion::Java17 => {
                    "https://corretto.aws/downloads/latest/amazon-corretto-17-aarch64-macos-jdk.tar.gz"
                }
                JavaVersion::Java21 => {
                    "https://corretto.aws/downloads/latest/amazon-corretto-21-aarch64-macos-jdk.tar.gz"
                }
                JavaVersion::Java8 => {
                    "https://corretto.aws/downloads/latest/amazon-corretto-8-aarch64-macos-jdk.tar.gz"
                }
            }
        } else if cfg!(target_arch = "x86") && cfg!(target_os = "windows") {
            match self {
                JavaVersion::Java16 | JavaVersion::Java17 => {
                    "https://corretto.aws/downloads/latest/amazon-corretto-17-x86-windows-jdk.zip"
                }
                JavaVersion::Java21 => {
                    "https://corretto.aws/downloads/latest/amazon-corretto-21-x86-windows-jdk.zip"
                }
                JavaVersion::Java8 => {
                    "https://corretto.aws/downloads/latest/amazon-corretto-8-x86-windows-jdk.zip"
                }
            }
        } else if cfg!(target_arch = "x86_64") && cfg!(target_os = "windows") {
            match self {
                JavaVersion::Java16 | JavaVersion::Java17 => {
                    "https://corretto.aws/downloads/latest/amazon-corretto-17-x64-windows-jdk.zip"
                }
                JavaVersion::Java21 => {
                    "https://corretto.aws/downloads/latest/amazon-corretto-21-x64-windows-jdk.zip"
                }
                JavaVersion::Java8 => {
                    "https://corretto.aws/downloads/latest/amazon-corretto-8-x64-windows-jdk.zip"
                }
            }
        } else if cfg!(target_arch = "x86_64") && cfg!(target_os = "linux") {
            match self {
                JavaVersion::Java16 | JavaVersion::Java17 => {
                    "https://corretto.aws/downloads/latest/amazon-corretto-17-x64-linux-jdk.zip"
                }
                JavaVersion::Java21 => {
                    "https://corretto.aws/downloads/latest/amazon-corretto-21-x64-linux-jdk.zip"
                }
                JavaVersion::Java8 => {
                    "https://corretto.aws/downloads/latest/amazon-corretto-8-x64-linux-jdk.tar.gz"
                }
            }
        } else {
            panic!("Unsupported OS")
        }
    }
}

impl Display for JavaVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                JavaVersion::Java16 => "java_16",
                JavaVersion::Java17 => "java_17",
                JavaVersion::Java21 => "java_21",
                JavaVersion::Java8 => "java_8",
            }
        )
    }
}

impl From<JavaVersionJson> for JavaVersion {
    fn from(version: JavaVersionJson) -> Self {
        match version.majorVersion {
            8 => JavaVersion::Java8,
            16 => JavaVersion::Java16,
            17 => JavaVersion::Java17,
            _ => JavaVersion::Java21,
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct JavaListJson {
    // gamecore: JavaList,
    linux: JavaList,
    linux_i386: JavaList,
    mac_os: JavaList,
    mac_os_arm64: JavaList,
    windows_arm64: JavaList,
    windows_x86: JavaList,
    windows_x64: JavaList,
}

impl JavaListJson {
    pub async fn download() -> Result<Self, JsonDownloadError> {
        pub const JAVA_LIST_URL: &str = "https://launchermeta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json";
        file_utils::download_file_to_json(JAVA_LIST_URL, false).await
    }

    pub fn get_url(&self, version: JavaVersion) -> Option<String> {
        let java_list = if cfg!(target_os = "linux") {
            if cfg!(target_arch = "x86_64") {
                &self.linux
            } else if cfg!(target_arch = "x86") {
                &self.linux_i386
            } else {
                // TODO: Add ARM32, RISC-V, and PowerPC support.
                return None;
            }
        } else if cfg!(target_os = "macos") {
            // aarch64
            if cfg!(target_arch = "aarch64") {
                &self.mac_os_arm64
            } else if cfg!(target_arch = "x86_64") {
                &self.mac_os
            } else {
                // TODO: Add x86 and PowerPC support.
                return None;
            }
        } else if cfg!(target_os = "windows") {
            if cfg!(target_arch = "x86_64") {
                &self.windows_x64
            } else if cfg!(target_arch = "x86") {
                &self.windows_x86
            } else if cfg!(target_arch = "aarch64") {
                &self.windows_arm64
            } else {
                // What if Windows supports some
                // other architecture in the future?
                return None;
            }
        } else {
            // TODO: Unsupported OS handling.
            // Some people might play this on BSD/Haiku?
            return None;
        };

        let version_listing = match version {
            JavaVersion::Java16 => &java_list.java_runtime_alpha,
            JavaVersion::Java17 => {
                if !java_list.java_runtime_gamma.is_empty() {
                    &java_list.java_runtime_gamma
                } else if !java_list.java_runtime_gamma_snapshot.is_empty() {
                    &java_list.java_runtime_gamma_snapshot
                } else {
                    &java_list.java_runtime_beta
                }
            }
            JavaVersion::Java21 => &java_list.java_runtime_delta,
            JavaVersion::Java8 => &java_list.jre_legacy,
        };

        let first_version = version_listing.first()?;
        Some(first_version.manifest.url.clone())
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct JavaList {
    /// Java 16.0.1.9.1
    java_runtime_alpha: Vec<JavaInstallListing>,
    /// Java 17.0.1.12.1
    java_runtime_beta: Vec<JavaInstallListing>,
    /// Java 21.0.3
    java_runtime_delta: Vec<JavaInstallListing>,
    /// Java 17.0.8
    java_runtime_gamma: Vec<JavaInstallListing>,
    /// Java 17.0.8
    java_runtime_gamma_snapshot: Vec<JavaInstallListing>,
    // TODO: Some platforms need exotic versions of Java 8?
    /// Java 8u202
    jre_legacy: Vec<JavaInstallListing>,
    // Ugly windows specific thing that doesn't seem to be required?
    // minecraft_java_exe: Vec<JavaInstallListing>,
}

#[derive(Deserialize, Debug)]
pub struct JavaInstallListing {
    // availability: JavaInstallListingAvailability,
    manifest: JavaInstallListingManifest,
    // version: JavaInstallListingVersion,
}

// WTF: Yes this is approaching Java levels of name length.
// #[derive(Deserialize, Debug)]
// pub struct JavaInstallListingAvailability {
// group: i64,
// progress: i64,
// }

#[derive(Deserialize, Debug)]
pub struct JavaInstallListingManifest {
    // sha1: String,
    // size: usize,
    url: String,
}

// #[derive(Deserialize, Debug)]
// pub struct JavaInstallListingVersion {
// name: String,
// released: String,
// }
