use std::sync::mpsc::Sender;

use omniarchive_api::{ListEntry, MinecraftVersionCategory};
use ql_core::{
    file_utils, info,
    json::{
        instance_config::{InstanceConfigJson, OmniarchiveEntry},
        manifest::Manifest,
        version::VersionDetails,
    },
    GenericProgress, IntoIoError,
};

use crate::ServerError;

/// [`create_server_w`] `_w` function
pub async fn create_server_w(
    name: String,
    version: ListEntry,
    sender: Option<Sender<GenericProgress>>,
) -> Result<String, String> {
    create_server(&name, version, sender)
        .await
        .map_err(|n| n.to_string())
        .map(|()| name)
}

/// Creates a minecraft server with the given name and version.
///
/// # Arguments
/// - `name` - The name of the server.
/// - `version` - The version of the server.
/// - `sender` - A sender to send progress updates to
///   (optional).
pub async fn create_server(
    name: &str,
    version: ListEntry,
    sender: Option<Sender<GenericProgress>>,
) -> Result<(), ServerError> {
    let client = reqwest::Client::new();
    info!("Creating server: Downloading Manifest");
    if let Some(sender) = &sender {
        sender
            .send(GenericProgress {
                done: 0,
                total: 3,
                message: Some("Downloading Manifest".to_owned()),
                has_finished: false,
            })
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
        .path(&server_dir)?;

    let server_jar_path = server_dir.join("server.jar");

    let mut is_classic_server = false;

    let (server_jar, version_json) = match &version {
        ListEntry::Normal(version) => {
            download_from_mojang(&manifest, version, sender.as_ref(), &client).await?
        }
        ListEntry::Omniarchive {
            category,
            name,
            url,
        } => {
            download_from_omniarchive(category, &manifest, name, sender.as_ref(), &client, url)
                .await?
        }
        ListEntry::OmniarchiveClassicZipServer { name, url } => {
            is_classic_server = true;

            if let Some(sender) = &sender {
                sender
                    .send(GenericProgress {
                        done: 2,
                        total: 3,
                        message: Some("Downloading Server Jar".to_owned()),
                        has_finished: false,
                    })
                    .unwrap();
            }
            let archive = file_utils::download_file_to_bytes(&client, url, true).await?;
            zip_extract::extract(std::io::Cursor::new(archive), &server_dir, true)?;

            let old_path = server_dir.join("minecraft-server.jar");
            tokio::fs::rename(&old_path, &server_jar_path)
                .await
                .path(old_path)?;

            let version_json = download_omniarchive_version(
                &MinecraftVersionCategory::Classic,
                &manifest,
                name,
                sender.as_ref(),
                &client,
            )
            .await?;

            (Vec::new(), version_json)
        }
    };

    if !is_classic_server {
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
        .path(server_config_path)?;

    let mods_dir = server_dir.join("mods");
    tokio::fs::create_dir(&mods_dir).await.path(mods_dir)?;

    Ok(())
}

async fn download_from_omniarchive(
    category: &MinecraftVersionCategory,
    manifest: &Manifest,
    name: &str,
    sender: Option<&Sender<GenericProgress>>,
    client: &reqwest::Client,
    url: &str,
) -> Result<(Vec<u8>, VersionDetails), ServerError> {
    let version_json =
        download_omniarchive_version(category, manifest, name, sender, client).await?;
    info!("Downloading server jar");
    if let Some(sender) = sender {
        sender
            .send(GenericProgress {
                done: 2,
                total: 3,
                message: Some("Downloading Server Jar".to_owned()),
                has_finished: false,
            })
            .unwrap();
    }
    let server_jar = file_utils::download_file_to_bytes(client, url, false).await?;
    Ok((server_jar, version_json))
}

async fn download_from_mojang(
    manifest: &Manifest,
    version: &str,
    sender: Option<&Sender<GenericProgress>>,
    client: &reqwest::Client,
) -> Result<(Vec<u8>, VersionDetails), ServerError> {
    let version = manifest
        .find_name(version)
        .ok_or(ServerError::VersionNotFoundInManifest(version.to_owned()))?;
    info!("Downloading version JSON");
    if let Some(sender) = sender {
        sender
            .send(GenericProgress {
                done: 1,
                total: 3,
                message: Some("Downloading Version JSON".to_owned()),
                has_finished: false,
            })
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
            .send(GenericProgress {
                done: 2,
                total: 3,
                message: Some("Downloading Server Jar".to_owned()),
                has_finished: false,
            })
            .unwrap();
    }
    let server_jar = file_utils::download_file_to_bytes(client, &server.url, false).await?;
    Ok((server_jar, version_json))
}

async fn download_omniarchive_version(
    category: &MinecraftVersionCategory,
    manifest: &Manifest,
    name: &str,
    sender: Option<&Sender<GenericProgress>>,
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
            .send(GenericProgress {
                done: 1,
                total: 3,
                message: Some("Downloading Version JSON".to_owned()),
                has_finished: false,
            })
            .unwrap();
    }
    let version_json = file_utils::download_file_to_string(client, &version.url, false).await?;
    let version_json: VersionDetails = serde_json::from_str(&version_json)?;
    Ok(version_json)
}

/// Deletes a server with the given name.
///
/// # Errors
/// - If the server does not exist.
/// - If the server directory could not be deleted.
/// - If the launcher directory could not be found or created.
pub fn delete_server(name: &str) -> Result<(), String> {
    let launcher_dir = file_utils::get_launcher_dir().map_err(|n| n.to_string())?;
    let server_dir = launcher_dir.join("servers").join(name);
    std::fs::remove_dir_all(&server_dir)
        .path(server_dir)
        .map_err(|n| n.to_string())?;

    Ok(())
}
