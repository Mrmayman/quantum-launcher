use std::{
    fmt::Display,
    path::PathBuf,
    sync::mpsc::{SendError, Sender},
};

use ql_instances::{
    error::IoError,
    file_utils::{self, RequestError},
    info, io_err,
    json_structs::{json_fabric::FabricJSON, json_version::VersionDetails},
};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::{change_instance_type, ChangeConfigError};

const FABRIC_URL: &str = "https://meta.fabricmc.net";

async fn download_file_to_string(client: &Client, url: &str) -> Result<String, RequestError> {
    file_utils::download_file_to_string(client, &format!("{FABRIC_URL}/{url}"), false).await
}

pub async fn get_list_of_versions() -> Result<Vec<FabricVersion>, String> {
    let client = Client::new();
    // The first one is the latest version.
    let version_list = download_file_to_string(&client, "v2/versions/loader")
        .await
        .map_err(|err| err.to_string())?;

    serde_json::from_str(&version_list).map_err(|err| err.to_string())
}

pub fn get_url(name: &str) -> String {
    let parts: Vec<&str> = name.split(':').collect();
    format!(
        "{}/{}/{}/{}-{}.jar",
        parts[0].replace('.', "/"),
        parts[1],
        parts[2],
        parts[1],
        parts[2],
    )
}

pub enum FabricInstallProgress {
    P1Start,
    P2Library { done: usize, out_of: usize },
    P3Done,
}

pub async fn install(
    loader_version: &str,
    instance_name: &str,
    progress: Option<Sender<FabricInstallProgress>>,
) -> Result<(), FabricInstallError> {
    let client = Client::new();

    let launcher_dir = file_utils::get_launcher_dir()?;
    let instance_dir = launcher_dir.join("instances").join(instance_name);

    let lock_path = instance_dir.join("fabric.lock");
    tokio::fs::write(
        &lock_path,
        "If you see this, fabric was not installed correctly.",
    )
    .await
    .map_err(io_err!(lock_path))?;

    let libraries_dir = instance_dir.join("libraries");

    let version_json_path = instance_dir.join("details.json");
    let version_json = tokio::fs::read_to_string(&version_json_path)
        .await
        .map_err(io_err!(version_json_path))?;
    let version_json: VersionDetails = serde_json::from_str(&version_json)?;

    let game_version = version_json.id;

    let json_path = instance_dir.join("fabric.json");
    let json_url = format!("v2/versions/loader/{game_version}/{loader_version}/profile/json");
    let json = download_file_to_string(&client, &json_url).await?;
    tokio::fs::write(&json_path, &json)
        .await
        .map_err(io_err!(json_path))?;

    let json: FabricJSON = serde_json::from_str(&json)?;

    info!("Started installing fabric");

    if let Some(progress) = &progress {
        progress.send(FabricInstallProgress::P1Start)?;
    }

    let number_of_libraries = json.libraries.len();
    for (i, library) in json.libraries.iter().enumerate() {
        println!(
            "Downloading fabric library ({} / {number_of_libraries}) {}",
            i + 1,
            library.name
        );

        if let Some(progress) = &progress {
            progress.send(FabricInstallProgress::P2Library {
                done: i + 1,
                out_of: number_of_libraries,
            })?;
        }

        let path = libraries_dir.join(library.get_path());
        let url = format!("{}{}", library.url, get_url(&library.name));

        let bytes = file_utils::download_file_to_bytes(&client, &url, false).await?;

        let parent_dir = path
            .parent()
            .ok_or(FabricInstallError::PathBufParentError(path.clone()))?;
        tokio::fs::create_dir_all(parent_dir)
            .await
            .map_err(io_err!(parent_dir))?;
        tokio::fs::write(&path, &bytes)
            .await
            .map_err(io_err!(path))?;
    }

    change_instance_type(&instance_dir, "Fabric".to_owned())?;

    if let Some(progress) = &progress {
        progress.send(FabricInstallProgress::P3Done)?;
    }

    tokio::fs::remove_file(&lock_path)
        .await
        .map_err(io_err!(lock_path))?;

    info!("Finished installing fabric");

    Ok(())
}

pub async fn uninstall(instance_name: &str) -> Result<(), FabricInstallError> {
    let launcher_dir = file_utils::get_launcher_dir()?;
    let instance_dir = launcher_dir.join("instances").join(instance_name);

    let lock_path = instance_dir.join("fabric_uninstall.lock");
    tokio::fs::write(
        &lock_path,
        "If you see this, fabric was not uninstalled correctly.",
    )
    .await
    .map_err(io_err!(lock_path))?;

    let fabric_json_path = instance_dir.join("fabric.json");
    let fabric_json = tokio::fs::read_to_string(&fabric_json_path)
        .await
        .map_err(io_err!(fabric_json_path))?;
    let fabric_json: FabricJSON = serde_json::from_str(&fabric_json)?;

    tokio::fs::remove_file(&fabric_json_path)
        .await
        .map_err(io_err!(fabric_json_path))?;

    let libraries_dir = instance_dir.join("libraries");

    for library in &fabric_json.libraries {
        let library_path = libraries_dir.join(library.get_path());
        tokio::fs::remove_file(&library_path)
            .await
            .map_err(io_err!(library_path))?;
    }

    change_instance_type(&instance_dir, "Vanilla".to_owned())?;

    tokio::fs::remove_file(&lock_path)
        .await
        .map_err(io_err!(lock_path))?;
    Ok(())
}

pub async fn uninstall_wrapped(instance_name: String) -> Result<(), String> {
    uninstall(&instance_name)
        .await
        .map_err(|err| err.to_string())
}

pub async fn install_wrapped(
    loader_version: String,
    instance_name: String,
    progress: Option<Sender<FabricInstallProgress>>,
) -> Result<(), String> {
    install(&loader_version, &instance_name, progress)
        .await
        .map_err(|err| err.to_string())
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FabricVersion {
    pub separator: String,
    pub build: usize,
    pub maven: String,
    pub version: String,
    pub stable: bool,
}

#[derive(Debug)]
pub enum FabricInstallError {
    Io(IoError),
    Json(serde_json::Error),
    RequestError(RequestError),
    Send(SendError<FabricInstallProgress>),
    ChangeConfigError(ChangeConfigError),
    PathBufParentError(PathBuf),
}

impl From<IoError> for FabricInstallError {
    fn from(value: IoError) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for FabricInstallError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

impl From<RequestError> for FabricInstallError {
    fn from(value: RequestError) -> Self {
        Self::RequestError(value)
    }
}

impl From<SendError<FabricInstallProgress>> for FabricInstallError {
    fn from(value: SendError<FabricInstallProgress>) -> Self {
        Self::Send(value)
    }
}

impl From<ChangeConfigError> for FabricInstallError {
    fn from(value: ChangeConfigError) -> Self {
        Self::ChangeConfigError(value)
    }
}

impl Display for FabricInstallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "error installing fabric: ")?;
        match self {
            // Look, I'm not the best at programming.
            FabricInstallError::Io(err) => write!(f, "(system io) {err}"),
            FabricInstallError::Json(err) => {
                write!(f, "(parsing json) {err}")
            }
            FabricInstallError::RequestError(err) => {
                write!(f, "(downloading file) {err}")
            }
            FabricInstallError::Send(err) => {
                write!(f, "(sending message) {err}")
            }
            FabricInstallError::ChangeConfigError(err) => {
                write!(f, "could not change instance config: {err}")
            }
            FabricInstallError::PathBufParentError(path_buf) => {
                write!(f, "could not get parent of pathbuf: {path_buf:?}")
            }
        }
    }
}
