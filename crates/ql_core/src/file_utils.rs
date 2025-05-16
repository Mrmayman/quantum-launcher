use std::{
    fs::Metadata,
    path::{Path, PathBuf},
    sync::LazyLock,
};

use reqwest::{header::InvalidHeaderValue, Client};
use serde::de::DeserializeOwned;
use thiserror::Error;
use tokio::fs::DirEntry;

use crate::{error::IoError, info_no_log, retry, IntoIoError, JsonDownloadError, CLIENT};

/// The path to the QuantumLauncher root folder.
///
/// This uses the current dir or executable location (portable mode)
/// if a `qlportable.txt` is found, otherwise it uses the system config dir:
/// - `~/.config` on Linux
/// - `~/AppData/Roaming` on Windows
/// - `~/Library/Application Support` on macOS
///
/// Use [`get_launcher_dir`] for a non-panicking solution.
///
/// # Panics
/// - if config dir is not found
/// - if you're on an unsupported platform (other than Windows, Linux, macOS, Redox, any linux-like unix)
/// - if the launcher directory could not be created (permissions issue)
#[allow(clippy::doc_markdown)]
pub static LAUNCHER_DIR: LazyLock<PathBuf> = LazyLock::new(|| get_launcher_dir().unwrap());

/// Returns the path to the QuantumLauncher root folder.
///
/// This uses the current dir or executable location (portable mode)
/// if a `qlportable.txt` is found, otherwise it uses the system config dir:
/// - `~/.config` on Linux
/// - `~/AppData/Roaming` on Windows
/// - `~/Library/Application Support` on macOS
///
/// # Errors
/// - if config dir is not found
/// - if you're on an unsupported platform (other than Windows, Linux, macOS, Redox, any linux-like unix)
/// - if the launcher directory could not be created (permissions issue)
#[allow(clippy::doc_markdown)]
pub fn get_launcher_dir() -> Result<PathBuf, IoError> {
    let launcher_directory = if let Some(n) = check_qlportable_file() {
        n
    } else {
        dirs::config_dir().ok_or(IoError::ConfigDirNotFound)?
    }
    .join("QuantumLauncher");
    std::fs::create_dir_all(&launcher_directory).path(&launcher_directory)?;
    Ok(launcher_directory)
}

fn check_qlportable_file() -> Option<PathBuf> {
    fn check_file(dir: Option<PathBuf>) -> Option<PathBuf> {
        const PORTABLE_FILENAME: &str = "qldir.txt";
        let dir = dir?;

        let file_path = dir.join(PORTABLE_FILENAME);
        if let Ok(mut n) = std::fs::read_to_string(&file_path) {
            // Handling of Home Directory `~`
            if let Some(short) = n.strip_prefix("~/") {
                if let Some(home) = dirs::home_dir().and_then(|n| n.to_str().map(str::to_owned)) {
                    n = format!("{home}/{short}");
                }
            }

            let n = n.trim();
            let path = PathBuf::from(n);

            if !n.is_empty() && path.is_dir() {
                info_no_log!("Custom dir: {n}/QuantumLauncher");
                Some(path)
            } else {
                file_path.exists().then_some(dir)
            }
        } else {
            None
        }
    }

    check_file(std::env::current_dir().ok())
        .or_else(|| {
            check_file(
                std::env::current_exe()
                    .ok()
                    .and_then(|n| n.parent().map(Path::to_owned)),
            )
        })
        .or_else(|| check_file(dirs::config_dir().map(|n| n.join("QuantumLauncher"))))
}

/// Returns whether the user is new to QuantumLauncher,
/// ie. whether they have never used the launcher before.
///
/// It checks whether the launcher directory does not exist.
#[must_use]
#[allow(clippy::doc_markdown)]
pub fn is_new_user() -> bool {
    let Some(config_directory) = dirs::config_dir() else {
        return false;
    };
    let launcher_directory = config_directory.join("QuantumLauncher");
    !launcher_directory.exists()
}

