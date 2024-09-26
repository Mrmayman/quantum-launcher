use std::{
    ffi::OsStr,
    fmt::Display,
    process::Command,
    sync::{
        mpsc::{SendError, Sender},
        Arc,
    },
};

use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::{
    error::IoError,
    file_utils::{self, RequestError},
    info, io_err, LAUNCHER_VERSION,
};

#[derive(Debug, Clone)]
pub enum UpdateCheckInfo {
    UpToDate,
    NewVersion { url: String },
}

pub async fn check_for_updates_wrapped() -> Result<UpdateCheckInfo, String> {
    check_for_updates().await.map_err(|err| err.to_string())
}

async fn download_release_info() -> Result<String, RequestError> {
    const URL: &str = "https://api.github.com/repos/Mrmayman/quantum-launcher/releases";

    let client = Client::new();
    let response = client
        .get(URL)
        .header("User-Agent", "quantumlauncher")
        .send()
        .await?;
    if response.status().is_success() {
        Ok(response.text().await?)
    } else {
        Err(RequestError::DownloadError {
            code: response.status(),
            url: response.url().clone(),
        })
    }
}

pub async fn check_for_updates() -> Result<UpdateCheckInfo, UpdateError> {
    let json = download_release_info().await?;
    let json: Vec<GithubRelease> = serde_json::from_str(&json)?;

    let latest = json.first().ok_or(UpdateError::NoReleases)?;

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

    match version.cmp(&LAUNCHER_VERSION) {
        std::cmp::Ordering::Less => Err(UpdateError::AheadOfLatestVersion),
        std::cmp::Ordering::Equal => Ok(UpdateCheckInfo::UpToDate),
        std::cmp::Ordering::Greater => {
            let arch = if cfg!(target_arch = "x86_64") {
                "x86_64"
            } else {
                eprintln!("[error] Update checking: Unsupported architecture");
                return Err(UpdateError::UnsupportedArchitecture);
            };

            let os = if cfg!(target_os = "windows") {
                "windows"
            } else if cfg!(target_os = "linux") {
                "linux"
            } else {
                eprintln!("[error] Update checking: Unsupported OS");
                return Err(UpdateError::UnsupportedOS);
            };

            let name = format!("quantum_launcher_{os}_{arch}.");

            let matching_release = latest
                .assets
                .iter()
                .find(|asset| asset.name.starts_with(&name))
                .ok_or(UpdateError::NoMatchingDownloadFound)?;

            Ok(UpdateCheckInfo::NewVersion {
                url: matching_release.browser_download_url.to_owned(),
            })
        }
    }
}

pub async fn install_update_wrapped(
    url: String,
    progress: Sender<UpdateProgress>,
) -> Result<(), String> {
    install_update(url, progress)
        .await
        .map_err(|err| err.to_string())
}

pub enum UpdateProgress {
    P1Start,
    P2Backup,
    P3Download,
    P4Extract,
}

pub async fn install_update(
    url: String,
    progress: Sender<UpdateProgress>,
) -> Result<(), UpdateError> {
    progress.send(UpdateProgress::P1Start)?;

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
    progress.send(UpdateProgress::P2Backup)?;
    let backup_path = exe_location.join(format!("backup_{backup_idx}_{exe_name}"));
    std::fs::rename(&exe_path, &backup_path).map_err(io_err!(backup_path))?;

    info!("Downloading new version of launcher");
    progress.send(UpdateProgress::P3Download)?;
    let client = reqwest::Client::new();
    let download_zip = file_utils::download_file_to_bytes(&client, &url).await?;

    info!("Extracting launcher");
    progress.send(UpdateProgress::P4Extract)?;
    zip_extract::extract(std::io::Cursor::new(download_zip), exe_location, true)?;
    let extract_name = if cfg!(target_os = "windows") {
        "quantum_launcher.exe"
    } else {
        "quantum_launcher"
    };

    let new_path = exe_location.join(extract_name);
    let _ = Command::new(&new_path).spawn().map_err(io_err!(new_path))?;

    std::process::exit(0);
}

pub enum UpdateError {
    Request(RequestError),
    Serde(serde_json::Error),
    NoReleases,
    SemverError(semver::Error),
    UnsupportedOS,
    UnsupportedArchitecture,
    NoMatchingDownloadFound,
    AheadOfLatestVersion,
    CurrentExeError(std::io::Error),
    ExeParentPathError,
    ExeFileNameError,
    OsStrToStr(Arc<OsStr>),
    Io(IoError),
    Zip(zip_extract::ZipExtractError),
    Send(SendError<UpdateProgress>),
}

impl Display for UpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "update check error: ")?;
        match self {
            UpdateError::Request(request_error) => write!(f, "request: {request_error}"),
            UpdateError::Serde(error) => write!(f, "json: {error}"),
            UpdateError::NoReleases => write!(f, "no releases found"),
            UpdateError::SemverError(error) => write!(f, "semver: {error}"),
            UpdateError::UnsupportedOS => write!(f, "unsupported os"),
            UpdateError::UnsupportedArchitecture => write!(f, "unsupported architecture"),
            UpdateError::NoMatchingDownloadFound => {
                write!(f, "no matching download found for your platform")
            }
            UpdateError::AheadOfLatestVersion => {
                write!(f, "current version is ahead of latest version! dev build?")
            }
            UpdateError::CurrentExeError(error) => {
                write!(f, "could not get current executable path: {error}")
            }
            UpdateError::ExeParentPathError => {
                write!(f, "could not get parent dir of current executable")
            }
            UpdateError::ExeFileNameError => {
                write!(f, "could not get file name of current executable")
            }
            UpdateError::OsStrToStr(arc) => write!(f, "could not convert OsStr to str: {arc:?}"),
            UpdateError::Io(io_error) => write!(f, "io error: {io_error}"),
            UpdateError::Zip(zip_extract_error) => {
                write!(f, "zip extract error: {zip_extract_error}")
            }
            UpdateError::Send(send_error) => write!(f, "progress send error: {send_error}"),
        }
    }
}

type SerdeError = serde_json::Error;
type SemverError = semver::Error;
type ZipError = zip_extract::ZipExtractError;
type SendErr = SendError<UpdateProgress>;

macro_rules! impl_error {
    ($from:ident, $to:ident) => {
        impl From<$from> for UpdateError {
            fn from(value: $from) -> Self {
                UpdateError::$to(value)
            }
        }
    };
}

impl_error!(RequestError, Request);
impl_error!(SerdeError, Serde);
impl_error!(SemverError, SemverError);
impl_error!(IoError, Io);
impl_error!(ZipError, Zip);
impl_error!(SendErr, Send);

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
