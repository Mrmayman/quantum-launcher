use std::fmt::Display;

use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::{
    error::IoError,
    file_utils::{self, RequestError},
    io_err,
    json_structs::{
        json_fabric::FabricJSON, json_instance_config::InstanceConfigJson,
        json_version::VersionDetails,
    },
};

const FABRIC_URL: &str = "https://meta.fabricmc.net";

async fn download_file_to_string(client: &Client, url: &str) -> Result<String, RequestError> {
    file_utils::download_file_to_string(client, &format!("{FABRIC_URL}/{url}")).await
}

pub async fn get_list_of_versions() -> Result<Vec<FabricVersion>, String> {
    let client = Client::new();
    // The first one is the latest version.
    let version_list = download_file_to_string(&client, "v2/versions/loader")
        .await
        .map_err(|err| err.to_string())?;

    serde_json::from_str(&version_list).map_err(|err| err.to_string())
}

fn get_url(name: &str) -> String {
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

pub async fn install(loader_version: &str, instance_name: &str) -> Result<(), FabricInstallError> {
    let client = Client::new();

    let launcher_dir = file_utils::get_launcher_dir()?;
    let instance_dir = launcher_dir.join("instances").join(instance_name);
    let libraries_dir = instance_dir.join("libraries");

    let version_json_path = instance_dir.join("details.json");
    let version_json =
        std::fs::read_to_string(&version_json_path).map_err(io_err!(version_json_path))?;
    let version_json: VersionDetails = serde_json::from_str(&version_json)?;

    let game_version = version_json.id;

    let json_path = instance_dir.join("fabric.json");
    let json_url = format!("v2/versions/loader/{game_version}/{loader_version}/profile/json");
    let json = download_file_to_string(&client, &json_url).await?;
    std::fs::write(&json_path, &json).map_err(io_err!(json_path))?;

    let json: FabricJSON = serde_json::from_str(&json)?;

    for library in json.libraries {
        println!("[info] Downloading fabric library {}", library.name);

        let path = libraries_dir.join(library.get_path());
        let url = format!("{}{}", library.url, get_url(&library.name));

        let bytes = file_utils::download_file_to_bytes(&client, &url).await?;

        let parent_dir = path.parent().unwrap();
        std::fs::create_dir_all(parent_dir).map_err(io_err!(parent_dir))?;
        std::fs::write(&path, &bytes).map_err(io_err!(path))?;
    }

    let config_path = instance_dir.join("config.json");
    let config = std::fs::read_to_string(&config_path).map_err(io_err!(config_path))?;
    let mut config: InstanceConfigJson = serde_json::from_str(&config)?;

    config.mod_type = "Fabric".to_owned();

    let config = serde_json::to_string(&config)?;
    std::fs::write(&config_path, config).map_err(io_err!(config_path))?;

    Ok(())
}

pub async fn install_wrapped(loader_version: String, instance_name: String) -> Result<(), String> {
    install(&loader_version, &instance_name)
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

impl Display for FabricInstallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // Look, I'm not the best at programming.
            FabricInstallError::Io(err) => write!(f, "error installing fabric: {err}"),
            FabricInstallError::Json(err) => write!(f, "error installing fabric: {err}"),
            FabricInstallError::RequestError(err) => write!(f, "error installing fabric: {err}"),
        }
    }
}
