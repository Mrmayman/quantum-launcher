use std::{
    ffi::OsStr,
    process::Command,
    sync::{
        mpsc::{SendError, Sender},
        Arc,
    },
};

use ql_core::{err, file_utils, info, GenericProgress, IntoIoError, IoError, RequestError};
use serde::Deserialize;
use thiserror::Error;

use crate::LAUNCHER_VERSION;

#[derive(Debug, Clone)]
pub enum UpdateCheckInfo {
    UpToDate,
    NewVersion { url: String },
}

/// [`check_for_launcher_updates`] `_w` function
pub async fn check_for_launcher_updates_w() -> Result<UpdateCheckInfo, String> {
    check_for_launcher_updates()
        .await
        .map_err(|err| err.to_string())
}

/// Checks for any launcher updates to be installed.
///
/// Returns `Ok(UpdateCheckInfo::UpToDate)` if the launcher is up to date.
///
/// Returns `Ok(UpdateCheckInfo::NewVersion { url })` if there is a new version available.
/// (url pointing to zip file containing new version executable).
pub async fn check_for_launcher_updates() -> Result<UpdateCheckInfo, UpdateError> {
    const URL: &str = "https://api.github.com/repos/Mrmayman/quantum-launcher/releases";

    let json = file_utils::download_file_to_string(URL, true).await?;
    let json: Vec<GithubRelease> = serde_json::from_str(&json)?;

    let latest = json.first().ok_or(UpdateError::NoReleases)?;

    let mut version = latest.tag_name.clone();
    // v0.2 -> 0.2
    if version.starts_with('v') {
        version = version[1..version.len()].to_owned();
    }
    // 0.2 -> 0.2.0
    if version.chars().filter(|n| *n == '.').count() == 1 {
        version.push_str(".0");
    }

    let version = semver::Version::parse(&version)?;

    match version.cmp(&LAUNCHER_VERSION) {
        std::cmp::Ordering::Less => Err(UpdateError::AheadOfLatestVersion),
        std::cmp::Ordering::Equal => Ok(UpdateCheckInfo::UpToDate),
        std::cmp::Ordering::Greater => {
            let arch = if cfg!(target_arch = "x86_64") {
                "x86_64"
            } else if cfg!(target_arch = "aarch64") {
                "aarch64"
            } else {
                err!("Update checking: Unsupported architecture");
                return Err(UpdateError::UnsupportedArchitecture);
            };

            let os = if cfg!(target_os = "windows") {
                "windows"
            } else if cfg!(target_os = "linux") {
                "linux"
            } else if cfg!(target_os = "macos") {
                "macos"
            } else {
                err!("Update checking: Unsupported OS");
                return Err(UpdateError::UnsupportedOS);
            };

            let name = format!("quantum_launcher_{os}_{arch}.");

            let matching_release = latest
                .assets
                .iter()
                .find(|asset| asset.name.replace('-', "_").starts_with(&name))
                .ok_or(UpdateError::NoMatchingDownloadFound)?;

            Ok(UpdateCheckInfo::NewVersion {
                url: matching_release.browser_download_url.clone(),
            })
        }
    }
}

/// [`install_launcher_update`] `_w` function
pub async fn install_launcher_update_w(
    url: String,
    progress: Sender<GenericProgress>,
) -> Result<(), String> {
    install_launcher_update(url, progress)
        .await
        .map_err(|err| err.to_string())
}

