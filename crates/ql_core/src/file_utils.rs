use std::{
    fmt::Display,
    path::{Path, PathBuf},
};

use reqwest::Client;

use crate::{error::IoError, InstanceSelection, IntoIoError};

/// Returns the path to the QuantumLauncher root folder.
pub fn get_launcher_dir() -> Result<PathBuf, IoError> {
    let config_directory = dirs::config_dir().ok_or(IoError::ConfigDirNotFound)?;
    let launcher_directory = config_directory.join("QuantumLauncher");
    std::fs::create_dir_all(&launcher_directory).path(&launcher_directory)?;

    Ok(launcher_directory)
}

pub fn is_new_user() -> bool {
    let Some(config_directory) = dirs::config_dir() else {
        return false;
    };
    let launcher_directory = config_directory.join("QuantumLauncher");
    !launcher_directory.exists()
}

/// Returns the path to `.minecraft` folder containing the game files.
pub fn get_dot_minecraft_dir(selection: &InstanceSelection) -> Result<PathBuf, IoError> {
    let launcher_dir = get_launcher_dir()?;
    Ok(match selection {
        InstanceSelection::Instance(name) => {
            launcher_dir.join("instances").join(name).join(".minecraft")
        }
        InstanceSelection::Server(name) => launcher_dir.join("servers").join(name),
    })
}

/// Returns the path to the instance directory containing
/// QuantumLauncher-specific files.
pub fn get_instance_dir(selection: &InstanceSelection) -> Result<PathBuf, IoError> {
    let launcher_dir = get_launcher_dir()?;
    Ok(match selection {
        InstanceSelection::Instance(name) => launcher_dir.join("instances").join(name),
        InstanceSelection::Server(name) => launcher_dir.join("servers").join(name),
    })
}

/// Downloads a file from the given URL into a `String`.
///
/// # Arguments
/// - `client`: the reqwest client to use for the request
/// - `url`: the URL to download from
/// - `user_agent`: whether to use the quantum launcher
///   user agent (required for modrinth)
pub async fn download_file_to_string(
    client: &Client,
    url: &str,
    user_agent: bool,
) -> Result<String, RequestError> {
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

/// Downloads a file from the given URL into a `Vec<u8>`.
///
/// # Arguments
/// - `client`: the reqwest client to use for the request
/// - `url`: the URL to download from
/// - `user_agent`: whether to use the quantum launcher
///   user agent (required for modrinth)
pub async fn download_file_to_bytes(
    client: &Client,
    url: &str,
    user_agent: bool,
) -> Result<Vec<u8>, RequestError> {
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

#[derive(Debug)]
pub enum RequestError {
    DownloadError {
        code: reqwest::StatusCode,
        url: reqwest::Url,
    },
    ReqwestError(reqwest::Error),
}

impl From<reqwest::Error> for RequestError {
    fn from(value: reqwest::Error) -> Self {
        Self::ReqwestError(value)
    }
}

impl Display for RequestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "could not send request: ")?;
        match self {
            RequestError::DownloadError { code, url } => {
                write!(f, "download error with code {code}, url {url}")
            }
            RequestError::ReqwestError(err) => {
                write!(f, "reqwest library error: {err}")
            }
        }
    }
}

/// Sets the executable bit on a file.
///
/// This makes a file executable on Unix systems,
/// ie. it can be run as a program.
#[cfg(target_family = "unix")]
pub fn set_executable(path: &Path) -> Result<(), IoError> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(path).path(path)?.permissions();
    perms.set_mode(0o755); // rwxr-xr-x
    std::fs::set_permissions(path, perms).path(path)
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
