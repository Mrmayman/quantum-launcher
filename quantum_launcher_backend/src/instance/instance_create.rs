use std::{fs::File, io::Write, sync::mpsc::Sender};

use crate::{
    download::{DownloadProgress, GameDownloader},
    error::{LauncherError, LauncherResult},
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

    let json_file_path = game_downloader.instance_dir.join("details.json");
    let mut json_file = File::create(&json_file_path)
        .map_err(|err| LauncherError::IoError(err, json_file_path.clone()))?;
    json_file
        .write_all(serde_json::to_string(&game_downloader.version_json)?.as_bytes())
        .map_err(|err| LauncherError::IoError(err, json_file_path.clone()))?;

    Ok(())
}
