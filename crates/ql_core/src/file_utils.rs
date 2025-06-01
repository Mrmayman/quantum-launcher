use std::{
    collections::HashSet,
    fs::Metadata,
    path::{Path, PathBuf},
    sync::LazyLock,
};

use futures::StreamExt;
use ql_reqwest::header::InvalidHeaderValue;
use serde::de::DeserializeOwned;
use thiserror::Error;
use tokio::fs::DirEntry;
use tokio_util::io::StreamReader;

use crate::{
    error::{DownloadFileError, IoError},
    info_no_log, retry, IntoIoError, JsonDownloadError, CLIENT,
};

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
        n.path
    } else {
        dirs::config_dir()
            .ok_or(IoError::ConfigDirNotFound)?
            .join("QuantumLauncher")
    };

    std::fs::create_dir_all(&launcher_directory).path(&launcher_directory)?;
    Ok(launcher_directory)
}

struct QlDirInfo {
    path: PathBuf,
}

fn line_and_body(input: &str) -> (String, String) {
    let mut lines = input.trim().lines();

    // Get the first line (if any)
    if let Some(first) = lines.next() {
        let rest = lines.collect::<Vec<_>>().join("\n");
        return (first.trim().to_owned(), rest);
    }

    (String::default(), String::default())
}

fn check_qlportable_file() -> Option<QlDirInfo> {
    const PORTABLE_FILENAME: &str = "qldir.txt";

    let places = [
        std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(Path::to_owned)),
        std::env::current_dir().ok(),
        dirs::config_dir().map(|d| d.join("QuantumLauncher")),
    ];

    for (i, place) in places
        .into_iter()
        .enumerate()
        .filter_map(|(i, n)| n.map(|n| (i, n)))
    {
        let qldir_path = place.join(PORTABLE_FILENAME);
        let Ok(contents) = std::fs::read_to_string(&qldir_path) else {
            continue;
        };
        let (path, qldir_options) = line_and_body(&contents);

        let flags: HashSet<_> = qldir_options
            .split(',')
            .map(|s| s.trim().to_lowercase())
            .collect();
        let mut join_dir = !flags.contains("top");

        let path = if let (Some(stripped), Some(home)) = (path.strip_prefix("~"), dirs::home_dir())
        {
            home.join(&stripped)
        } else if path == ".." || path == "." {
            join_dir = false;
            place
        } else if path.is_empty() && i < 2 {
            place
        } else {
            PathBuf::from(&path)
        };

        return Some(if join_dir {
            QlDirInfo {
                path: path.join("QuantumLauncher"),
            }
        } else {
            QlDirInfo { path }
        });
    }

    None
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
/// - Request is rejected (HTTP status code)
/// - Redirect loop detected
/// - Redirect limit exhausted.
pub async fn download_file_to_string(url: &str, user_agent: bool) -> Result<String, RequestError> {
    async fn inner(url: &str, user_agent: bool) -> Result<String, RequestError> {
        let mut get = CLIENT.get(url);
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

    retry(async || inner(url, user_agent).await).await
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
/// - Request is rejected (HTTP status code)
/// - Redirect loop detected
/// - Redirect limit exhausted.
pub async fn download_file_to_json<T: DeserializeOwned>(
    url: &str,
    user_agent: bool,
) -> Result<T, JsonDownloadError> {
    async fn inner<T: DeserializeOwned>(
        url: &str,
        user_agent: bool,
    ) -> Result<T, JsonDownloadError> {
        let mut get = CLIENT.get(url);
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

    retry(async || inner(url, user_agent).await).await
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
/// - Request is rejected (HTTP status code)
/// - Redirect loop detected
/// - Redirect limit exhausted.
pub async fn download_file_to_bytes(url: &str, user_agent: bool) -> Result<Vec<u8>, RequestError> {
    async fn inner(url: &str, user_agent: bool) -> Result<Vec<u8>, RequestError> {
        let mut get = CLIENT.get(url);
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

    retry(async || inner(url, user_agent).await).await
}

/// Downloads a file from the given URL and saves it to a path.
///
/// This uses `tokio` streams internally allowing for highly
/// efficient downloading.
///
/// # Arguments
/// - `url`: the URL to download from
/// - `user_agent`: whether to use the quantum launcher
///   user agent (required for modrinth)
/// - `path`: the `&Path` to save the files to
///
/// # Errors
/// Returns an error if:
/// - Error sending request
/// - Request is rejected (HTTP status code)
/// - Redirect loop detected
/// - Redirect limit exhausted.
pub async fn download_file_to_path(
    url: &str,
    user_agent: bool,
    path: &Path,
) -> Result<(), DownloadFileError> {
    async fn inner(url: &str, user_agent: bool, path: &Path) -> Result<(), DownloadFileError> {
        let mut get = CLIENT.get(url);
        if user_agent {
            get = get.header("User-Agent", "quantumlauncher");
        }
        let response = get.send().await?;

        if response.status().is_success() {
            let stream = response
                .bytes_stream()
                .map(|n| n.map_err(std::io::Error::other));
            let mut stream = StreamReader::new(stream);

            if let Some(parent) = path.parent() {
                if !parent.is_dir() {
                    tokio::fs::create_dir_all(&parent).await.path(parent)?;
                }
            }

            let mut file = tokio::fs::File::create(&path).await.path(path)?;
            tokio::io::copy(&mut stream, &mut file).await.path(path)?;
            Ok(())
        } else {
            Err(RequestError::DownloadError {
                code: response.status(),
                url: response.url().clone(),
            }
            .into())
        }
    }

    retry(async || inner(url, user_agent, path).await).await
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
/// - Request is rejected (HTTP status code)
/// - Redirect loop detected
/// - Redirect limit exhausted.
pub async fn download_file_to_bytes_with_agent(
    url: &str,
    user_agent: &str,
) -> Result<Vec<u8>, RequestError> {
    async fn inner(url: &str, user_agent: &str) -> Result<Vec<u8>, RequestError> {
        let response = CLIENT
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

    retry(async || inner(url, user_agent).await).await
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
        code: ql_reqwest::StatusCode,
        url: ql_reqwest::Url,
    },
    #[error("Network Request Error{NETWORK_ERROR_MSG}{0}")]
    ReqwestError(#[from] ql_reqwest::Error),
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
    if !logs_dir.is_dir() {
        tokio::fs::create_dir_all(&logs_dir).await.path(logs_dir)?;
        return Ok(());
    }
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
