use std::{fs::File, io::Write};

use crate::{download::GameDownloader, error::LauncherResult};

pub async fn create(instance_name: &str, version: String) -> LauncherResult<()> {
    println!("[info] Started creating instance.");

    let game_downloader = GameDownloader::new(instance_name, &version)?;
    game_downloader.download_jar()?;
    game_downloader.download_libraries()?;
    game_downloader.download_logging_config()?;
    game_downloader.download_assets()?;

    let mut json_file = File::create(game_downloader.instance_dir.join("details.json"))?;
    json_file.write_all(serde_json::to_string(&game_downloader.version_json)?.as_bytes())?;

    Ok(())
}
