use std::sync::mpsc::Sender;

use crate::download::{progress::DownloadProgress, DownloadError, GameDownloader};

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
    println!("[info] Started creating instance.");

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

    Ok(())
}
