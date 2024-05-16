use serde::{Deserialize, Serialize};

use crate::file_utils;

use super::JsonDownloadError;

pub const JAVA_LIST_URL: &str = "https://launchermeta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json";

pub enum JavaVersion {
    Java16,
    Java17Beta,
    Java21,
    Java17Gamma,
    Java17GammaSnapshot,
    Java8,
}

#[derive(Serialize, Deserialize, Debug)]
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
        let client = reqwest::Client::new();
        let json = file_utils::download_file_to_string(&client, JAVA_LIST_URL).await?;
        Ok(serde_json::from_str(&json)?)
    }

    pub fn get_url(&self, version: JavaVersion) -> Option<String> {
        let java_list = if cfg!(target_os = "linux") {
            if cfg!(target_arch = "x86") {
                &self.linux_i386
            } else if cfg!(target_arch = "x86_64") {
                &self.linux
            } else {
                // TODO Unsupported architecture handling.
                // Some people might play this on powerpc/risc v?
                panic!("Java Install - Unsupported Architecture");
            }
        } else if cfg!(target_os = "macos") {
            // aarch64
            if cfg!(target_arch = "aarch64") {
                &self.mac_os_arm64
            } else if cfg!(target_arch = "x86_64") {
                &self.mac_os
            } else {
                // TODO Unsupported architecture handling.
                // Some people might play this on powerpc/risc v?
                panic!("Java Install - Unsupported Architecture");
            }
        } else if cfg!(target_os = "windows") {
            if cfg!(target_arch = "x86") {
                &self.windows_x86
            } else if cfg!(target_arch = "x86_64") {
                &self.windows_x64
            } else if cfg!(target_arch = "aarch64") {
                &self.windows_arm64
            } else {
                // TODO Unsupported architecture handling.
                // Some people might play this on powerpc/risc v?
                panic!("Java Install - Unsupported Architecture");
            }
        } else {
            // TODO Unsupported OS handling.
            // Some people might play this on Solaris/BSD?
            panic!("Java Install - Unsupported OS")
        };

        let version = match version {
            JavaVersion::Java16 => &java_list.java_runtime_alpha,
            JavaVersion::Java17Beta => &java_list.java_runtime_beta,
            JavaVersion::Java21 => &java_list.java_runtime_delta,
            JavaVersion::Java17Gamma => &java_list.java_runtime_gamma,
            JavaVersion::Java17GammaSnapshot => &java_list.java_runtime_gamma_snapshot,
            JavaVersion::Java8 => &java_list.jre_legacy,
        };

        let first_version = version.first()?;
        Some(first_version.manifest.url.clone())
    }
}

#[derive(Serialize, Deserialize, Debug)]
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

#[derive(Serialize, Deserialize, Debug)]
pub struct JavaInstallListing {
    pub availability: JavaInstallListingAvailability,
    pub manifest: JavaInstallListingManifest,
    pub version: JavaInstallListingVersion,
}

// Yes this is approaching Java levels of name length.
#[derive(Serialize, Deserialize, Debug)]
pub struct JavaInstallListingAvailability {
    pub group: i64,
    pub progress: i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct JavaInstallListingManifest {
    pub sha1: String,
    pub size: usize,
    pub url: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct JavaInstallListingVersion {
    pub name: String,
    pub released: String,
}

#[cfg(test)]
mod tests {
    use reqwest::blocking::Client;

    use super::*;

    #[test]
    fn test_java_list_deserialize() {
        let client = Client::new();
        let response = client.get(JAVA_LIST_URL).send().unwrap();

        let text = response.text().unwrap();
        let json: JavaListJson = serde_json::from_str(&text).unwrap();

        dbg!(json);
    }
}
