use std::{collections::HashMap, fmt::Display, path::Path};

use ql_core::{
    file_utils, info, json::VersionDetails, pt, IntoIoError, IoError, JsonFileError, RequestError,
};
use serde::{Deserialize, Serialize};

use crate::{loaders::change_instance_type, mod_manager::Loader};

#[derive(Serialize, Deserialize)]
pub struct PaperVersions {
    latest: String,
    versions: HashMap<String, String>,
}

const PAPER_VERSIONS_URL: &str = "https://qing762.is-a.dev/api/papermc";

/// Moves a directory from `old_path` to `new_path`.
/// If `new_path` exists, it will be deleted before the move.
///
/// # Arguments
///
/// * `old_path` - The path to the directory you want to move.
/// * `new_path` - The destination path.
///
/// # Errors
///
/// Returns an `IoError` if any operation fails.
async fn move_dir(old_path: &Path, new_path: &Path) -> Result<(), IoError> {
    // Check if the new_path exists, and remove it if it does
    if new_path.exists() {
        tokio::fs::remove_dir_all(new_path).await.path(new_path)?;
    }

    copy_recursive(old_path, new_path).await?;

    // Remove the original directory
    tokio::fs::remove_dir_all(old_path).await.path(old_path)?;

    Ok(())
}

async fn copy_recursive(src: &Path, dst: &Path) -> Result<(), IoError> {
    tokio::fs::create_dir_all(dst).await.path(dst)?;

    let mut dir = tokio::fs::read_dir(src).await.path(src)?;
    while let Ok(Some(entry)) = dir.next_entry().await {
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            Box::pin(copy_recursive(&src_path, &dst_path)).await?;
        } else {
            tokio::fs::copy(&src_path, &dst_path).await.path(src_path)?;
        }
    }

    Ok(())
}

pub async fn uninstall_w(instance_name: String) -> Result<Loader, String> {
    uninstall(&instance_name)
        .await
        .map_err(|err| err.to_string())
        .map(|()| Loader::Paper)
}

pub async fn uninstall(instance_name: &str) -> Result<(), PaperInstallerError> {
    let server_dir = file_utils::get_launcher_dir()?
        .join("servers")
        .join(instance_name);

    let jar_path = server_dir.join("paper_server.jar");
    tokio::fs::remove_file(&jar_path).await.path(jar_path)?;

    // Paper stores Nether and End dimension worlds
    // in a separate directory, so we migrate it back.

    move_dir(
        &server_dir.join("world_nether/DIM-1"),
        &server_dir.join("world/DIM-1"),
    )
    .await?;
    move_dir(
        &server_dir.join("world_the_end/DIM1"),
        &server_dir.join("world/DIM1"),
    )
    .await?;

    let path = server_dir.join("world_nether");
    tokio::fs::remove_dir_all(&path).await.path(path)?;
    let path = server_dir.join("world_the_end");
    tokio::fs::remove_dir_all(&path).await.path(path)?;

    change_instance_type(&server_dir, "Vanilla".to_owned()).await?;

    Ok(())
}

pub async fn install_w(instance_name: String) -> Result<(), String> {
    install(&instance_name).await.map_err(|err| err.to_string())
}

pub async fn install(instance_name: &str) -> Result<(), PaperInstallerError> {
    info!("Installing Paper");
    let client = reqwest::Client::new();
    pt!("Getting version list");
    let paper_versions =
        file_utils::download_file_to_string(&client, PAPER_VERSIONS_URL, false).await?;
    let paper_version: PaperVersions = serde_json::from_str(&paper_versions)?;

    let server_dir = file_utils::get_launcher_dir()?
        .join("servers")
        .join(instance_name);
    let json_path = server_dir.join("details.json");
    let json = tokio::fs::read_to_string(&json_path)
        .await
        .path(json_path)?;
    let json: VersionDetails = serde_json::from_str(&json)?;

    let url = paper_version
        .versions
        .get(&json.id)
        .ok_or(PaperInstallerError::NoMatchingVersionFound(json.id.clone()))?;

    pt!("Downloading jar");
    let jar_file = file_utils::download_file_to_bytes(&client, url, true).await?;
    let jar_path = server_dir.join("paper_server.jar");
    tokio::fs::write(&jar_path, &jar_file)
        .await
        .path(jar_path)?;

    change_instance_type(&server_dir, "Paper".to_owned()).await?;

    pt!("Done");
    Ok(())
}

pub enum PaperInstallerError {
    Request(RequestError),
    Io(IoError),
    Serde(serde_json::Error),
    NoMatchingVersionFound(String),
}

impl Display for PaperInstallerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "could not install paper: ")?;
        match self {
            PaperInstallerError::Request(err) => write!(f, "{err}"),
            PaperInstallerError::Io(err) => write!(f, "{err}"),
            PaperInstallerError::Serde(err) => write!(f, "(json) {err}"),
            PaperInstallerError::NoMatchingVersionFound(version) => {
                write!(f, "no compatible paper version found for version {version}")
            }
        }
    }
}

impl From<JsonFileError> for PaperInstallerError {
    fn from(value: JsonFileError) -> Self {
        match value {
            JsonFileError::SerdeError(err) => Self::Serde(err),
            JsonFileError::Io(err) => Self::Io(err),
        }
    }
}

impl From<RequestError> for PaperInstallerError {
    fn from(value: RequestError) -> Self {
        Self::Request(value)
    }
}

impl From<IoError> for PaperInstallerError {
    fn from(value: IoError) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for PaperInstallerError {
    fn from(value: serde_json::Error) -> Self {
        Self::Serde(value)
    }
}
