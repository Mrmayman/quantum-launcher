use std::{path::PathBuf, sync::atomic::AtomicBool};

use lazy_static::lazy_static;
use reqwest::Client;
use thiserror::Error;

use crate::{error::IoError, InstanceSelection, IntoIoError, CLIENT};

const REQUEST_TRY_LIMIT: usize = 5;

lazy_static! {
    pub static ref MOCK_DIR_FAILURE: AtomicBool = AtomicBool::new(false);
}

/// Returns the path to the QuantumLauncher root folder.
///
/// # Errors
/// - if config dir (~/.config on linux or AppData/Roaming on windows) is not found
/// - if you're on an unsupported platform (other than Windows, Linux, macOS, Redox, any linux-like unix)
/// - if the launcher directory could not be created (permissions issue)
pub async fn get_launcher_dir() -> Result<PathBuf, IoError> {
    let config_directory = dirs::config_dir().ok_or(IoError::ConfigDirNotFound)?;
    let launcher_directory = config_directory.join("QuantumLauncher");
    tokio::fs::create_dir_all(&launcher_directory)
        .await
        .path(&launcher_directory)?;

    Ok(launcher_directory)
}

/// Returns the path to the QuantumLauncher root folder. Sync version.
///
/// # Errors
/// - if config dir (~/.config on linux or AppData/Roaming on windows) is not found
/// - if you're on an unsupported platform (other than Windows, Linux, macOS, Redox, any linux-like unix)
/// - if the launcher directory could not be created (permissions issue)
pub fn get_launcher_dir_s() -> Result<PathBuf, IoError> {
    let config_directory = dirs::config_dir().ok_or(IoError::ConfigDirNotFound)?;
    let launcher_directory = config_directory.join("QuantumLauncher");
    std::fs::create_dir_all(&launcher_directory).path(&launcher_directory)?;

    if MOCK_DIR_FAILURE.load(std::sync::atomic::Ordering::SeqCst) {
        Err(IoError::MockError)
    } else {
        Ok(launcher_directory)
    }
}

/// Returns whether the user is new to QuantumLauncher,
/// ie. whether they have never used the launcher before.
///
/// It checks whether the launcher directory does not exist.
#[must_use]
pub fn is_new_user() -> bool {
    let Some(config_directory) = dirs::config_dir() else {
        return false;
    };
    let launcher_directory = config_directory.join("QuantumLauncher");
    !launcher_directory.exists()
}

/// Returns the path to `.minecraft` folder containing the game files.
///
/// # Errors
/// - if the instance directory is outside the launcher directory (escape attack)
/// - if config dir (~/.config on linux or AppData/Roaming on windows) is not found
/// - if the launcher directory could not be created (permissions issue)
pub async fn get_dot_minecraft_dir(selection: &InstanceSelection) -> Result<PathBuf, IoError> {
    let launcher_dir = get_launcher_dir().await?;
    let dir = match selection {
        InstanceSelection::Instance(name) => {
            launcher_dir.join("instances").join(name).join(".minecraft")
        }
        InstanceSelection::Server(name) => launcher_dir.join("servers").join(name),
    };
    if !dir.starts_with(&launcher_dir) {
        return Err(IoError::DirEscapeAttack);
    }
    Ok(dir)
}

/// Returns the path to `.minecraft` folder containing the game files. Sync version.
///
/// # Errors
/// - if the instance directory is outside the launcher directory (escape attack)
/// - if config dir (~/.config on linux or AppData/Roaming on windows) is not found
/// - if the launcher directory could not be created (permissions issue)
pub fn get_dot_minecraft_dir_s(selection: &InstanceSelection) -> Result<PathBuf, IoError> {
    let launcher_dir = get_launcher_dir_s()?;
    let mc_dir = match selection {
        InstanceSelection::Instance(name) => {
            launcher_dir.join("instances").join(name).join(".minecraft")
        }
        InstanceSelection::Server(name) => launcher_dir.join("servers").join(name),
    };
    if !mc_dir.starts_with(&launcher_dir) {
        return Err(IoError::DirEscapeAttack);
    }
    Ok(mc_dir)
}

/// Returns the path to the instance directory containing
/// QuantumLauncher-specific files.
///
/// # Errors
/// - if the instance directory is outside the launcher directory (escape attack)
/// - if config dir (~/.config on linux or AppData/Roaming on windows) is not found
/// - if the launcher directory could not be created (permissions issue)
pub async fn get_instance_dir(selection: &InstanceSelection) -> Result<PathBuf, IoError> {
    let launcher_dir = get_launcher_dir().await?;
    let instance_dir = match selection {
        InstanceSelection::Instance(name) => launcher_dir.join("instances").join(name),
        InstanceSelection::Server(name) => launcher_dir.join("servers").join(name),
    };
    if !instance_dir.starts_with(&launcher_dir) {
        return Err(IoError::DirEscapeAttack);
    }
    Ok(instance_dir)
}

/// Returns the path to the instance directory containing
/// QuantumLauncher-specific files. Sync version.
///
/// # Errors
/// - if the instance directory is outside the launcher directory (escape attack)
/// - if config dir (~/.config on linux or AppData/Roaming on windows) is not found
/// - if the launcher directory could not be created (permissions issue)
pub fn get_instance_dir_s(selection: &InstanceSelection) -> Result<PathBuf, IoError> {
    let launcher_dir = get_launcher_dir_s()?;
    let instance_dir = match selection {
        InstanceSelection::Instance(name) => launcher_dir.join("instances").join(name),
        InstanceSelection::Server(name) => launcher_dir.join("servers").join(name),
    };
    if !instance_dir.starts_with(&launcher_dir) {
        return Err(IoError::DirEscapeAttack);
    }
    Ok(instance_dir)
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

    let mut result = inner(&CLIENT, url, user_agent).await;

    for _ in 0..REQUEST_TRY_LIMIT {
        if let Ok(n) = result {
            return Ok(n);
        }
        result = inner(&CLIENT, url, user_agent).await;
    }

    result
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

    let mut result = inner(&CLIENT, url, user_agent).await;

    for _ in 0..REQUEST_TRY_LIMIT {
        if let Ok(n) = result {
            return Ok(n);
        }
        result = inner(&CLIENT, url, user_agent).await;
    }

    result
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

    let mut result = inner(&CLIENT, url, user_agent).await;

    for _ in 0..REQUEST_TRY_LIMIT {
        if let Ok(n) = result {
            return Ok(n);
        }
        result = inner(&CLIENT, url, user_agent).await;
    }

    result
}

#[derive(Debug, Error)]
pub enum RequestError {
    #[error("download error with code {code}, url {url}")]
    DownloadError {
        code: reqwest::StatusCode,
        url: reqwest::Url,
    },
    #[error("reqwest error: {0}")]
    ReqwestError(#[from] reqwest::Error),
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

// #[cfg(unix)]
// use std::os::unix::fs::symlink;

// #[cfg(windows)]
// use std::os::windows::fs::{symlink_dir, symlink_file};

// pub fn create_symlink(src: &Path, dest: &Path) -> Result<(), IoError> {
//     #[cfg(unix)]
//     {
//         symlink(src, dest).path(src.clone())
//     }

//     #[cfg(windows)]
//     {
//         if src.is_dir() {
//             symlink_dir(src, dest).path(src.clone())
//         } else {
//             symlink_file(src, dest).path(src.clone())
//         }
//     }
// }
