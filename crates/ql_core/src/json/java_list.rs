use std::fmt::Display;

use crate::{err, file_utils};
use serde::Deserialize;

use crate::JsonDownloadError;

pub const JAVA_LIST_URL: &str = "https://launchermeta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json";

#[derive(Clone, Copy)]
pub enum JavaVersion {
    Java16,
    Java17Beta,
    Java21,
    Java17Gamma,
    Java17GammaSnapshot,
    Java8,
}

impl JavaVersion {
    #[must_use]
    pub fn get_amazon_corretto_aarch64_url(&self) -> &'static str {
        match self {
            JavaVersion::Java16
            | JavaVersion::Java17Beta
            | JavaVersion::Java17Gamma
            | JavaVersion::Java17GammaSnapshot => {
                "https://corretto.aws/downloads/latest/amazon-corretto-17-aarch64-linux-jdk.tar.gz"
            }
            JavaVersion::Java21 => {
                "https://corretto.aws/downloads/latest/amazon-corretto-21-aarch64-linux-jdk.tar.gz"
            }
            JavaVersion::Java8 => {
                "https://corretto.aws/downloads/latest/amazon-corretto-8-aarch64-linux-jdk.tar.gz"
            }
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
                JavaVersion::Java17Beta => "java_17_beta",
                JavaVersion::Java21 => "java_21",
                JavaVersion::Java17Gamma => "java_17_gamma",
                JavaVersion::Java17GammaSnapshot => "java_17_gamma_snapshot",
                JavaVersion::Java8 => "java_8",
            }
        )
    }
}

impl From<crate::json::version::JavaVersion> for JavaVersion {
    fn from(version: crate::json::version::JavaVersion) -> Self {
        match version.majorVersion {
            8 => JavaVersion::Java8,
            16 => JavaVersion::Java16,
            17 => JavaVersion::Java17Gamma,
            _ => JavaVersion::Java21,
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct JavaListJson {
    pub gamecore: JavaList,
    pub linux: JavaList,
    pub linux_i386: JavaList,
    pub mac_os: JavaList,
    pub mac_os_arm64: JavaList,
    pub windows_arm64: JavaList,
    pub windows_x86: JavaList,
    pub windows_x64: JavaList,
}

impl JavaListJson {
    pub async fn download() -> Result<Self, JsonDownloadError> {
        let json = file_utils::download_file_to_string(JAVA_LIST_URL, false).await?;
        Ok(serde_json::from_str(&json)?)
    }

    pub fn get_url(&self, version: JavaVersion) -> Option<String> {
        let java_list = if cfg!(target_os = "linux") {
            if cfg!(target_arch = "x86") {
                &self.linux_i386
            } else if cfg!(target_arch = "x86_64") {
                &self.linux
            } else {
                err!("Unsupported architecture!");
                // TODO Unsupported architecture handling.
                // Add ARM32, RISC-V, and PowerPC support.
                return None;
            }
        } else if cfg!(target_os = "macos") {
            // aarch64
            if cfg!(target_arch = "aarch64") {
                &self.mac_os_arm64
            } else if cfg!(target_arch = "x86_64") {
                &self.mac_os
            } else {
                err!("Unsupported architecture!");
                // TODO Unsupported architecture handling.
                // Add x86 and PowerPC support.
                return None;
            }
        } else if cfg!(target_os = "windows") {
            if cfg!(target_arch = "x86") {
                &self.windows_x86
            } else if cfg!(target_arch = "x86_64") {
                &self.windows_x64
            } else if cfg!(target_arch = "aarch64") {
                &self.windows_arm64
            } else {
                err!("Unsupported architecture!");
                // TODO Unsupported architecture handling.
                // What if Windows supports some other architecture
                // in the future?
                return None;
            }
        } else {
            err!("Unsupported OS!");
            // TODO Unsupported OS handling.
            // Some people might play this on Solaris/BSD/Haiku?
            return None;
        };

        let version_listing = match version {
            JavaVersion::Java16 => &java_list.java_runtime_alpha,
            JavaVersion::Java17Beta => &java_list.java_runtime_beta,
            JavaVersion::Java21 => &java_list.java_runtime_delta,
            JavaVersion::Java17Gamma => &java_list.java_runtime_gamma,
            JavaVersion::Java17GammaSnapshot => &java_list.java_runtime_gamma_snapshot,
            JavaVersion::Java8 => &java_list.jre_legacy,
        };

        let Some(first_version) = version_listing.first() else {
            err!("{version} doesn't support your OS or Architecture!");
            return None;
        };
        Some(first_version.manifest.url.clone())
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct JavaList {
    /// Java 16
    pub java_runtime_alpha: Vec<JavaInstallListing>,
    /// Java 17
    pub java_runtime_beta: Vec<JavaInstallListing>,
    /// Java 21
    pub java_runtime_delta: Vec<JavaInstallListing>,
    /// Java 17
    pub java_runtime_gamma: Vec<JavaInstallListing>,
    /// Java 17
    pub java_runtime_gamma_snapshot: Vec<JavaInstallListing>,
    /// Java 8
    pub jre_legacy: Vec<JavaInstallListing>,
    pub minecraft_java_exe: Vec<JavaInstallListing>,
}

#[derive(Deserialize, Debug)]
pub struct JavaInstallListing {
    pub availability: JavaInstallListingAvailability,
    pub manifest: JavaInstallListingManifest,
    pub version: JavaInstallListingVersion,
}

// WTF: Yes this is approaching Java levels of name length.
#[derive(Deserialize, Debug)]
pub struct JavaInstallListingAvailability {
    pub group: i64,
    pub progress: i64,
}

#[derive(Deserialize, Debug)]
pub struct JavaInstallListingManifest {
    pub sha1: String,
    pub size: usize,
    pub url: String,
}

#[derive(Deserialize, Debug)]
pub struct JavaInstallListingVersion {
    pub name: String,
    pub released: String,
}
