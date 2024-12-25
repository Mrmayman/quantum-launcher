use std::sync::mpsc::Sender;

use omniarchive_api::{ListEntry, MinecraftVersionCategory};
use ql_core::{
    file_utils, info, io_err,
    json::{
        instance_config::{InstanceConfigJson, OmniarchiveEntry},
        manifest::Manifest,
        version::VersionDetails,
    },
};

use crate::ServerError;

pub async fn create_server_wrapped(
    name: String,
    version: ListEntry,
    sender: Option<Sender<ServerCreateProgress>>,
) -> Result<String, String> {
    create_server(&name, version, sender)
        .await
        .map_err(|n| n.to_string())
        .map(|_| name)
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

    let launcher_dir = file_utils::get_launcher_dir()?;
    let server_dir = launcher_dir.join("servers").join(name);
    if server_dir.exists() {
        return Err(ServerError::ServerAlreadyExists);
    }

    tokio::fs::create_dir_all(&server_dir)
        .await
        .map_err(io_err!(server_dir))?;

    let server_jar_path = server_dir.join("server.jar");

    let mut is_classic_server = false;

    let (server_jar, version_json) = match &version {
        ListEntry::Normal(version) => {
            download_from_mojang(&manifest, version, &sender, &client).await?
        }
        ListEntry::Omniarchive {
            category,
            name,
            url,
        } => download_from_omniarchive(category, &manifest, name, &sender, &client, url).await?,
        ListEntry::OmniarchiveClassicZipServer { name, url } => {
            is_classic_server = true;

            if let Some(sender) = &sender {
                sender
                    .send(ServerCreateProgress::P3DownloadingServerJar)
                    .unwrap();
            }
            let archive = file_utils::download_file_to_bytes(&client, &url, true).await?;
            zip_extract::extract(std::io::Cursor::new(archive), &server_dir, true)?;

            let old_path = server_dir.join("minecraft-server.jar");
            tokio::fs::rename(&old_path, &server_jar_path)
                .await
                .map_err(io_err!(old_path))?;

            let version_json = download_omniarchive_version(
                &MinecraftVersionCategory::Classic,
                &manifest,
                name,
                &sender,
                &client,
            )
            .await?;

            (Vec::new(), version_json)
        }
    };

    if !is_classic_server {
        tokio::fs::write(&server_jar_path, server_jar)
            .await
            .map_err(io_err!(server_jar_path))?;
    }

    let version_json_path = server_dir.join("details.json");
    tokio::fs::write(&version_json_path, serde_json::to_string(&version_json)?)
        .await
        .map_err(io_err!(version_json_path))?;

    let eula_path = server_dir.join("eula.txt");
    tokio::fs::write(&eula_path, "eula=true\n")
        .await
        .map_err(io_err!(eula_path))?;

    let server_config = InstanceConfigJson {
        mod_type: "Vanilla".to_owned(),
        java_override: None,
        ram_in_mb: 2048,
        enable_logger: Some(true),
        java_args: None,
        game_args: None,
        omniarchive: if let ListEntry::Omniarchive {
            category,
            name,
            url,
        } = version
        {
            Some(OmniarchiveEntry {
                name,
                url,
                category: category.to_string(),
            })
        } else {
            None
        },
        is_classic_server: is_classic_server.then_some(true),
    };

    let server_config_path = server_dir.join("config.json");
    tokio::fs::write(&server_config_path, serde_json::to_string(&server_config)?)
        .await
        .map_err(io_err!(server_config_path))?;

    Ok(())
}

async fn download_from_omniarchive(
    category: &MinecraftVersionCategory,
    manifest: &Manifest,
    name: &String,
    sender: &Option<Sender<ServerCreateProgress>>,
    client: &reqwest::Client,
    url: &String,
) -> Result<(Vec<u8>, VersionDetails), ServerError> {
    let version_json =
        download_omniarchive_version(category, manifest, name, sender, client).await?;
    info!("Downloading server jar");
    if let Some(sender) = sender {
        sender
            .send(ServerCreateProgress::P3DownloadingServerJar)
            .unwrap();
    }
    let server_jar = file_utils::download_file_to_bytes(client, &url, false).await?;
    Ok((server_jar, version_json))
}

async fn download_from_mojang(
    manifest: &Manifest,
    version: &String,
    sender: &Option<Sender<ServerCreateProgress>>,
    client: &reqwest::Client,
) -> Result<(Vec<u8>, VersionDetails), ServerError> {
    let version = manifest
        .find_name(version)
        .ok_or(ServerError::VersionNotFoundInManifest(version.clone()))?;
    info!("Downloading version JSON");
    if let Some(sender) = sender {
        sender
            .send(ServerCreateProgress::P2DownloadingVersionJson)
            .unwrap();
    }
    let version_json = file_utils::download_file_to_string(client, &version.url, false).await?;
    let version_json: VersionDetails = serde_json::from_str(&version_json)?;
    let Some(server) = &version_json.downloads.server else {
        return Err(ServerError::NoServerDownload);
    };
    info!("Downloading server jar");
    if let Some(sender) = sender {
        sender
            .send(ServerCreateProgress::P3DownloadingServerJar)
            .unwrap();
    }
    let server_jar = file_utils::download_file_to_bytes(client, &server.url, false).await?;
    Ok((server_jar, version_json))
}

async fn download_omniarchive_version(
    category: &MinecraftVersionCategory,
    manifest: &Manifest,
    name: &str,
    sender: &Option<Sender<ServerCreateProgress>>,
    client: &reqwest::Client,
) -> Result<VersionDetails, ServerError> {
    let version = match category {
        MinecraftVersionCategory::PreClassic => manifest.find_fuzzy(name, "rd-"),
        MinecraftVersionCategory::Classic => manifest.find_fuzzy(name, "c0."),
        MinecraftVersionCategory::Alpha => manifest.find_fuzzy(name, "a1."),
        MinecraftVersionCategory::Beta => manifest.find_fuzzy(name, "b1."),
        MinecraftVersionCategory::Indev => manifest.find_name("c0.30_01c"),
        MinecraftVersionCategory::Infdev => manifest.find_name("inf-20100618"),
    }
    .ok_or(ServerError::VersionNotFoundInManifest(name.to_owned()))?;
    info!("Downloading version JSON");
    if let Some(sender) = sender {
        sender
            .send(ServerCreateProgress::P2DownloadingVersionJson)
            .unwrap();
    }
    let version_json = file_utils::download_file_to_string(client, &version.url, false).await?;
    let version_json: VersionDetails = serde_json::from_str(&version_json)?;
    Ok(version_json)
}

pub fn delete_server(name: &str) -> Result<(), String> {
    let launcher_dir = file_utils::get_launcher_dir().map_err(|n| n.to_string())?;
    let server_dir = launcher_dir.join("servers").join(name);
    std::fs::remove_dir_all(&server_dir)
        .map_err(io_err!(server_dir))
        .map_err(|n| n.to_string())?;

    Ok(())
}