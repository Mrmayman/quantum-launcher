pub mod constants;
mod library_downloader;

use std::{
    path::{Path, PathBuf},
    sync::mpsc::{SendError, Sender},
};

use indicatif::ProgressBar;
use omniarchive_api::MinecraftVersionCategory;
use ql_core::{
    do_jobs, err, file_utils, info,
    json::{InstanceConfigJson, Manifest, OmniarchiveEntry, VersionDetails},
    DownloadError, DownloadProgress, GenericProgress, IntoIoError, IoError, JsonDownloadError,
};
use serde_json::Value;
use tokio::sync::Mutex;

use crate::{json_profiles::ProfileJson, ListEntry};

use self::constants::DEFAULT_RAM_MB_FOR_INSTANCE;

const OBJECTS_URL: &str = "https://resources.download.minecraft.net";

/// A struct that helps download a Minecraft instance.
///
/// # Example
/// Check the [`crate::create_instance`] function for an example.
pub struct GameDownloader {
    pub instance_dir: PathBuf,
    pub version_json: VersionDetails,
    pub version: ListEntry,
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
        let version_json =
            GameDownloader::new_download_version_json(version, sender.as_ref()).await?;

        Ok(Self {
            instance_dir,
            version_json,
            version: version.clone(),
            sender,
        })
    }

    pub fn with_existing_instance(
        version_json: VersionDetails,
        instance_dir: PathBuf,
        sender: Option<Sender<DownloadProgress>>,
    ) -> Self {
        let version = ListEntry::Normal(version_json.id.clone());
        Self {
            instance_dir,
            version_json,
            version,
            sender,
        }
    }

    pub async fn download_jar(&self) -> Result<(), DownloadError> {
        info!("Downloading game jar file.");
        self.send_progress(DownloadProgress::DownloadingJar)?;

        let url = match &self.version {
            ListEntry::Normal(_) => &self.version_json.downloads.client.url,
            ListEntry::Omniarchive { url, .. } => url,
            ListEntry::OmniarchiveClassicZipServer { .. } => {
                return Err(DownloadError::DownloadClassicZip)
            }
        };
        let jar_bytes = file_utils::download_file_to_bytes(url, false).await?;

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

            let log_config =
                file_utils::download_file_to_string(&logging.client.file.url, false).await?;

            let config_path = self.instance_dir.join(log_config_name);
            tokio::fs::write(&config_path, log_config.as_bytes())
                .await
                .path(config_path)?;
        }
        Ok(())
    }

    async fn download_assets_fn(
        &self,
        object_data: &serde_json::Value,
        objects_len: usize,
        assets_objects_path: &Path,
        bar: &ProgressBar,
        progress: &Mutex<usize>,
    ) -> Result<(), DownloadError> {
        let obj_hash = object_data["hash"]
            .as_str()
            .ok_or(DownloadError::SerdeFieldNotFound(
                "asset_index.objects[].hash".to_owned(),
            ))?;

        let obj_id = &obj_hash[0..2];

        let obj_folder = assets_objects_path.join(obj_id);
        tokio::fs::create_dir_all(&obj_folder)
            .await
            .path(&obj_folder)?;

        let obj_file_path = obj_folder.join(obj_hash);
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

        let obj_data = file_utils::download_file_to_bytes(
            &format!("{OBJECTS_URL}/{obj_id}/{obj_hash}"),
            false,
        )
        .await?;

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
        sender: Option<&Sender<GenericProgress>>,
    ) -> Result<(), DownloadError> {
        info!("Downloading assets.");
        if let Some(sender) = sender {
            sender.send(GenericProgress::default()).unwrap();
        }
        let asset_index = GameDownloader::download_json(&self.version_json.assetIndex.url).await?;

        let launcher_dir = file_utils::get_launcher_dir().await?;

        let assets_dir = launcher_dir.join("assets");
        tokio::fs::create_dir_all(&assets_dir)
            .await
            .path(&assets_dir)?;

        /*if self.version_json.assetIndex.id == "legacy" {
            let legacy_path = assets_dir.join("legacy_assets");
            tokio::fs::create_dir_all(&legacy_path)
                .await
                .path(assets_dir)?;

            let objects =
                asset_index["objects"]
                    .as_object()
                    .ok_or(DownloadError::SerdeFieldNotFound(
                        "asset_index.objects".to_owned(),
                    ))?;

            let bar = indicatif::ProgressBar::new(objects.len() as u64);

            let progress = Mutex::new(0);

            let results = objects.iter().map(|(obj_id, object_data)| {
                self.download_assets_legacy_fn(
                    legacy_path.join(obj_id),
                    object_data,
                    &bar,
                    &progress,
                    objects.len(),
                    sender,
                )
            });

            let outputs = do_jobs(results).await;

            if let Some(err) = outputs.into_iter().find_map(Result::err) {
                return Err(err);
            }
        } else */
        {
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

            let objects =
                asset_index["objects"]
                    .as_object()
                    .ok_or(DownloadError::SerdeFieldNotFound(
                        "asset_index.objects".to_owned(),
                    ))?;
            let objects_len = objects.len();

            let bar = indicatif::ProgressBar::new(objects_len as u64);

            let progress_num = Mutex::new(0);

            let results = objects.iter().map(|(_, object_data)| {
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
            sender.send(GenericProgress::finished()).unwrap();
        }
        Ok(())
    }

    /*async fn download_assets_legacy_fn(
        &self,
        file_path: PathBuf,
        object_data: &Value,
        bar: &ProgressBar,
        progress: &Mutex<usize>,
        objects_len: usize,
        sender: Option<&Sender<GenericProgress>>,
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

        let obj_hash = object_data["hash"]
            .as_str()
            .ok_or(DownloadError::SerdeFieldNotFound(
                "asset_index.objects[].hash".to_owned(),
            ))?;
        let obj_hash_sliced = &obj_hash[0..2];
        let obj_data = file_utils::download_file_to_bytes(

            &format!("{OBJECTS_URL}/{obj_hash_sliced}/{obj_hash}"),
            false,
        )
        .await?;
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
        sender: Option<&Sender<GenericProgress>>,
        bar: &ProgressBar,
    ) -> Result<(), DownloadError> {
        let mut progress = progress.lock().await;
        self.send_progress(DownloadProgress::DownloadingAssets {
            progress: *progress,
            out_of: objects_len,
        })?;
        if let Some(sender) = sender {
            sender
                .send(GenericProgress {
                    done: *progress,
                    total: objects_len,
                    message: None,
                    has_finished: false,
                })
                .unwrap();
        }
        *progress += 1;
        bar.inc(1);
        Ok(())
    }*/

    async fn save_asset_index_json(
        &self,
        assets_indexes_path: &Path,
        asset_index: &Value,
    ) -> Result<(), DownloadError> {
        let assets_indexes_json_path =
            assets_indexes_path.join(format!("{}.json", self.version_json.assetIndex.id));
        tokio::fs::write(
            &assets_indexes_json_path,
            asset_index.to_string().as_bytes(),
        )
        .await
        .path(assets_indexes_json_path)?;
        Ok(())
    }

    pub async fn download_json(url: &str) -> Result<Value, JsonDownloadError> {
        let json = file_utils::download_file_to_string(url, false).await?;
        Ok(serde_json::from_str::<serde_json::Value>(&json)?)
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

    pub async fn create_config_json(
        &self,
        omniarchive: Option<OmniarchiveEntry>,
    ) -> Result<(), DownloadError> {
        let config_json = InstanceConfigJson {
            java_override: None,
            ram_in_mb: DEFAULT_RAM_MB_FOR_INSTANCE,
            mod_type: "Vanilla".to_owned(),
            enable_logger: Some(true),
            java_args: None,
            game_args: None,
            omniarchive,
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
        version: &ListEntry,
        sender: Option<&Sender<DownloadProgress>>,
    ) -> Result<VersionDetails, DownloadError> {
        info!("Downloading version manifest JSON.");
        if let Some(sender) = sender {
            sender.send(DownloadProgress::DownloadingJsonManifest)?;
        }
        let manifest = Manifest::download().await?;

        let version = match version {
            ListEntry::Normal(name) => manifest
                .find_name(name)
                .ok_or(DownloadError::VersionNotFoundInManifest(name.to_owned()))?,
            ListEntry::Omniarchive { category, name, .. } => match category {
                MinecraftVersionCategory::PreClassic => manifest.find_fuzzy(name, "rd-"),
                MinecraftVersionCategory::Classic => manifest.find_fuzzy(name, "c0."),
                MinecraftVersionCategory::Alpha => manifest.find_fuzzy(name, "a1."),
                MinecraftVersionCategory::Beta => manifest.find_fuzzy(name, "b1."),
                MinecraftVersionCategory::Indev => manifest.find_name("c0.30_01c"),
                MinecraftVersionCategory::Infdev => manifest.find_name("inf-20100618"),
            }
            .ok_or(DownloadError::VersionNotFoundInManifest(name.to_owned()))?,
            ListEntry::OmniarchiveClassicZipServer { .. } => {
                return Err(DownloadError::DownloadClassicZip)
            }
        };

        info!("Downloading version details JSON.");
        if let Some(sender) = sender {
            sender.send(DownloadProgress::DownloadingVersionJson)?;
        }
        let version_json = file_utils::download_file_to_string(&version.url, false).await?;
        let version_json = serde_json::from_str(&version_json)?;
        Ok(version_json)
    }

    async fn new_get_instance_dir(instance_name: &str) -> Result<Option<PathBuf>, IoError> {
        info!("Initializing instance folder.");
        let launcher_dir = file_utils::get_launcher_dir().await?;
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
