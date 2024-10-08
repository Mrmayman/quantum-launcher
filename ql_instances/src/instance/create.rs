use std::sync::mpsc::Sender;

use crate::{
    download::{progress::DownloadProgress, DownloadError, GameDownloader},
    file_utils, info, io_err, LAUNCHER_VERSION_NAME,
};

pub async fn create_instance(
    instance_name: String,
    version: String,
    progress_sender: Option<Sender<DownloadProgress>>,
    download_assets: bool,
) -> Result<(), String> {
    create(&instance_name, version, progress_sender, download_assets)
        .await
        .map_err(|n| n.to_string())
}

async fn create(
    instance_name: &str,
    version: String,
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
        game_downloader.download_assets().await?;
    }

    game_downloader.create_version_json()?;
    game_downloader.create_profiles_json()?;
    game_downloader.create_config_json()?;

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
