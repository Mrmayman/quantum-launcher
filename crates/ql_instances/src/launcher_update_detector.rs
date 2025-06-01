use std::{
    ffi::OsStr,
    process::Command,
    sync::{mpsc::Sender, Arc},
};

use ql_core::{
    err, file_utils, impl_3_errs_jri, info, GenericProgress, IntoIoError, IoError, JsonError,
    RequestError,
};
use serde::Deserialize;
use thiserror::Error;

use crate::LAUNCHER_VERSION;

#[derive(Debug, Clone)]
pub enum UpdateCheckInfo {
    UpToDate,
    NewVersion { url: String },
}

/// Checks for any launcher updates to be installed.
///
/// Returns `Ok(UpdateCheckInfo::UpToDate)` if the launcher is up to date.
///
/// Returns `Ok(UpdateCheckInfo::NewVersion { url: String })` if there is a new version available.
/// (url pointing to zip file containing new version executable).
///
/// # Errors
/// - If current version is ahead of latest version
///   (experimental dev build)
/// - If the release info couldn't be downloaded
/// - If the release info couldn't be parsed into JSON
/// - If no releases could be found in the Github repo
/// - If the new version's version number is incompatible
///   with semver (even after conversion)
/// - If user is on unsupported architecture
/// - If user is on unsupported OS
/// - If no matching download could be found for your OS
pub async fn check_for_launcher_updates() -> Result<UpdateCheckInfo, UpdateError> {
    const URL: &str = "https://api.github.com/repos/Mrmayman/quantum-launcher/releases";

    let json: Vec<GithubRelease> = file_utils::download_file_to_json(URL, true).await?;
    let mut json = json.into_iter();

    let mut version;
    let mut latest;

    loop {
        latest = json.next().ok_or(UpdateError::NoReleases)?;

        version = latest.tag_name.clone();
        // v0.2 -> 0.2
        if version.starts_with('v') {
            version = version[1..version.len()].to_owned();
        }
        // 0.2 -> 0.2.0
        if version.chars().filter(|n| *n == '.').count() == 1 {
            version.push_str(".0");
        }

        // The new update has been disabled/yanked for whatever reason
        // so look for another one.
        // Naming scheme: ends with "-D" followed by (optional) numbers
        if version
            .trim_end_matches(|c: char| c.is_numeric())
            .ends_with("-D")
        {
            continue;
        }
        break;
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
            } else if cfg!(target_arch = "arm") {
                "arm32"
            } else if cfg!(target_arch = "x86") {
                "i686"
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
            // Note: Currently not supported,
            // but hook left here for any future plans
            } else if cfg!(target_os = "freebsd") {
                "freebsd"
            } else if cfg!(target_os = "netbsd") {
                "netbsd"
            } else if cfg!(target_os = "solaris") {
                "solaris"
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

/// Installs a new version of the launcher.
/// The launcher will be backed up, the new version
/// will be downloaded and extracted.
///
/// The new version will be started and the current process will exit.
///
/// # Arguments
/// - `url`: The url to the zip file containing the new version of the launcher.
/// - `progress`: A channel to send progress updates to.
///
/// # Errors
/// ## New version:
/// - Couldn't be downloaded
/// - Couldn't be extracted (invalid zip)
/// - Couldn't be started
/// ## Current executable:
/// - Couldn't be found
/// - Has a name with invalid unicode
pub async fn install_launcher_update(
    url: String,
    progress: Sender<GenericProgress>,
) -> Result<(), UpdateError> {
    _ = progress.send(GenericProgress::default());

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
    _ = progress.send(GenericProgress {
        done: 1,
        total: 4,
        message: Some("Backing up existing launcher".to_owned()),
        has_finished: false,
    });
    let backup_path = exe_location.join(format!("backup_{backup_idx}_{exe_name}"));
    tokio::fs::rename(&exe_path, &backup_path)
        .await
        .path(backup_path)?;

    info!("Downloading new version of launcher");
    _ = progress.send(GenericProgress {
        done: 2,
        total: 4,
        message: Some("Downloading new launcher".to_owned()),
        has_finished: false,
    });
    let download_zip = file_utils::download_file_to_bytes(&url, false).await?;

    info!("Extracting launcher");
    _ = progress.send(GenericProgress {
        done: 3,
        total: 4,
        message: Some("Extracting new launcher".to_owned()),
        has_finished: false,
    });
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

const UPDATE_ERR_PREFIX: &str = "while managing launcher update:\n";

#[derive(Debug, Error)]
pub enum UpdateError {
    #[error("{UPDATE_ERR_PREFIX}{0}")]
    Request(#[from] RequestError),
    #[error("{UPDATE_ERR_PREFIX}{0}")]
    Serde(#[from] JsonError),
    #[error("{UPDATE_ERR_PREFIX}no Release entries found in launcher github")]
    NoReleases,
    #[error("{UPDATE_ERR_PREFIX}semver error: {0}")]
    SemverError(#[from] semver::Error),
    #[error("unsupported OS for launcher update")]
    UnsupportedOS,
    #[error("unsupported architecture for launcher update")]
    UnsupportedArchitecture,
    #[error("no matching launcher update download found for your platform")]
    NoMatchingDownloadFound,
    #[error("Current launcher version is ahead of latest version! dev build?")]
    AheadOfLatestVersion,
    #[error("{UPDATE_ERR_PREFIX}could not get current exe path: {0}")]
    CurrentExeError(std::io::Error),
    #[error("{UPDATE_ERR_PREFIX}could not get current exe parent path")]
    ExeParentPathError,
    #[error("{UPDATE_ERR_PREFIX}could not get current exe file name")]
    ExeFileNameError,
    #[error("{UPDATE_ERR_PREFIX}could not convert OsStr to str: {0:?}")]
    OsStrToStr(Arc<OsStr>),
    #[error("{UPDATE_ERR_PREFIX}{0}")]
    Io(#[from] IoError),
    #[error("{UPDATE_ERR_PREFIX}zip extract error: {0}")]
    Zip(#[from] zip_extract::ZipExtractError),
}

impl_3_errs_jri!(UpdateError, Serde, Request, Io);

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
