pub mod constants;
mod library_downloader;
pub mod progress;

use std::{
    fmt::Display,
    path::{Path, PathBuf},
    sync::mpsc::{SendError, Sender},
};

use futures::StreamExt;
use indicatif::ProgressBar;
use reqwest::Client;
use serde_json::Value;
use tokio::sync::Mutex;
use zip_extract::ZipExtractError;

/// Limit on how many files to download concurrently.
const JOBS: usize = 64;

use crate::{
    err,
    error::IoError,
    file_utils::{self, RequestError},
    info,
    instance::launch::AssetRedownloadProgress,
    io_err,
    json_structs::{
        json_instance_config::InstanceConfigJson, json_manifest::Manifest,
        json_profiles::ProfileJson, json_version::VersionDetails, JsonDownloadError,
    },
};

use self::{constants::DEFAULT_RAM_MB_FOR_INSTANCE, progress::DownloadProgress};

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
        version: &str,
        sender: Option<Sender<DownloadProgress>>,
    ) -> Result<GameDownloader, DownloadError> {
        let Some(instance_dir) = GameDownloader::new_get_instance_dir(instance_name).await? else {
            return Err(DownloadError::InstanceAlreadyExists);
        };
        let network_client = Client::new();
        let version_json =
            GameDownloader::new_download_version_json(&network_client, version, &sender).await?;

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

        let jar_bytes = file_utils::download_file_to_bytes(
            &self.network_client,
            &self.version_json.downloads.client.url,
            false,
        )
        .await?;

        let version_dir = self
            .instance_dir
            .join(".minecraft")
            .join("versions")
            .join(&self.version_json.id);
        tokio::fs::create_dir_all(&version_dir)
            .await
            .map_err(io_err!(version_dir))?;

        let jar_path = version_dir.join(format!("{}.jar", self.version_json.id));
        tokio::fs::write(&jar_path, jar_bytes)
            .await
            .map_err(io_err!(jar_path))?;

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
                .map_err(io_err!(config_path))?;
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
            .map_err(io_err!(obj_folder))?;

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
            &self.network_client,
            &format!("{OBJECTS_URL}/{obj_id}/{obj_hash}"),
            false,
        )
        .await?;

        tokio::fs::write(&obj_file_path, &obj_data)
            .await
            .map_err(io_err!(obj_file_path))?;

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
        let asset_index =
            GameDownloader::download_json(&self.network_client, &self.version_json.assetIndex.url)
                .await?;

        let launcher_dir = file_utils::get_launcher_dir()?;

        let assets_dir = launcher_dir.join("assets");
        tokio::fs::create_dir_all(&assets_dir)
            .await
            .map_err(io_err!(assets_dir))?;

        if self.version_json.assetIndex.id == "legacy" {
            let legacy_path = assets_dir.join("legacy_assets");
            tokio::fs::create_dir_all(&legacy_path)
                .await
                .map_err(io_err!(assets_dir))?;

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
                    &legacy_path,
                    obj_id,
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
        } else {
            let current_assets_dir = assets_dir.join("dir");
            tokio::fs::create_dir_all(&current_assets_dir)
                .await
                .map_err(io_err!(current_assets_dir))?;

            let assets_indexes_path = current_assets_dir.join("indexes");
            tokio::fs::create_dir_all(&assets_indexes_path)
                .await
                .map_err(io_err!(assets_indexes_path))?;
            let assets_objects_path = current_assets_dir.join("objects");
            tokio::fs::create_dir_all(&assets_objects_path)
                .await
                .map_err(io_err!(assets_objects_path))?;

            let lock_path = current_assets_dir.join("download.lock");

            if lock_path.exists() {
                err!("Asset downloading previously interrupted?");
            }

            let lock_contents = "If you see this, the asset downloading hasn't finished. This will be deleted once finished.";
            tokio::fs::write(&lock_path, lock_contents)
                .await
                .map_err(io_err!(lock_path))?;

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

            tokio::fs::remove_file(&lock_path)
                .await
                .map_err(io_err!(lock_path))?;
        }
        if let Some(sender) = sender {
            sender.send(AssetRedownloadProgress::P3Done).unwrap();
        }
        Ok(())
    }

    async fn download_assets_legacy_fn(
        &self,
        legacy_path: &Path,
        obj_id: &str,
        object_data: &Value,
        bar: &ProgressBar,
        progress: &Mutex<usize>,
        objects_len: usize,
        sender: Option<&Sender<AssetRedownloadProgress>>,
    ) -> Result<(), DownloadError> {
        let file_path = legacy_path.join(obj_id);
        if let Some(parent) = file_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(io_err!(parent))?;
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
            &self.network_client,
            &format!("{OBJECTS_URL}/{obj_hash_sliced}/{obj_hash}"),
            false,
        )
        .await?;
        tokio::fs::write(&file_path, &obj_data)
            .await
            .map_err(io_err!(file_path))?;

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
        asset_index: &Value,
    ) -> Result<(), DownloadError> {
        let assets_indexes_json_path =
            assets_indexes_path.join(format!("{}.json", self.version_json.assetIndex.id));
        tokio::fs::write(
            &assets_indexes_json_path,
            asset_index.to_string().as_bytes(),
        )
        .await
        .map_err(io_err!(assets_indexes_json_path))?;
        Ok(())
    }

    pub async fn download_json(
        network_client: &Client,
        url: &str,
    ) -> Result<Value, JsonDownloadError> {
        let json = file_utils::download_file_to_string(network_client, url, false).await?;
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
            .map_err(io_err!(profile_json_path))?;

        Ok(())
    }

    pub async fn create_version_json(&self) -> Result<(), DownloadError> {
        let json_file_path = self.instance_dir.join("details.json");

        tokio::fs::write(
            &json_file_path,
            serde_json::to_string(&self.version_json)?.as_bytes(),
        )
        .await
        .map_err(io_err!(json_file_path))?;
        Ok(())
    }

    pub async fn create_config_json(&self) -> Result<(), DownloadError> {
        let config_json = InstanceConfigJson {
            java_override: None,
            ram_in_mb: DEFAULT_RAM_MB_FOR_INSTANCE,
            mod_type: "Vanilla".to_owned(),
            enable_logger: Some(true),
        };
        let config_json = serde_json::to_string(&config_json)?;

        let config_json_path = self.instance_dir.join("config.json");
        tokio::fs::write(&config_json_path, config_json)
            .await
            .map_err(io_err!(config_json_path))?;

        Ok(())
    }

    async fn new_download_version_json(
        network_client: &Client,
        version: &str,
        sender: &Option<Sender<DownloadProgress>>,
    ) -> Result<VersionDetails, DownloadError> {
        info!("Started downloading version manifest JSON.");
        if let Some(sender) = sender {
            sender.send(DownloadProgress::DownloadingJsonManifest)?;
        }
        let manifest = Manifest::download().await?;

        let Some(version) = manifest.versions.iter().find(|n| n.id == version) else {
            return Err(DownloadError::VersionNotFoundInManifest(version.to_owned()));
        };

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
            .map_err(io_err!(instances_dir))?;

        let current_instance_dir = instances_dir.join(instance_name);
        if current_instance_dir.exists() {
            return Ok(None);
        }
        tokio::fs::create_dir_all(&current_instance_dir)
            .await
            .map_err(io_err!(current_instance_dir))?;

        Ok(Some(current_instance_dir))
    }

    fn send_progress(&self, progress: DownloadProgress) -> Result<(), SendError<DownloadProgress>> {
        if let Some(ref sender) = self.sender {
            sender.send(progress)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum DownloadError {
    Json(serde_json::Error),
    Request(RequestError),
    Io(IoError),
    InstanceAlreadyExists,
    SendProgress(SendError<DownloadProgress>),
    VersionNotFoundInManifest(String),
    SerdeFieldNotFound(String),
    NativesExtractError(ZipExtractError),
    NativesOutsideDirRemove,
}

impl From<serde_json::Error> for DownloadError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

impl From<RequestError> for DownloadError {
    fn from(value: RequestError) -> Self {
        Self::Request(value)
    }
}

impl From<IoError> for DownloadError {
    fn from(value: IoError) -> Self {
        Self::Io(value)
    }
}

impl From<SendError<DownloadProgress>> for DownloadError {
    fn from(value: SendError<DownloadProgress>) -> Self {
        Self::SendProgress(value)
    }
}

impl From<JsonDownloadError> for DownloadError {
    fn from(value: JsonDownloadError) -> Self {
        match value {
            JsonDownloadError::RequestError(err) => DownloadError::from(err),
            JsonDownloadError::SerdeError(err) => DownloadError::from(err),
        }
    }
}

impl Display for DownloadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DownloadError::Json(err) => write!(f, "download error: json error {err}"),
            DownloadError::Request(err) => write!(f, "download error: {err}"),
            DownloadError::Io(err) => write!(f, "download error: {err}"),
            DownloadError::InstanceAlreadyExists => {
                write!(f, "download error: instance already exists")
            }
            DownloadError::SendProgress(err) => write!(f, "download error: send error: {err}"),
            DownloadError::VersionNotFoundInManifest(err) => write!(f, "download error: version not found in manifest {err}"),
            DownloadError::SerdeFieldNotFound(err) => write!(f, "download error: serde field not found \"{err}\""),
            DownloadError::NativesExtractError(err) => write!(f, "download error: could not extract native libraries: {err}"),
            DownloadError::NativesOutsideDirRemove => write!(f, "download error: tried to remove natives outside folder. POTENTIAL SECURITY RISK AVOIDED"),
        }
    }
}

pub async fn do_jobs<ResultType>(
    results: impl Iterator<Item = impl std::future::Future<Output = ResultType>>,
) -> Vec<ResultType> {
    let mut tasks = futures::stream::FuturesUnordered::new();
    let mut outputs = Vec::new();

    for result in results {
        tasks.push(result);
        if tasks.len() > JOBS {
            if let Some(task) = tasks.next().await {
                outputs.push(task);
            }
        }
    }

    while let Some(task) = tasks.next().await {
        outputs.push(task);
    }
    outputs
}