/// Installs a new version of the launcher.
/// The launcher will be backed up, the new version
/// will be downloaded and extracted.
///
/// The new version will be started and the current process will exit.
///
/// # Arguments
/// - `url`: The url to the zip file containing the new version of the launcher.
/// - `progress`: A channel to send progress updates to.
pub async fn install_launcher_update(
    url: String,
    progress: Sender<GenericProgress>,
) -> Result<(), UpdateError> {
    progress.send(GenericProgress::default())?;

    let exe_path = std::env::current_exe().map_err(UpdateError::CurrentExeError)?;
    let exe_location = exe_path.parent().ok_or(UpdateError::ExeParentPathError)?;

    let exe_name = exe_path.file_name().ok_or(UpdateError::ExeFileNameError)?;
    let exe_name = exe_name
        .to_str()
        .ok_or(UpdateError::OsStrToStr(exe_name.into()))?;

    let mut backup_idx = 1;
    while exe_location
        .join(format!("backup_{backup_idx}_{exe_name}"))
        .exists()
    {
        backup_idx += 1;
    }

    info!("Backing up existing launcher");
    progress.send(GenericProgress {
        done: 1,
        total: 4,
        message: Some("Backing up existing launcher".to_owned()),
        has_finished: false,
    })?;
    let backup_path = exe_location.join(format!("backup_{backup_idx}_{exe_name}"));
    tokio::fs::rename(&exe_path, &backup_path)
        .await
        .path(backup_path)?;

    info!("Downloading new version of launcher");
    progress.send(GenericProgress {
        done: 2,
        total: 4,
        message: Some("Downloading new launcher".to_owned()),
        has_finished: false,
    })?;
    let download_zip = file_utils::download_file_to_bytes(&url, false).await?;

    info!("Extracting launcher");
    progress.send(GenericProgress {
        done: 3,
        total: 4,
        message: Some("Extracting new launcher".to_owned()),
        has_finished: false,
    })?;
    zip_extract::extract(std::io::Cursor::new(download_zip), exe_location, true)?;

    // Should I, though?
    let rm_path = exe_location.join("README.md");
    if rm_path.exists() {
        tokio::fs::remove_file(&rm_path).await.path(rm_path)?;
    }
    let rm_path = exe_location.join("LICENSE");
    if rm_path.exists() {
        tokio::fs::remove_file(&rm_path).await.path(rm_path)?;
    }

    let extract_name = if cfg!(target_os = "windows") {
        "quantum_launcher.exe"
    } else {
        "quantum_launcher"
    };

    let new_path = exe_location.join(extract_name);
    _ = Command::new(&new_path).spawn().path(new_path)?;

    std::process::exit(0);
}

#[derive(Debug, Error)]
pub enum UpdateError {
    #[error("launcher update error: {0}")]
    Request(#[from] RequestError),
    #[error("launcher update error: json error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("no launcher update releases found")]
    NoReleases,
    #[error("launcher update error: semver error: {0}")]
    SemverError(#[from] semver::Error),
    #[error("unsupported OS for launcher update")]
    UnsupportedOS,
    #[error("unsupported architecture for launcher update")]
    UnsupportedArchitecture,
    #[error("no matching launcher update download found for your platform")]
    NoMatchingDownloadFound,
    #[error("current launcher version is ahead of latest version! dev build?")]
    AheadOfLatestVersion,
    #[error("launcher update error: could not get current exe path: {0}")]
    CurrentExeError(std::io::Error),
    #[error("launcher update error: could not get current exe parent path")]
    ExeParentPathError,
    #[error("launcher update error: could not get current exe file name")]
    ExeFileNameError,
    #[error("launcher update error: could not convert OsStr to str: {0:?}")]
    OsStrToStr(Arc<OsStr>),
    #[error("launcher update error: {0}")]
    Io(#[from] IoError),
    #[error("launcher update error: zip extract error: {0}")]
    Zip(#[from] zip_extract::ZipExtractError),
    #[error("launcher update error: send error: {0}")]
    Send(#[from] SendError<GenericProgress>),
}

#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
    assets: Vec<GithubAsset>,
    // url: String,
    // assets_url: String,
    // upload_url: String,
    // html_url: String,
    // id: usize,
    // name: String,
    // draft: bool,
    // prerelease: bool,
    // created_at: String,
    // published_at: String,
}

#[derive(Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
}