/// Downloads a file from the given URL into a `String`.
///
/// # Arguments
/// - `url`: the URL to download from
/// - `user_agent`: whether to use the quantum launcher
///   user agent (required for modrinth)
///
/// # Errors
/// Returns an error if:
/// - Error sending request
/// - Redirect loop detected
/// - Redirect limit exhausted.
pub async fn download_file_to_string(url: &str, user_agent: bool) -> Result<String, RequestError> {
    async fn inner(client: &Client, url: &str, user_agent: bool) -> Result<String, RequestError> {
        let mut get = client.get(url);
        if user_agent {
            get = get.header(
                "User-Agent",
                "Mrmayman/quantumlauncher (mrmayman.github.io/quantumlauncher)",
            );
        }
        let response = get.send().await?;
        if response.status().is_success() {
            Ok(response.text().await?)
        } else {
            Err(RequestError::DownloadError {
                code: response.status(),
                url: response.url().clone(),
            })
        }
    }

    retry(async || inner(&CLIENT, url, user_agent).await).await
}

/// Downloads a file from the given URL into a JSON.
///
/// More specifically, it tries to parse the contents
/// into anything implementing `serde::Deserialize`
///
/// # Arguments
/// - `url`: the URL to download from
/// - `user_agent`: whether to use the quantum launcher
///   user agent (required for modrinth)
///
/// # Errors
/// Returns an error if:
/// - Error sending request
/// - Redirect loop detected
/// - Redirect limit exhausted.
pub async fn download_file_to_json<T: DeserializeOwned>(
    url: &str,
    user_agent: bool,
) -> Result<T, JsonDownloadError> {
    async fn inner<T: DeserializeOwned>(
        client: &Client,
        url: &str,
        user_agent: bool,
    ) -> Result<T, JsonDownloadError> {
        let mut get = client.get(url);
        if user_agent {
            get = get.header(
                "User-Agent",
                "Mrmayman/quantumlauncher (mrmayman.github.io/quantumlauncher)",
            );
        }
        let response = get.send().await?;
        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            Err(JsonDownloadError::RequestError(
                RequestError::DownloadError {
                    code: response.status(),
                    url: response.url().clone(),
                },
            ))
        }
    }

    retry(async || inner(&CLIENT, url, user_agent).await).await
}

/// Downloads a file from the given URL into a `Vec<u8>`.
///
/// # Arguments
/// - `url`: the URL to download from
/// - `user_agent`: whether to use the quantum launcher
///   user agent (required for modrinth)
///
/// # Errors
/// Returns an error if:
/// - Error sending request
/// - Redirect loop detected
/// - Redirect limit exhausted.
pub async fn download_file_to_bytes(url: &str, user_agent: bool) -> Result<Vec<u8>, RequestError> {
    async fn inner(client: &Client, url: &str, user_agent: bool) -> Result<Vec<u8>, RequestError> {
        let mut get = client.get(url);
        if user_agent {
            get = get.header("User-Agent", "quantumlauncher");
        }
        let response = get.send().await?;
        if response.status().is_success() {
            Ok(response.bytes().await?.to_vec())
        } else {
            Err(RequestError::DownloadError {
                code: response.status(),
                url: response.url().clone(),
            })
        }
    }

    retry(async || inner(&CLIENT, url, user_agent).await).await
}

/// Downloads a file from the given URL into a `Vec<u8>`,
/// with a custom user agent.
///
/// # Arguments
/// - `url`: the URL to download from
/// - `user_agent`: whether to use the quantum launcher
///   user agent (required for modrinth)
///
/// # Errors
/// Returns an error if:
/// - Error sending request
/// - Redirect loop detected
/// - Redirect limit exhausted.
pub async fn download_file_to_bytes_with_agent(
    url: &str,
    user_agent: &str,
) -> Result<Vec<u8>, RequestError> {
    async fn inner(client: &Client, url: &str, user_agent: &str) -> Result<Vec<u8>, RequestError> {
        let response = client
            .get(url)
            .header("User-Agent", user_agent)
            .send()
            .await?;
        if response.status().is_success() {
            Ok(response.bytes().await?.to_vec())
        } else {
            Err(RequestError::DownloadError {
                code: response.status(),
                url: response.url().clone(),
            })
        }
    }

    retry(async || inner(&CLIENT, url, user_agent).await).await
}

