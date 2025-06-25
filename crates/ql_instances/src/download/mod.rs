pub mod constants;
mod library_downloader;

use std::path::Path;
use std::{path::PathBuf, sync::mpsc::Sender};

use ql_core::json::AssetIndexMap;
use ql_core::{
    do_jobs,
    file_utils::{self, LAUNCHER_DIR},
    impl_3_errs_jri, info,
    json::{InstanceConfigJson, Manifest, VersionDetails},
    pt, DownloadFileError, DownloadProgress, IntoIoError, IntoJsonError, IoError, JsonError,
    ListEntry, RequestError,
};
use thiserror::Error;
use tokio::sync::Mutex;

use crate::json_profiles::ProfileJson;

use self::constants::DEFAULT_RAM_MB_FOR_INSTANCE;

const DOWNLOAD_ERR_PREFIX: &str = "while creating instance:\n";

#[derive(Debug, Error)]
pub enum DownloadError {
    #[error("{DOWNLOAD_ERR_PREFIX}{0}")]
    Json(#[from] JsonError),
    #[error("{DOWNLOAD_ERR_PREFIX}{0}")]
    Request(#[from] RequestError),
    #[error("{DOWNLOAD_ERR_PREFIX}{0}")]
    Io(#[from] IoError),
    #[error("an instance with that name already exists!")]
    InstanceAlreadyExists,
    #[error("{DOWNLOAD_ERR_PREFIX}version not found in manifest.json: {0}")]
    VersionNotFoundInManifest(String),
    #[error("{DOWNLOAD_ERR_PREFIX}in assets JSON, field not found: \"{0}\"")]
    AssetsJsonFieldNotFound(String),
    #[error("{DOWNLOAD_ERR_PREFIX}could not extract native libraries:\n{0}")]
    NativesExtractError(#[from] zip_extract::ZipExtractError),
    #[error("{DOWNLOAD_ERR_PREFIX}tried to remove natives outside folder. POTENTIAL SECURITY RISK AVOIDED")]
    NativesOutsideDirRemove,
}

impl_3_errs_jri!(DownloadError, Json, Request, Io);

/// A struct that helps download a Minecraft instance.
///
/// # Example
/// Check the [`crate::create_instance`] function for an example.
pub struct GameDownloader {
    pub instance_dir: PathBuf,
    pub version_json: VersionDetails,
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
            sender,
        })
    }

    #[allow(unused)]
    pub fn with_existing_instance(
        version_json: VersionDetails,
        instance_dir: PathBuf,
        sender: Option<Sender<DownloadProgress>>,
    ) -> Self {
        Self {
            instance_dir,
            version_json,
            sender,
        }
    }

    pub async fn download_jar(&self) -> Result<(), DownloadError> {
        info!("Downloading game jar file.");
        self.send_progress(DownloadProgress::DownloadingJar, false);

        let version_dir = self
            .instance_dir
            .join(".minecraft")
            .join("versions")
            .join(&self.version_json.id);
        tokio::fs::create_dir_all(&version_dir)
            .await
            .path(&version_dir)?;

        let jar_path = version_dir.join(format!("{}.jar", self.version_json.id));

        file_utils::download_file_to_path(
            &self.version_json.downloads.client.url,
            false,
            &jar_path,
        )
        .await?;

        Ok(())
    }

    pub async fn download_logging_config(&self) -> Result<(), DownloadError> {
        if let Some(ref logging) = self.version_json.logging {
            info!("Downloading logging configuration.");
            self.send_progress(DownloadProgress::DownloadingLoggingConfig, false);

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

    pub async fn download_assets(&self) -> Result<(), DownloadError> {
        info!("Downloading assets");
        let asset_index: AssetIndexMap =
            file_utils::download_file_to_json(&self.version_json.assetIndex.url, false).await?;

        let assets_dir = LAUNCHER_DIR.join("assets");
        tokio::fs::create_dir_all(&assets_dir)
            .await
            .path(&assets_dir)?;

        // assets/dir is the current location, because
        // other assets/* folders are used by old
        // QuantumLauncher versions for an outdated format
        // (which automigrates to assets/dir when launching game).
        let current_assets_dir = assets_dir.join("dir");
        tokio::fs::create_dir_all(&current_assets_dir)
            .await
            .path(&current_assets_dir)?;

        self.save_asset_index(&asset_index, &current_assets_dir)
            .await?;

        let assets_objects_path = &current_assets_dir.join("objects");
        tokio::fs::create_dir_all(&assets_objects_path)
            .await
            .path(assets_objects_path)?;

        let out_of = asset_index.objects.len();
        let bar = &indicatif::ProgressBar::new(out_of as u64);
        let progress_num = &Mutex::new(0);

        let results = asset_index.objects.values().map(|asset| async move {
            asset.download(assets_objects_path).await?;

            let mut progress = progress_num.lock().await;
            *progress += 1;

            self.send_progress(
                DownloadProgress::DownloadingAssets {
                    progress: *progress,
                    out_of,
                },
                true,
            );

            bar.inc(1);

            Ok::<(), DownloadFileError>(())
        });

        _ = do_jobs(results).await?;
        Ok(())
    }

    async fn save_asset_index(
        &self,
        asset_index: &AssetIndexMap,
        current_assets_dir: &Path,
    ) -> Result<(), DownloadError> {
        let assets_indexes_path = current_assets_dir.join("indexes");
        tokio::fs::create_dir_all(&assets_indexes_path)
            .await
            .path(&assets_indexes_path)?;

        let assets_indexes_json_path =
            assets_indexes_path.join(format!("{}.json", self.version_json.assetIndex.id));
        tokio::fs::write(
            &assets_indexes_json_path,
            serde_json::to_string(&asset_index).json_to()?,
        )
        .await
        .path(assets_indexes_json_path)?;

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

    pub async fn create_profiles_json(&self) -> Result<(), DownloadError> {
        let profile_json = ProfileJson::default();

        let profile_json = serde_json::to_string(&profile_json).json_to()?;
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
            serde_json::to_string(&self.version_json).json_to()?,
        )
        .await
        .path(json_file_path)?;
        Ok(())
    }

    pub async fn create_config_json(&self) -> Result<(), DownloadError> {
        #[allow(deprecated)]
        let config_json = InstanceConfigJson {
            java_override: None,
            ram_in_mb: DEFAULT_RAM_MB_FOR_INSTANCE,
            mod_type: "Vanilla".to_owned(),
            enable_logger: Some(true),
            java_args: None,
            game_args: None,
            is_classic_server: None,
            do_gc_tuning: None,
            close_on_start: None,
            is_server: Some(false),
            omniarchive: None,
        };
        let config_json = serde_json::to_string(&config_json).json_to()?;

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
        info!("Downloading version manifest JSON");
        if let Some(sender) = sender {
            _ = sender.send(DownloadProgress::DownloadingJsonManifest);
        }
        let manifest = Manifest::download().await?;

        let version =
            manifest
                .find_name(&version.name)
                .ok_or(DownloadError::VersionNotFoundInManifest(
                    version.name.clone(),
                ))?;

        info!("Downloading version details JSON");
        if let Some(sender) = sender {
            _ = sender.send(DownloadProgress::DownloadingVersionJson);
        }
        let json = file_utils::download_file_to_string(&version.url, false).await?;
        let json = serde_json::from_str(&json).json(json)?;
        Ok(json)
    }

    async fn new_get_instance_dir(instance_name: &str) -> Result<Option<PathBuf>, IoError> {
        info!("Initializing instance folder.");
        let instances_dir = LAUNCHER_DIR.join("instances");
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

    fn send_progress(&self, progress: DownloadProgress, print: bool) {
        if let Some(ref sender) = self.sender {
            if sender.send(progress).is_ok() {
                return;
            }
        }
        if print {
            pt!("{progress}");
        }
    }
}
