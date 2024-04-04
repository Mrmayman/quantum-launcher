use std::{fs::File, io::Write, sync::mpsc::Sender};

use crate::{
    download::{DownloadProgress, GameDownloader},
    error::LauncherResult,
};

pub async fn create_instance(
    instance_name: String,
    version: String,
    progress_sender: Option<Sender<DownloadProgress>>,
) -> Result<(), String> {
    create(&instance_name, version, progress_sender).map_err(|n| n.to_string())
}

fn create(
    instance_name: &str,
    version: String,
    progress_sender: Option<Sender<DownloadProgress>>,
) -> LauncherResult<()> {
    println!("[info] Started creating instance.");

    if let Some(ref sender) = progress_sender {
        sender.send(DownloadProgress::Started)?;
    }

    let game_downloader = GameDownloader::new(instance_name, &version, progress_sender)?;
    game_downloader.download_logging_config()?;
    game_downloader.download_jar()?;
    game_downloader.download_libraries()?;
    game_downloader.download_assets()?;

    let mut json_file = File::create(game_downloader.instance_dir.join("details.json"))?;
    json_file.write_all(serde_json::to_string(&game_downloader.version_json)?.as_bytes())?;

    Ok(())
}
