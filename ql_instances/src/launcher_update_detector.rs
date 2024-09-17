use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::{
    file_utils::{self, RequestError},
    LAUNCHER_VERSION,
};

pub enum UpdateCheckInfo {
    UpToDate,
    NewVersion { url: String },
}

pub async fn check_for_updates() -> Result<UpdateCheckInfo, UpdateCheckError> {
    let client = Client::new();

    let url = "https://api.github.com/repos/Mrmayman/quantum-launcher/releases";
    let json = file_utils::download_file_to_string(&client, url).await?;
    let json: Vec<GithubRelease> = serde_json::from_str(&json)?;

    let latest = json.first().ok_or(UpdateCheckError::NoReleases)?;

    let mut version = latest.tag_name.to_owned();
    // v0.2 -> 0.2
    if version.starts_with('v') {
        version = version[1..version.len()].to_owned()
    }
    // 0.2 -> 0.2.0
    if version.chars().filter(|n| *n == '.').count() == 1 {
        version.push_str(".0");
    }

    let version = semver::Version::parse(&version)?;

    if version > LAUNCHER_VERSION {
        let arch = if cfg!(target_arch = "x86_64") {
            "x86_64"
        } else {
            eprintln!("[error] Update checking: Unsupported architecture");
            return Err(UpdateCheckError::UnsupportedArchitecture);
        };

        let os = if cfg!(target_os = "windows") {
            "windows"
        } else if cfg!(target_os = "linux") {
            "linux"
        } else {
            eprintln!("[error] Update checking: Unsupported OS");
            return Err(UpdateCheckError::UnsupportedOS);
        };

        let name = format!("quantum_launcher_{os}_{arch}.");

        let matching_release = latest
            .assets
            .iter()
            .find(|asset| asset.name.starts_with(&name))
            .ok_or(UpdateCheckError::NoMatchingReleaseFound)?;

        Ok(UpdateCheckInfo::NewVersion {
            url: matching_release.browser_download_url.to_owned(),
        })
    } else if version < LAUNCHER_VERSION {
        Err(UpdateCheckError::AheadOfLatestVersion)
    } else {
        Ok(UpdateCheckInfo::UpToDate)
    }
}

pub enum UpdateCheckError {
    Request(RequestError),
    Serde(serde_json::Error),
    NoReleases,
    SemverError(semver::Error),
    UnsupportedOS,
    UnsupportedArchitecture,
    NoMatchingReleaseFound,
    AheadOfLatestVersion,
}

type SerdeError = serde_json::Error;
type SemverError = semver::Error;

macro_rules! impl_error {
    ($from:ident, $to:ident) => {
        impl From<$from> for UpdateCheckError {
            fn from(value: $from) -> Self {
                UpdateCheckError::$to(value)
            }
        }
    };
}

impl_error!(RequestError, Request);
impl_error!(SerdeError, Serde);
impl_error!(SemverError, SemverError);

#[derive(Serialize, Deserialize)]
struct GithubRelease {
    url: String,
    assets_url: String,
    upload_url: String,
    html_url: String,
    id: usize,
    tag_name: String,
    name: String,
    draft: bool,
    prerelease: bool,
    created_at: String,
    published_at: String,
    assets: Vec<GithubAsset>,
}

#[derive(Serialize, Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
}
