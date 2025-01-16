use std::{path::Path, sync::mpsc::Sender};

use error::FabricInstallError;
use ql_core::{
    file_utils, info,
    json::{fabric::FabricJSON, version::VersionDetails},
    GenericProgress, InstanceSelection, IntoIoError, JsonFileError, RequestError,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use version_compare::compare_versions;

use super::change_instance_type;

mod error;
mod make_launch_jar;
mod uninstall;
pub use uninstall::{
    uninstall_client, uninstall_client_w, uninstall_server, uninstall_server_w, uninstall_w,
};
mod version_compare;

const FABRIC_URL: &str = "https://meta.fabricmc.net/v2";
const QUILT_URL: &str = "https://meta.quiltmc.org/v3";

async fn download_file_to_string(
    client: &Client,
    url: &str,
    is_quilt: bool,
) -> Result<String, RequestError> {
    file_utils::download_file_to_string(
        client,
        &format!("{}/{url}", if is_quilt { QUILT_URL } else { FABRIC_URL }),
        false,
    )
    .await
}

async fn get_version_json(
    instance_name: &InstanceSelection,
) -> Result<VersionDetails, JsonFileError> {
    let version_json_path = file_utils::get_instance_dir(instance_name)?.join("details.json");
    let version_json = tokio::fs::read_to_string(&version_json_path)
        .await
        .path(version_json_path)?;
    Ok(serde_json::from_str(&version_json)?)
}

pub async fn get_list_of_versions_w(
    instance_name: InstanceSelection,
    is_quilt: bool,
) -> Result<Vec<FabricVersionListItem>, String> {
    get_list_of_versions(&instance_name, is_quilt)
        .await
        .map_err(|err| err.to_string())
}

pub async fn get_list_of_versions(
    instance_name: &InstanceSelection,
    is_quilt: bool,
) -> Result<Vec<FabricVersionListItem>, FabricInstallError> {
    let client = Client::new();

    let version_json = get_version_json(instance_name).await?;

    // The first one is the latest version.
    let version_list = download_file_to_string(
        &client,
        &format!("/versions/loader/{}", version_json.id),
        is_quilt,
    )
    .await?;

    Ok(serde_json::from_str(&version_list)?)
}

pub async fn install_server_w(
    loader_version: String,
    server_name: String,
    progress: Option<Sender<GenericProgress>>,
    is_quilt: bool,
) -> Result<(), String> {
    install_server(&loader_version, &server_name, progress, is_quilt)
        .await
        .map_err(|err| err.to_string())
}

pub async fn install_server(
    loader_version: &str,
    server_name: &str,
    progress: Option<Sender<GenericProgress>>,
    is_quilt: bool,
) -> Result<(), FabricInstallError> {
    let loader_name = if is_quilt { "Quilt" } else { "Fabric" };

    if let Some(progress) = &progress {
        progress.send(GenericProgress::default())?;
    }

    let server_dir = file_utils::get_launcher_dir()?
        .join("servers")
        .join(server_name);

    let libraries_dir = server_dir.join("libraries");
    tokio::fs::create_dir_all(&libraries_dir)
        .await
        .path(&libraries_dir)?;

    let version_json_path = server_dir.join("details.json");
    let version_json = tokio::fs::read_to_string(&version_json_path)
        .await
        .path(version_json_path)?;
    let version_json: VersionDetails = serde_json::from_str(&version_json)?;

    let game_version = version_json.id;
    let client = Client::new();

    let json_url = format!("/versions/loader/{game_version}/{loader_version}/server/json");
    let json = download_file_to_string(&client, &json_url, is_quilt).await?;

    let json_path = server_dir.join("fabric.json");
    tokio::fs::write(&json_path, &json).await.path(json_path)?;

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
            .path(library_parent_dir)?;

        let url = library.get_url();
        let file = file_utils::download_file_to_bytes(&client, &url, false).await?;
        tokio::fs::write(&library_path, &file)
            .await
            .path(library_path)?;
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

    change_instance_type(&server_dir, loader_name.to_owned()).await?;

    if let Some(progress) = &progress {
        progress.send(GenericProgress::finished())?;
    }

    info!("Finished installing {loader_name}");

    Ok(())
}

pub async fn install_client(
    loader_version: &str,
    instance_name: &str,
    progress: Option<Sender<GenericProgress>>,
    is_quilt: bool,
) -> Result<(), FabricInstallError> {
    let loader_name = if is_quilt { "Quilt" } else { "Fabric" };
    let client = Client::new();

    let launcher_dir = file_utils::get_launcher_dir()?;
    let instance_dir = launcher_dir.join("instances").join(instance_name);

    migrate_index_file(&instance_dir)?;

    let lock_path = instance_dir.join("fabric.lock");
    tokio::fs::write(
        &lock_path,
        "If you see this, fabric/quilt was not installed correctly.",
    )
    .await
    .path(&lock_path)?;

    let libraries_dir = instance_dir.join("libraries");

    let version_json_path = instance_dir.join("details.json");
    let version_json = tokio::fs::read_to_string(&version_json_path)
        .await
        .path(version_json_path)?;
    let version_json: VersionDetails = serde_json::from_str(&version_json)?;

    let game_version = version_json.id;

    let json_path = instance_dir.join("fabric.json");
    let json_url = format!("/versions/loader/{game_version}/{loader_version}/profile/json");
    let json = download_file_to_string(&client, &json_url, is_quilt).await?;
    tokio::fs::write(&json_path, &json).await.path(json_path)?;

    let json: FabricJSON = serde_json::from_str(&json)?;

    info!("Started installing {loader_name}: {game_version}, {loader_version}");

    if let Some(progress) = &progress {
        progress.send(GenericProgress::default())?;
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
            .path(parent_dir)?;
        tokio::fs::write(&path, &bytes).await.path(path)?;
    }

    change_instance_type(&instance_dir, loader_name.to_owned()).await?;

    if let Some(progress) = &progress {
        progress.send(GenericProgress::default())?;
    }

    tokio::fs::remove_file(&lock_path).await.path(lock_path)?;

    info!("Finished installing {loader_name}",);

    Ok(())
}

fn migrate_index_file(instance_dir: &Path) -> Result<(), FabricInstallError> {
    let old_index_dir = instance_dir.join(".minecraft/mods/index.json");
    let new_index_dir = instance_dir.join(".minecraft/mod_index.json");
    if old_index_dir.exists() {
        let index = std::fs::read_to_string(&old_index_dir).path(&old_index_dir)?;

        std::fs::remove_file(&old_index_dir).path(old_index_dir)?;
        std::fs::write(&new_index_dir, &index).path(new_index_dir)?;
    }
    Ok(())
}

fn send_progress(
    i: usize,
    library: &ql_core::json::fabric::Library,
    progress: Option<&Sender<GenericProgress>>,
    number_of_libraries: usize,
) -> Result<(), FabricInstallError> {
    let message = format!(
        "Downloading library ({} / {number_of_libraries}) {}",
        i + 1,
        library.name
    );
    info!("{message}");
    if let Some(progress) = progress {
        progress.send(GenericProgress {
            done: i + 1,
            total: number_of_libraries,
            message: Some(message),
            has_finished: false,
        })?;
    }
    Ok(())
}

/// Installs Fabric or Quilt to the given instance.
///
/// # Arguments
/// - `loader_version` - The version of the loader to install.
/// - `instance_name` - The name of the instance to install to.
///   `InstanceSelection::Instance(n)` for a client instance,
///   `InstanceSelection::Server(n)` for a server instance.
/// - `progress` - A channel to send progress updates to.
/// - `is_quilt` - Whether to install Quilt instead of Fabric.
///   As much as people want you to think, Quilt is almost
///   identical to Fabric installer wise. So it's just a
///   matter of changing the URL.
pub async fn install_w(
    loader_version: String,
    instance_name: InstanceSelection,
    progress: Option<Sender<GenericProgress>>,
    is_quilt: bool,
) -> Result<(), String> {
    match instance_name {
        InstanceSelection::Instance(n) => {
            install_client_w(loader_version, n, progress, is_quilt).await
        }
        InstanceSelection::Server(n) => {
            install_server_w(loader_version, n, progress, is_quilt).await
        }
    }
}

pub async fn install_client_w(
    loader_version: String,
    instance_name: String,
    progress: Option<Sender<GenericProgress>>,
    is_quilt: bool,
) -> Result<(), String> {
    install_client(&loader_version, &instance_name, progress, is_quilt)
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
    pub stable: Option<bool>,
}