const NETWORK_ERROR_MSG: &str = r"
- Check your internet connection
- Check if you are behind a firewall/proxy
- Try doing the action again

";

#[derive(Debug, Error)]
pub enum RequestError {
    #[error("Download Error (code {code}){NETWORK_ERROR_MSG}Url: {url}")]
    DownloadError {
        code: reqwest::StatusCode,
        url: reqwest::Url,
    },
    #[error("Network Request Error{NETWORK_ERROR_MSG}{0}")]
    ReqwestError(#[from] reqwest::Error),
    #[error("Download Error (invalid header value){NETWORK_ERROR_MSG}")]
    InvalidHeaderValue(#[from] InvalidHeaderValue),
}

/// Sets the executable bit on a file.
///
/// This makes a file executable on Unix systems,
/// ie. it can be run as a program.
///
/// # Errors
/// Returns an error if:
/// - the file does not exist
/// - the user doesn't have permission to read the file metadata
/// - the user doesn't have permission to change the file metadata
#[cfg(target_family = "unix")]
pub async fn set_executable(path: &std::path::Path) -> Result<(), IoError> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = tokio::fs::metadata(path).await.path(path)?.permissions();
    perms.set_mode(0o755); // rwxr-xr-x
    tokio::fs::set_permissions(path, perms).await.path(path)
}

#[cfg(unix)]
use std::os::unix::fs::symlink;

#[cfg(windows)]
use std::os::windows::fs::{symlink_dir, symlink_file};

/// Creates a symbolic link (ie. the thing at `src` "points" to `dest`,
/// accessing `src` will actually access `dest`)
///
/// # Errors
/// (depending on platform):
/// - If `src` already exists
/// - If `dest` doesn't exist
/// - If user doesn't have permission for `src`
/// - If the path is invalid (part of path is not a directory for example)
/// - Other niche stuff (Read only filesystem, Running out of disk space)
pub fn create_symlink(src: &Path, dest: &Path) -> Result<(), IoError> {
    #[cfg(unix)]
    {
        symlink(src, dest).path(src)
    }

    #[cfg(windows)]
    {
        if src.is_dir() {
            symlink_dir(src, dest).path(src)
        } else {
            symlink_file(src, dest).path(src)
        }
    }
}

pub async fn clean_log_spam() -> Result<(), IoError> {
    const SIZE_LIMIT_BYTES: u64 = 100 * 1024 * 1024; // 100 MB

    let logs_dir = LAUNCHER_DIR.join("logs");
    let mut total_size = 0;
    let mut files: Vec<(DirEntry, Metadata)> = Vec::new();

    let mut read_dir = tokio::fs::read_dir(&logs_dir).await.dir(&logs_dir)?;

    while let Some(entry) = read_dir.next_entry().await.dir(&logs_dir)? {
        let metadata = entry.metadata().await.path(entry.path())?;
        if metadata.is_file() {
            total_size += metadata.len();
            files.push((entry, metadata));
        }
    }

    if total_size <= SIZE_LIMIT_BYTES {
        return Ok(());
    }

    info_no_log!(
        "Log exceeded {} MB, cleaning up",
        SIZE_LIMIT_BYTES / (1024 * 1024)
    );
    files.sort_unstable_by_key(|(_, metadata)| {
        metadata.modified().unwrap_or(std::time::SystemTime::now())
    });

    for (file, metadata) in files {
        let path = file.path();
        tokio::fs::remove_file(&path).await.path(path)?;
        total_size -= metadata.len();

        if total_size <= SIZE_LIMIT_BYTES {
            break;
        }
    }

    Ok(())
}
