use std::sync::mpsc::Sender;

use crate::{
    download::{progress::DownloadProgress, GameDownloader},
    error::LauncherResult,
};

pub async fn create_instance(
    instance_name: String,
    version: String,
    progress_sender: Option<Sender<DownloadProgress>>,
) -> Result<(), String> {
    create(&instance_name, version, progress_sender)
        .await
        .map_err(|n| n.to_string())
}

async fn create(
    instance_name: &str,
    version: String,
    progress_sender: Option<Sender<DownloadProgress>>,
) -> LauncherResult<()> {
    println!("[info] Started creating instance.");

    if let Some(ref sender) = progress_sender {
        sender.send(DownloadProgress::Started)?;
    }

    let game_downloader = GameDownloader::new(instance_name, &version, progress_sender).await?;
    game_downloader.download_logging_config().await?;
    game_downloader.download_jar().await?;
    game_downloader.download_libraries().await?;
    game_downloader.download_assets().await?;

    game_downloader.create_version_json()?;
    game_downloader.create_profiles_json()?;
    game_downloader.create_config_json()?;

    Ok(())
}
