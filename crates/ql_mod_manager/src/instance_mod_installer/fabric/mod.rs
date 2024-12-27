use std::sync::mpsc::Sender;

use error::FabricInstallError;
use ql_core::{
    file_utils, info, io_err,
    json::{fabric::FabricJSON, version::VersionDetails},
    InstanceSelection, JsonFileError, RequestError,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use version_compare::compare_versions;

use crate::mod_manager::Loader;

use super::change_instance_type;

mod error;
mod make_launch_jar;
mod version_compare;

const FABRIC_URL: &str = "https://meta.fabricmc.net";

async fn download_file_to_string(client: &Client, url: &str) -> Result<String, RequestError> {
    file_utils::download_file_to_string(client, &format!("{FABRIC_URL}/{url}"), false).await
}

async fn get_version_json(
    instance_name: &InstanceSelection,
) -> Result<VersionDetails, JsonFileError> {
    let version_json_path = file_utils::get_instance_dir(instance_name)?.join("details.json");
    let version_json = tokio::fs::read_to_string(&version_json_path)
        .await
        .map_err(io_err!(version_json_path))?;
    Ok(serde_json::from_str(&version_json)?)
}

pub async fn get_list_of_versions_w(
    instance_name: InstanceSelection,
) -> Result<Vec<FabricVersionListItem>, String> {
    get_list_of_versions(&instance_name)
        .await
        .map_err(|err| err.to_string())
}

pub async fn get_list_of_versions(
    instance_name: &InstanceSelection,
) -> Result<Vec<FabricVersionListItem>, FabricInstallError> {
    let client = Client::new();

    let version_json = get_version_json(instance_name).await?;

    // The first one is the latest version.
    let version_list =
        download_file_to_string(&client, &format!("v2/versions/loader/{}", version_json.id))
            .await?;

    Ok(serde_json::from_str(&version_list)?)
}

pub enum FabricInstallProgress {
    P1Start,
    P2Library {
        done: usize,
        out_of: usize,
        message: String,
    },
    P3Done,
}

pub async fn install_server_w(
    loader_version: String,
    server_name: String,
    progress: Option<Sender<FabricInstallProgress>>,
) -> Result<(), String> {
    install_server(&loader_version, &server_name, progress)
        .await
        .map_err(|err| err.to_string())
}

pub async fn install_server(
    loader_version: &str,
    server_name: &str,
    progress: Option<Sender<FabricInstallProgress>>,
) -> Result<(), FabricInstallError> {
    if let Some(progress) = &progress {
        progress.send(FabricInstallProgress::P1Start)?;
    }

    let server_dir = file_utils::get_launcher_dir()?
        .join("servers")
        .join(server_name);

    let libraries_dir = server_dir.join("libraries");
    tokio::fs::create_dir_all(&libraries_dir)
        .await
        .map_err(io_err!(libraries_dir))?;

    let version_json_path = server_dir.join("details.json");
    let version_json = tokio::fs::read_to_string(&version_json_path)
        .await
        .map_err(io_err!(version_json_path))?;
    let version_json: VersionDetails = serde_json::from_str(&version_json)?;

    let game_version = version_json.id;
    let client = Client::new();

    let json_url = format!("v2/versions/loader/{game_version}/{loader_version}/server/json");
    let json = download_file_to_string(&client, &json_url).await?;

    let json_path = server_dir.join("fabric.json");
    tokio::fs::write(&json_path, &json)
        .await
        .map_err(io_err!(json_path))?;

    let json: FabricJSON = serde_json::from_str(&json)?;

    let number_of_libraries = json.libraries.len();
    let mut library_files = Vec::new();
    for (i, library) in json.libraries.iter().enumerate() {
        send_progress(i, library, progress.as_ref(), number_of_libraries)?;

        let library_path = libraries_dir.join(library.get_path());

        let library_parent_dir = library_path.parent().unwrap();
        library_files.push(library_path.clone());
        tokio::fs::create_dir_all(&library_parent_dir)
            .await
            .map_err(io_err!(library_parent_dir))?;

        let url = library.get_url();
        let file = file_utils::download_file_to_bytes(&client, &url, false).await?;
        tokio::fs::write(&library_path, &file)
            .await
            .map_err(io_err!(library_path))?;
    }

    let shade_libraries = compare_versions(loader_version, "0.12.5").is_le();
    let launch_jar = server_dir.join("fabric-server-launch.jar");

    info!("Making launch jar");
    make_launch_jar::make_launch_jar(
        &launch_jar,
        &json.mainClass,
        &library_files,
        shade_libraries,
    )
    .await?;

    change_instance_type(&server_dir, "Fabric".to_owned())?;

    if let Some(progress) = &progress {
        progress.send(FabricInstallProgress::P3Done)?;
    }

    info!("Finished installing fabric");

    Ok(())
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

    info!("Started installing fabric: {game_version}, {loader_version}");

    if let Some(progress) = &progress {
        progress.send(FabricInstallProgress::P1Start)?;
    }

    let number_of_libraries = json.libraries.len();
    for (i, library) in json.libraries.iter().enumerate() {
        send_progress(i, library, progress.as_ref(), number_of_libraries)?;

        let path = libraries_dir.join(library.get_path());
        let url = library.get_url();

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

fn send_progress(
    i: usize,
    library: &ql_core::json::fabric::Library,
    progress: Option<&Sender<FabricInstallProgress>>,
    number_of_libraries: usize,
) -> Result<(), FabricInstallError> {
    let message = format!(
        "Downloading fabric library ({} / {number_of_libraries}) {}",
        i + 1,
        library.name
    );
    info!("{message}");
    if let Some(progress) = progress {
        progress.send(FabricInstallProgress::P2Library {
            done: i + 1,
            out_of: number_of_libraries,
            message,
        })?;
    }
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

pub async fn uninstall_client_w(instance_name: String) -> Result<Loader, String> {
    uninstall(&instance_name)
        .await
        .map_err(|err| err.to_string())
        .map(|()| Loader::Fabric)
}

pub async fn install_w(
    loader_version: String,
    instance_name: InstanceSelection,
    progress: Option<Sender<FabricInstallProgress>>,
) -> Result<(), String> {
    match instance_name {
        InstanceSelection::Instance(n) => install_client_w(loader_version, n, progress).await,
        InstanceSelection::Server(n) => install_server_w(loader_version, n, progress).await,
    }
}

pub async fn install_client_w(
    loader_version: String,
    instance_name: String,
    progress: Option<Sender<FabricInstallProgress>>,
) -> Result<(), String> {
    install(&loader_version, &instance_name, progress)
        .await
        .map_err(|err| err.to_string())
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FabricVersionListItem {
    pub loader: FabricVersion,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FabricVersion {
    pub separator: String,
    pub build: usize,
    pub maven: String,
    pub version: String,
    pub stable: bool,
}
