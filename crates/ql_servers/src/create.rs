use std::sync::mpsc::Sender;

use omniarchive_api::{ListEntry, MinecraftVersionCategory};
use ql_core::{
    file_utils, info, io_err,
    json::{instance_config::InstanceConfigJson, manifest::Manifest, version::VersionDetails},
};

use crate::ServerError;

pub async fn create_server_wrapped(
    name: String,
    version: ListEntry,
    sender: Option<Sender<ServerCreateProgress>>,
) -> Result<(), String> {
    create_server(&name, version, sender)
        .await
        .map_err(|n| n.to_string())
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
    if let Some(sender) = &sender {
        sender
            .send(ServerCreateProgress::P1DownloadingManifest)
            .unwrap();
    }
    let manifest = Manifest::download().await?;

    let (server_jar, version_json) = match version {
        ListEntry::Normal(version) => {
            let version = manifest
                .find_name(&version)
                .ok_or(ServerError::VersionNotFoundInManifest(version))?;

            info!("Downloading version JSON");
            if let Some(sender) = &sender {
                sender
                    .send(ServerCreateProgress::P2DownloadingVersionJson)
                    .unwrap();
            }
            let version_json =
                file_utils::download_file_to_string(&client, &version.url, false).await?;
            let version_json: VersionDetails = serde_json::from_str(&version_json)?;

            let Some(server) = &version_json.downloads.server else {
                return Err(ServerError::NoServerDownload);
            };

            info!("Downloading server jar");
            if let Some(sender) = &sender {
                sender
                    .send(ServerCreateProgress::P3DownloadingServerJar)
                    .unwrap();
            }
            let server_jar =
                file_utils::download_file_to_bytes(&client, &server.url, false).await?;

            (server_jar, version_json)
        }
        ListEntry::Omniarchive {
            category,
            name,
            url,
        } => {
            let version = match category {
                MinecraftVersionCategory::PreClassic => manifest.find_fuzzy(&name, "rd-"),
                MinecraftVersionCategory::Classic => manifest.find_fuzzy(&name, "c0."),
                MinecraftVersionCategory::Alpha => manifest.find_fuzzy(&name, "a1."),
                MinecraftVersionCategory::Beta => manifest.find_fuzzy(&name, "b1."),
                MinecraftVersionCategory::Indev => manifest.find_name("c0.30_01c"),
                MinecraftVersionCategory::Infdev => manifest.find_name("inf-20100618"),
            }
            .ok_or(ServerError::VersionNotFoundInManifest(name.to_owned()))?;

            info!("Downloading version JSON");
            if let Some(sender) = &sender {
                sender
                    .send(ServerCreateProgress::P2DownloadingVersionJson)
                    .unwrap();
            }
            let version_json =
                file_utils::download_file_to_string(&client, &version.url, false).await?;
            let version_json: VersionDetails = serde_json::from_str(&version_json)?;

            info!("Downloading server jar");
            if let Some(sender) = &sender {
                sender
                    .send(ServerCreateProgress::P3DownloadingServerJar)
                    .unwrap();
            }
            let server_jar = file_utils::download_file_to_bytes(&client, &url, false).await?;

            (server_jar, version_json)
        }
    };

    let launcher_dir = file_utils::get_launcher_dir()?;
    let server_dir = launcher_dir.join("servers").join(name);
    tokio::fs::create_dir_all(&server_dir)
        .await
        .map_err(io_err!(server_dir))?;

    let server_jar_path = server_dir.join("server.jar");
    tokio::fs::write(&server_jar_path, server_jar)
        .await
        .map_err(io_err!(server_jar_path))?;

    let version_json_path = server_dir.join("details.json");
    tokio::fs::write(&version_json_path, serde_json::to_string(&version_json)?)
        .await
        .map_err(io_err!(version_json_path))?;

    let server_config = InstanceConfigJson {
        mod_type: "Vanilla".to_owned(),
        java_override: None,
        ram_in_mb: 2048,
        enable_logger: Some(true),
        java_args: None,
        game_args: None,
    };

    let server_config_path = server_dir.join("config.json");
    tokio::fs::write(&server_config_path, serde_json::to_string(&server_config)?)
        .await
        .map_err(io_err!(server_config_path))?;

    Ok(())
}
