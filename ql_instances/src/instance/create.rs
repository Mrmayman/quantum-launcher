use std::sync::mpsc::Sender;

use ql_core::{file_utils, info, io_err};

use crate::{
    download::{progress::DownloadProgress, DownloadError, GameDownloader},
    ListEntry, LAUNCHER_VERSION_NAME,
};

/// Creates a Minecraft instance. The argument types have been made owned objects
/// for ease of used with `iced::Command` API.
///
/// # Arguments
/// - `instance_name` : Name of the instance (for example: "my cool instance")
/// - `version` : Version of the game to download (for example: "1.21.1", "1.12.2", "b1.7.3", etc.)
/// - `progress_sender` : If you want, you can create an `mpsc::channel()` of [`DownloadProgress`],
///   provide the receiver and keep polling the sender for progress updates. *If not needed, leave as `None`*
/// - `download_assets` : Whether to download the assets. Default: true. Disable this if you want to speed
///   up the download or reduce file size. *Disabling this will make the game completely silent;
///   No sounds or music will play*
///
/// # Errors
/// The errors have been converted to `String`. For the possible errors read the documentation
/// of `ql_instances::create_instance`.
pub async fn create_instance_wrapped(
    instance_name: String,
    version: ListEntry,
    progress_sender: Option<Sender<DownloadProgress>>,
    download_assets: bool,
) -> Result<(), String> {
    create_instance(&instance_name, version, progress_sender, download_assets)
        .await
        .map_err(|n| n.to_string())
}

/// Creates a Minecraft instance.
///
/// # Arguments
/// - `instance_name` : Name of the instance (for example: "my cool instance")
/// - `version` : Version of the game to download (for example: "1.21.1", "1.12.2", "b1.7.3", etc.)
/// - `progress_sender` : If you want, you can create an `mpsc::channel()` of [`DownloadProgress`],
///   provide the receiver and keep polling the sender for progress updates. *If not needed, leave as `None`*
/// - `download_assets` : Whether to download the assets. Default: true. Disable this if you want to speed
///   up the download or reduce file size. *Disabling this will make the game completely silent;
///   No sounds or music will play*
///
/// # Errors
/// Check the [`DownloadError`] documentation (if there is, lol). This is crap code and you must have standards.
pub async fn create_instance(
    instance_name: &str,
    version: ListEntry,
    progress_sender: Option<Sender<DownloadProgress>>,
    download_assets: bool,
) -> Result<(), DownloadError> {
    info!("Started creating instance.");

    // An empty asset directory.
    let launcher_dir = file_utils::get_launcher_dir()?;

    let assets_dir = launcher_dir.join("assets/null");
    std::fs::create_dir_all(&assets_dir).map_err(io_err!(assets_dir))?;

    if let Some(ref sender) = progress_sender {
        sender.send(DownloadProgress::Started)?;
    }

    let game_downloader = GameDownloader::new(instance_name, &version, progress_sender).await?;

    game_downloader.download_logging_config().await?;
    game_downloader.download_jar().await?;
    game_downloader.download_libraries().await?;

    if download_assets {
        game_downloader.download_assets(None).await?;
    }

    game_downloader.create_version_json().await?;
    game_downloader.create_profiles_json().await?;
    game_downloader.create_config_json().await?;

    let version_file_path = launcher_dir
        .join("instances")
        .join(instance_name)
        .join("launcher_version.txt");
    std::fs::write(&version_file_path, LAUNCHER_VERSION_NAME)
        .map_err(io_err!(version_file_path))?;

    let mods_dir = launcher_dir
        .join("instances")
        .join(instance_name)
        .join(".minecraft/mods");
    std::fs::create_dir_all(&mods_dir).map_err(io_err!(mods_dir))?;

    info!("Finished creating instance: {instance_name}");

    Ok(())
}
