use std::sync::mpsc::Sender;

use ql_core::{
    info, json::OmniarchiveEntry, DownloadProgress, IntoIoError, LAUNCHER_DIR,
    LAUNCHER_VERSION_NAME,
};

use crate::{
    download::{DownloadError, GameDownloader},
    ListEntry,
};

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
/// # Returns
/// Returns the instance name that you passed in.
///
/// # Errors
/// Check the [`DownloadError`] documentation (if there is, lol).
/// This is crap code and you must have standards. (WTF: )
pub async fn create_instance(
    instance_name: String,
    version: ListEntry,
    progress_sender: Option<Sender<DownloadProgress>>,
    download_assets: bool,
) -> Result<String, DownloadError> {
    info!("Started creating instance.");

    // An empty asset directory.
    let launcher_dir = &*LAUNCHER_DIR;

    let assets_dir = launcher_dir.join("assets/null");
    tokio::fs::create_dir_all(&assets_dir)
        .await
        .path(assets_dir)?;

    let mut game_downloader =
        GameDownloader::new(&instance_name, &version, progress_sender).await?;

    game_downloader.download_logging_config().await?;
    game_downloader.download_jar().await?;
    game_downloader.download_libraries().await?;

    if download_assets {
        game_downloader.download_assets(None).await?;
    }

    game_downloader.create_version_json().await?;
    game_downloader.create_profiles_json().await?;
    game_downloader
        .create_config_json(
            if let ListEntry::Omniarchive {
                category,
                name,
                url,
                nice_name,
            } = &version
            {
                Some(OmniarchiveEntry {
                    name: name.clone(),
                    url: url.clone(),
                    category: category.to_string(),
                    nice_name: Some(nice_name.clone()),
                })
            } else {
                None
            },
        )
        .await?;

    let version_file_path = launcher_dir
        .join("instances")
        .join(&instance_name)
        .join("launcher_version.txt");
    tokio::fs::write(&version_file_path, LAUNCHER_VERSION_NAME)
        .await
        .path(version_file_path)?;

    let mods_dir = launcher_dir
        .join("instances")
        .join(&instance_name)
        .join(".minecraft/mods");
    tokio::fs::create_dir_all(&mods_dir).await.path(mods_dir)?;

    info!("Finished creating instance: {instance_name}");

    Ok(instance_name)
}
