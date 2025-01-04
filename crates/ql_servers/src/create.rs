use std::sync::mpsc::Sender;

use ql_core::{
    file_utils, info,
    json::{instance_config::InstanceConfigJson, manifest::Manifest, version::VersionDetails},
    IntoIoError, ListEntry,
};

use crate::ServerError;

pub async fn create_server_w(
    name: String,
    version: ListEntry,
    sender: Option<Sender<ServerCreateProgress>>,
) -> Result<String, String> {
    create_server(&name, version, sender)
        .await
        .map_err(|n| n.to_string())
        .map(|()| name)
}

pub enum ServerCreateProgress {
    P1DownloadingManifest,
    P2DownloadingVersionJson,
    P3DownloadingServerJar,
}

pub async fn create_server(
    name: &str,
    version: ListEntry,
    sender: Option<Sender<ServerCreateProgress>>,
) -> Result<(), ServerError> {
    let client = reqwest::Client::new();
    info!("Creating server: Downloading Manifest");
    send_progress(&sender, ServerCreateProgress::P1DownloadingManifest);
    let manifest = Manifest::download().await?;

    let server_dir = get_server_dir(name).await?;
    let server_jar_path = server_dir.join("server.jar");

    info!("Downloading version JSON");
    send_progress(&sender, ServerCreateProgress::P2DownloadingVersionJson);
    let version = manifest
        .find_name(&version.0)
        .ok_or(ServerError::VersionNotFoundInManifest(version.0.to_owned()))?;
    let version_json = file_utils::download_file_to_string(&client, &version.url, false).await?;
    let version_json: VersionDetails = serde_json::from_str(&version_json)?;

    let Some(server) = &version_json.downloads.server else {
        return Err(ServerError::NoServerDownload);
    };
    info!("Downloading server jar");
    send_progress(&sender, ServerCreateProgress::P3DownloadingServerJar);
    let server_jar = file_utils::download_file_to_bytes(&client, &server.url, false).await?;

    let is_classic_server = version.phase.as_deref() == Some("classic");
    if is_classic_server {
        zip_extract::extract(std::io::Cursor::new(server_jar), &server_dir, true)?;
        let old_path = server_dir.join("minecraft-server.jar");
        tokio::fs::rename(&old_path, &server_jar_path)
            .await
            .path(old_path)?;
    } else {
        tokio::fs::write(&server_jar_path, server_jar)
            .await
            .path(server_jar_path)?;
    }

    let version_json_path = server_dir.join("details.json");
    tokio::fs::write(&version_json_path, serde_json::to_string(&version_json)?)
        .await
        .path(version_json_path)?;

    let eula_path = server_dir.join("eula.txt");
    tokio::fs::write(&eula_path, "eula=true\n")
        .await
        .path(eula_path)?;

    let server_config = InstanceConfigJson {
        mod_type: "Vanilla".to_owned(),
        java_override: None,
        ram_in_mb: 2048,
        enable_logger: Some(true),
        java_args: None,
        game_args: None,
        is_classic_server: is_classic_server.then_some(true),
    };

    let server_config_path = server_dir.join("config.json");
    tokio::fs::write(&server_config_path, serde_json::to_string(&server_config)?)
        .await
        .path(server_config_path)?;

    let mods_dir = server_dir.join("mods");
    tokio::fs::create_dir(&mods_dir).await.path(mods_dir)?;

    Ok(())
}

fn send_progress(sender: &Option<Sender<ServerCreateProgress>>, msg: ServerCreateProgress) {
    if let Some(sender) = sender {
        sender.send(msg).unwrap();
    }
}

async fn get_server_dir(name: &str) -> Result<std::path::PathBuf, ServerError> {
    let launcher_dir = file_utils::get_launcher_dir()?;
    let server_dir = launcher_dir.join("servers").join(name);
    if server_dir.exists() {
        return Err(ServerError::ServerAlreadyExists);
    }
    tokio::fs::create_dir_all(&server_dir)
        .await
        .path(&server_dir)?;
    Ok(server_dir)
}

pub fn delete_server(name: &str) -> Result<(), String> {
    let launcher_dir = file_utils::get_launcher_dir().map_err(|n| n.to_string())?;
    let server_dir = launcher_dir.join("servers").join(name);
    std::fs::remove_dir_all(&server_dir)
        .path(server_dir)
        .map_err(|n| n.to_string())?;

    Ok(())
}
