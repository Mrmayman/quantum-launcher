pub mod constants;
mod library_downloader;

// TODO: Implement BetaCraft wrapper
// https://codex-ipsa.dejvoss.cz/MCL-Data/launcher/libraries/betacraft-wrapper-20230129.jar

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::mpsc::{SendError, Sender},
};

use indicatif::ProgressBar;
use ql_core::{
    do_jobs, err, file_utils, info,
    json::{instance_config::InstanceConfigJson, manifest::Manifest, version::VersionDetails},
    DownloadError, DownloadProgress, IntoIoError, IoError, JsonDownloadError, ListEntry,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::{instance::launch::AssetRedownloadProgress, json_profiles::ProfileJson};

use self::constants::DEFAULT_RAM_MB_FOR_INSTANCE;

const OBJECTS_URL: &str = "https://resources.download.minecraft.net";

/// A struct that helps download a Minecraft instance.
///
/// # Example
/// Check the [`crate::create_instance`] function for an example.
pub struct GameDownloader {
    pub instance_dir: PathBuf,
    pub version_json: VersionDetails,
    network_client: Client,
    sender: Option<Sender<DownloadProgress>>,
}

impl GameDownloader {
    /// Create a game downloader and downloads the version JSON.
    ///
    /// For information on what order to download things in, check the `GameDownloader` struct documentation.
    ///
    /// `sender: Option<Sender<Progress>>` is an optional `mspc::Sender`
    /// that can be used if you are running this asynchronously or
    /// on a separate thread, and want to communicate progress with main thread.
    ///
    /// Leave as `None` if not required.
    pub async fn new(
        instance_name: &str,
        version: &ListEntry,
        sender: Option<Sender<DownloadProgress>>,
    ) -> Result<GameDownloader, DownloadError> {
        let Some(instance_dir) = GameDownloader::new_get_instance_dir(instance_name).await? else {
            return Err(DownloadError::InstanceAlreadyExists);
        };
        let network_client = Client::new();
        let version_json =
            GameDownloader::new_download_version_json(&network_client, version, sender.as_ref())
                .await?;

        Ok(Self {
            instance_dir,
            version_json,
            network_client,
            sender,
        })
    }

    pub fn with_existing_instance(
        version_json: VersionDetails,
        instance_dir: PathBuf,
        sender: Option<Sender<DownloadProgress>>,
    ) -> Self {
        let network_client = Client::new();
        Self {
            instance_dir,
            version_json,
            network_client,
            sender,
        }
    }

    pub async fn download_jar(&self) -> Result<(), DownloadError> {
        info!("Downloading game jar file.");
        self.send_progress(DownloadProgress::DownloadingJar)?;

        let url = &self.version_json.downloads.client.url;
        let jar_bytes =
            file_utils::download_file_to_bytes(&self.network_client, url, false).await?;

        let version_dir = self
            .instance_dir
            .join(".minecraft")
            .join("versions")
            .join(&self.version_json.id);
        tokio::fs::create_dir_all(&version_dir)
            .await
            .path(&version_dir)?;

        let jar_path = version_dir.join(format!("{}.jar", self.version_json.id));
        tokio::fs::write(&jar_path, jar_bytes)
            .await
            .path(jar_path)?;

        Ok(())
    }

    pub async fn download_logging_config(&self) -> Result<(), DownloadError> {
        if let Some(ref logging) = self.version_json.logging {
            info!("Downloading logging configuration.");
            self.send_progress(DownloadProgress::DownloadingLoggingConfig)?;

            let log_config_name = format!("logging-{}", logging.client.file.id);

            let log_config = file_utils::download_file_to_string(
                &self.network_client,
                &logging.client.file.url,
                false,
            )
            .await?;

            let config_path = self.instance_dir.join(log_config_name);
            tokio::fs::write(&config_path, log_config.as_bytes())
                .await
                .path(config_path)?;
        }
        Ok(())
    }

    async fn download_assets_fn(
        &self,
        object_data: &AssetIndexObject,
        objects_len: usize,
        assets_objects_path: &Path,
        bar: &ProgressBar,
        progress: &Mutex<usize>,
    ) -> Result<(), DownloadError> {
        let obj_id = &object_data.hash[0..2];

        let obj_folder = assets_objects_path.join(obj_id);
        tokio::fs::create_dir_all(&obj_folder)
            .await
            .path(&obj_folder)?;

        let obj_file_path = obj_folder.join(&object_data.hash);
        if obj_file_path.exists() {
            // Asset has already been downloaded. Skip.
            {
                let mut progress = progress.lock().await;
                *progress += 1;

                self.send_progress(DownloadProgress::DownloadingAssets {
                    progress: *progress,
                    out_of: objects_len,
                })?;
            }

            bar.inc(1);
            return Ok(());
        }

        let url = object_data
            .url
            .clone()
            .unwrap_or(format!("{OBJECTS_URL}/{obj_id}/{}", object_data.hash));
        let obj_data =
            file_utils::download_file_to_bytes(&self.network_client, &url, false).await?;

        tokio::fs::write(&obj_file_path, &obj_data)
            .await
            .path(obj_file_path)?;

        {
            let mut progress = progress.lock().await;
            *progress += 1;

            self.send_progress(DownloadProgress::DownloadingAssets {
                progress: *progress,
                out_of: objects_len,
            })?;
        }

        bar.inc(1);
        Ok(())
    }

    pub async fn download_assets(
        &self,
        sender: Option<&Sender<AssetRedownloadProgress>>,
    ) -> Result<(), DownloadError> {
        info!("Downloading assets.");
        if let Some(sender) = sender {
            sender.send(AssetRedownloadProgress::P1Start).unwrap();
        }
        let asset_index = GameDownloader::download_asset_index(
            &self.network_client,
            &self.version_json.assetIndex.url,
        )
        .await?;

        let launcher_dir = file_utils::get_launcher_dir()?;

        let assets_dir = launcher_dir.join("assets");
        tokio::fs::create_dir_all(&assets_dir)
            .await
            .path(&assets_dir)?;

        if self.version_json.assetIndex.id == "legacy" {
            let legacy_path = assets_dir.join("legacy_assets");
            tokio::fs::create_dir_all(&legacy_path)
                .await
                .path(assets_dir)?;

            let bar = indicatif::ProgressBar::new(asset_index.objects.len() as u64);

            let progress = Mutex::new(0);

            let objects_len = asset_index.objects.len();
            let results = asset_index.objects.iter().map(|(obj_id, object_data)| {
                self.download_assets_legacy_fn(
                    legacy_path.join(obj_id),
                    object_data,
                    &bar,
                    &progress,
                    objects_len,
                    sender,
                )
            });

            let outputs = do_jobs(results).await;

            if let Some(err) = outputs.into_iter().find_map(Result::err) {
                return Err(err);
            }
        } else {
            let current_assets_dir = assets_dir.join("dir");
            tokio::fs::create_dir_all(&current_assets_dir)
                .await
                .path(&current_assets_dir)?;

            let assets_indexes_path = current_assets_dir.join("indexes");
            tokio::fs::create_dir_all(&assets_indexes_path)
                .await
                .path(&assets_indexes_path)?;
            let assets_objects_path = current_assets_dir.join("objects");
            tokio::fs::create_dir_all(&assets_objects_path)
                .await
                .path(&assets_objects_path)?;

            let lock_path = current_assets_dir.join("download.lock");

            if lock_path.exists() {
                err!("Asset downloading previously interrupted?");
            }

            let lock_contents = "If you see this, the asset downloading hasn't finished. This will be deleted once finished.";
            tokio::fs::write(&lock_path, lock_contents)
                .await
                .path(&lock_path)?;

            self.save_asset_index_json(&assets_indexes_path, &asset_index)
                .await?;

            let objects_len = asset_index.objects.len();

            let bar = indicatif::ProgressBar::new(objects_len as u64);

            let progress_num = Mutex::new(0);

            let results = asset_index.objects.iter().map(|(_, object_data)| {
                self.download_assets_fn(
                    object_data,
                    objects_len,
                    &assets_objects_path,
                    &bar,
                    &progress_num,
                )
            });

            let outputs = do_jobs(results).await;

            if let Some(err) = outputs.into_iter().find_map(Result::err) {
                return Err(err);
            }

            tokio::fs::remove_file(&lock_path).await.path(&lock_path)?;
        }
        if let Some(sender) = sender {
            sender.send(AssetRedownloadProgress::P3Done).unwrap();
        }
        Ok(())
    }

    async fn download_assets_legacy_fn(
        &self,
        file_path: PathBuf,
        object_data: &AssetIndexObject,
        bar: &ProgressBar,
        progress: &Mutex<usize>,
        objects_len: usize,
        sender: Option<&Sender<AssetRedownloadProgress>>,
    ) -> Result<(), DownloadError> {
        if let Some(parent) = file_path.parent() {
            tokio::fs::create_dir_all(parent).await.path(parent)?;
        }

        if file_path.exists() {
            // Asset already downloaded. Skipping.
            self.send_asset_download_progress(progress, objects_len, sender, bar)
                .await?;
            return Ok(());
        }

        let obj_hash_sliced = &object_data.hash[0..2];
        let url = object_data.url.clone().unwrap_or(format!(
            "{OBJECTS_URL}/{obj_hash_sliced}/{}",
            object_data.hash
        ));
        let obj_data =
            file_utils::download_file_to_bytes(&self.network_client, &url, false).await?;
        tokio::fs::write(&file_path, &obj_data)
            .await
            .path(file_path)?;

        self.send_asset_download_progress(progress, objects_len, sender, bar)
            .await?;

        Ok(())
    }

    async fn send_asset_download_progress(
        &self,
        progress: &Mutex<usize>,
        objects_len: usize,
        sender: Option<&Sender<AssetRedownloadProgress>>,
        bar: &ProgressBar,
    ) -> Result<(), DownloadError> {
        let mut progress = progress.lock().await;
        self.send_progress(DownloadProgress::DownloadingAssets {
            progress: *progress,
            out_of: objects_len,
        })?;
        if let Some(sender) = sender {
            sender
                .send(AssetRedownloadProgress::P2Progress {
                    done: *progress,
                    out_of: objects_len,
                })
                .unwrap();
        }
        *progress += 1;
        bar.inc(1);
        Ok(())
    }

    async fn save_asset_index_json(
        &self,
        assets_indexes_path: &Path,
        asset_index: &AssetIndex,
    ) -> Result<(), DownloadError> {
        let assets_indexes_json_path =
            assets_indexes_path.join(format!("{}.json", self.version_json.assetIndex.id));
        tokio::fs::write(
            &assets_indexes_json_path,
            serde_json::to_string(asset_index)?.as_bytes(),
        )
        .await
        .path(assets_indexes_json_path)?;
        Ok(())
    }

    pub async fn download_asset_index(
        network_client: &Client,
        url: &str,
    ) -> Result<AssetIndex, JsonDownloadError> {
        let json = file_utils::download_file_to_string(network_client, url, false).await?;
        Ok(serde_json::from_str(&json)?)
    }

    pub async fn create_profiles_json(&self) -> Result<(), DownloadError> {
        let profile_json = ProfileJson::default();

        let profile_json = serde_json::to_string(&profile_json)?;
        let profile_json_path = self
            .instance_dir
            .join(".minecraft")
            .join("launcher_profiles.json");
        tokio::fs::write(&profile_json_path, profile_json)
            .await
            .path(profile_json_path)?;

        Ok(())
    }

    pub async fn create_version_json(&self) -> Result<(), DownloadError> {
        let json_file_path = self.instance_dir.join("details.json");

        tokio::fs::write(
            &json_file_path,
            serde_json::to_string(&self.version_json)?.as_bytes(),
        )
        .await
        .path(json_file_path)?;
        Ok(())
    }

    pub async fn create_config_json(&self) -> Result<(), DownloadError> {
        let config_json = InstanceConfigJson {
            java_override: None,
            ram_in_mb: DEFAULT_RAM_MB_FOR_INSTANCE,
            mod_type: "Vanilla".to_owned(),
            enable_logger: Some(true),
            java_args: None,
            game_args: None,
            is_classic_server: None,
        };
        let config_json = serde_json::to_string(&config_json)?;

        let config_json_path = self.instance_dir.join("config.json");
        tokio::fs::write(&config_json_path, config_json)
            .await
            .path(config_json_path)?;

        Ok(())
    }

    async fn new_download_version_json(
        network_client: &Client,
        version: &ListEntry,
        sender: Option<&Sender<DownloadProgress>>,
    ) -> Result<VersionDetails, DownloadError> {
        info!("Started downloading version manifest JSON.");
        if let Some(sender) = sender {
            sender.send(DownloadProgress::DownloadingJsonManifest)?;
        }
        let manifest = Manifest::download().await?;

        let version =
            manifest
                .find_name(&version.0)
                .ok_or(DownloadError::VersionNotFoundInManifest(
                    version.0.to_owned(),
                ))?;

        info!("Started downloading version details JSON.");
        if let Some(sender) = sender {
            sender.send(DownloadProgress::DownloadingVersionJson)?;
        }
        let version_json =
            file_utils::download_file_to_string(network_client, &version.url, false).await?;
        let version_json = serde_json::from_str(&version_json)?;
        Ok(version_json)
    }

    async fn new_get_instance_dir(instance_name: &str) -> Result<Option<PathBuf>, IoError> {
        info!("Initializing instance folder.");
        let launcher_dir = file_utils::get_launcher_dir()?;
        let instances_dir = launcher_dir.join("instances");
        tokio::fs::create_dir_all(&instances_dir)
            .await
            .path(&instances_dir)?;

        let current_instance_dir = instances_dir.join(instance_name);
        if current_instance_dir.exists() {
            return Ok(None);
        }
        tokio::fs::create_dir_all(&current_instance_dir)
            .await
            .path(&current_instance_dir)?;

        Ok(Some(current_instance_dir))
    }

    fn send_progress(&self, progress: DownloadProgress) -> Result<(), SendError<DownloadProgress>> {
        if let Some(ref sender) = self.sender {
            sender.send(progress)?;
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
pub struct AssetIndex {
    objects: HashMap<String, AssetIndexObject>,
}

#[derive(Serialize, Deserialize)]
pub struct AssetIndexObject {
    hash: String,
    size: usize,
    url: Option<String>,
}
