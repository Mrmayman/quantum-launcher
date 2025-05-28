use std::sync::mpsc::Sender;

use ql_core::{
    file_utils, info,
    json::{InstanceConfigJson, Manifest, VersionDetails},
    pt, GenericProgress, IntoIoError, IntoJsonError, IntoStringError, ListEntry, LAUNCHER_DIR,
};

use crate::ServerError;

/// Creates a minecraft server with the given name and version.
///
/// # Arguments
/// - `name` - The name of the server.
/// - `version` - The version of the server.
/// - `sender` - A sender to send progress updates to
///   (optional).
///
/// # Errors
///
/// TLDR; there's a lot of errors. I only wrote this because
/// clippy was bothering me (WTF: )
///
/// If:
/// - server already exists
/// - EULA and `config.json` file couldn't be saved
/// ## Server Jar...
/// - ...couldn't be downloaded from
///   mojang/omniarchive (internet/server issue)
/// - ...couldn't be saved to a file
/// - classic server zip file couldn't be extracted
/// - classic server zip file doesn't have a `minecraft-server.jar`
/// ## Manifest...
/// - ...couldn't be downloaded
/// - ...couldn't be parsed into JSON
/// - ...doesn't have server version
/// ## Version JSON...
/// - ...couldn't be downloaded
/// - ...couldn't be parsed into JSON
/// - ...couldn't be saved to `details.json`
/// - ...doesn't have `downloads` field
pub async fn create_server(
    name: String,
    version: ListEntry,
    sender: Option<Sender<GenericProgress>>,
) -> Result<String, ServerError> {
    info!("Creating server");
    pt!("Downloading Manifest");
    progress_manifest(sender.as_ref());
    let manifest = Manifest::download().await?;

    let server_dir = get_server_dir(&name).await?;
    let server_jar_path = server_dir.join("server.jar");

    let mut is_classic_server = false;

    let version_manifest = manifest
        .find_name(&version.name)
        .ok_or(ServerError::VersionNotFoundInManifest(version.name.clone()))?;
    pt!("Downloading version JSON");
    progress_json(sender.as_ref());

    let version_json: VersionDetails =
        file_utils::download_file_to_json(&version_manifest.url, false).await?;
    let Some(server) = &version_json.downloads.server else {
        return Err(ServerError::NoServerDownload);
    };

    pt!("Downloading server jar");
    progress_server_jar(sender.as_ref());
    if version.is_classic_server {
        is_classic_server = true;

        let archive = file_utils::download_file_to_bytes(&server.url, true).await?;
        zip_extract::extract(std::io::Cursor::new(archive), &server_dir, true)?;

        let old_path = server_dir.join("minecraft-server.jar");
        tokio::fs::rename(&old_path, &server_jar_path)
            .await
            .path(old_path)?;
    } else {
        file_utils::download_file_to_path(&server.url, false, &server_jar_path).await?;
    }

    write_json(&server_dir, version_json).await?;
    write_eula(&server_dir).await?;
    write_config(is_classic_server, &server_dir).await?;

    let mods_dir = server_dir.join("mods");
    tokio::fs::create_dir(&mods_dir).await.path(mods_dir)?;

    pt!("Finished");

    Ok(name)
}

async fn write_config(
    is_classic_server: bool,
    server_dir: &std::path::Path,
) -> Result<(), ServerError> {
    let server_config = InstanceConfigJson {
        mod_type: "Vanilla".to_owned(),
        java_override: None,
        ram_in_mb: 2048,
        enable_logger: Some(true),
        java_args: None,
        game_args: None,
        is_classic_server: is_classic_server.then_some(true),

        #[allow(deprecated)]
        omniarchive: None,

        // # Doesn't affect servers:
        // I could add GC tuning to servers too, but I can't find
        // a way to measure performance on a server. Besides this setting
        // makes performance worse on clients so I guess it's same for servers?
        do_gc_tuning: None,
        // This won't do anything on servers. Who wants to lose their *only way*
        // to control the server instantly after starting it?
        close_on_start: None,
    };
    let server_config_path = server_dir.join("config.json");
    tokio::fs::write(
        &server_config_path,
        serde_json::to_string(&server_config).json_to()?,
    )
    .await
    .path(server_config_path)?;
    Ok(())
}

async fn get_server_dir(name: &str) -> Result<std::path::PathBuf, ServerError> {
    let server_dir = LAUNCHER_DIR.join("servers").join(name);
    if server_dir.exists() {
        return Err(ServerError::ServerAlreadyExists);
    }
    tokio::fs::create_dir_all(&server_dir)
        .await
        .path(&server_dir)?;
    Ok(server_dir)
}

fn progress_manifest(sender: Option<&Sender<GenericProgress>>) {
    if let Some(sender) = sender {
        sender
            .send(GenericProgress {
                done: 0,
                total: 3,
                message: Some("Downloading Manifest".to_owned()),
                has_finished: false,
            })
            .unwrap();
    }
}

async fn write_eula(server_dir: &std::path::Path) -> Result<(), ServerError> {
    let eula_path = server_dir.join("eula.txt");
    tokio::fs::write(&eula_path, "eula=true\n")
        .await
        .path(eula_path)?;
    Ok(())
}

async fn write_json(
    server_dir: &std::path::Path,
    version_json: VersionDetails,
) -> Result<(), ServerError> {
    let version_json_path = server_dir.join("details.json");
    tokio::fs::write(
        &version_json_path,
        serde_json::to_string(&version_json).json_to()?,
    )
    .await
    .path(version_json_path)?;
    Ok(())
}

fn progress_server_jar(sender: Option<&Sender<GenericProgress>>) {
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
}

fn progress_json(sender: Option<&Sender<GenericProgress>>) {
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
}

/// Deletes a server with the given name.
///
/// # Errors
/// - If the server does not exist.
/// - If the server directory couldn't be deleted.
/// - If the launcher directory couldn't be found or created.
pub fn delete_server(name: &str) -> Result<(), String> {
    let server_dir = LAUNCHER_DIR.join("servers").join(name);
    std::fs::remove_dir_all(&server_dir)
        .path(server_dir)
        .strerr()?;

    Ok(())
}
