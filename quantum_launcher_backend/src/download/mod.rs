pub mod constants;
mod library_downloader;
pub mod progress;

use std::{
    path::PathBuf,
    sync::mpsc::{SendError, Sender},
};

use reqwest::Client;
use serde_json::Value;

use crate::{
    download::constants::VERSIONS_JSON,
    error::{LauncherError, LauncherResult},
    file_utils,
    json_structs::{
        json_instance_config::InstanceConfigJson, json_manifest::Manifest,
        json_profiles::ProfileJson, json_version::VersionDetails,
    },
};

use self::{constants::DEFAULT_RAM_MB_FOR_INSTANCE, progress::DownloadProgress};

/// A struct that helps download a Minecraft instance.
///
/// # Example
/// ```
/// // progress_sender: Option<mspc::Sender<Progress>>
/// // Btw don't run this doctest! It will burn 600 MB of disk space.
/// // let game_downloader = GameDownloader::new("1.20.4", &version, progress_sender)?;
/// // game_downloader.download_jar()?;
/// // game_downloader.download_libraries()?;
/// // game_downloader.download_logging_config()?;
/// // game_downloader.download_assets()?;
///
/// // let mut json_file = File::create(game_downloader.instance_dir.join("details.json"))?;
/// // json_file.write_all(serde_json::to_string(&game_downloader.version_json)?.as_bytes())?;
/// ```
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
    /// `sender: Option<Sender<Progress>>` is an optional mspc::Sender
    /// that can be used if you are running this asynchronously or
    /// on a separate thread, and want to communicate progress with main thread.
    ///
    /// Leave as `None` if not required.
    pub async fn new(
        instance_name: &str,
        version: &str,
        sender: Option<Sender<DownloadProgress>>,
    ) -> LauncherResult<GameDownloader> {
        let instance_dir = GameDownloader::new_get_instance_dir(instance_name)?;
        let network_client = Client::new();
        let version_json =
            GameDownloader::new_download_version_json(&network_client, version, &sender).await?;

        Ok(Self {
            instance_dir,
            network_client,
            version_json,
            sender,
        })
    }

    pub async fn download_jar(&self) -> LauncherResult<()> {
        println!("[info] Downloading game jar file.");
        self.send_progress(DownloadProgress::DownloadingJar)?;

        let jar_bytes = file_utils::download_file_to_bytes(
            &self.network_client,
            &self.version_json.downloads.client.url,
        )
        .await?;

        let version_dir = self
            .instance_dir
            .join(".minecraft")
            .join("versions")
            .join(&self.version_json.id);
        std::fs::create_dir_all(&version_dir)
            .map_err(|err| LauncherError::IoError(err, version_dir.clone()))?;

        let jar_path = version_dir.join(format!("{}.jar", self.version_json.id));
        std::fs::write(&jar_path, &jar_bytes)
            .map_err(|err| LauncherError::IoError(err, jar_path))?;

        Ok(())
    }

    pub async fn download_logging_config(&self) -> Result<(), LauncherError> {
        if let Some(ref logging) = self.version_json.logging {
            println!("[info] Downloading logging configuration.");
            self.send_progress(DownloadProgress::DownloadingLoggingConfig)?;

            let log_config_name = format!("logging-{}", logging.client.file.id);

            let log_config =
                file_utils::download_file_to_string(&self.network_client, &logging.client.file.url)
                    .await?;

            let config_path = self.instance_dir.join(log_config_name);
            std::fs::write(&config_path, log_config.as_bytes())
                .map_err(|err| LauncherError::IoError(err, config_path))?;
        }
        Ok(())
    }

    pub async fn download_assets(&self) -> Result<(), LauncherError> {
        const OBJECTS_URL: &str = "https://resources.download.minecraft.net";

        println!("[info] Downloading assets.");

        let assets_indexes_path = self.instance_dir.join("assets").join("indexes");
        std::fs::create_dir_all(&assets_indexes_path)
            .map_err(|err| LauncherError::IoError(err, assets_indexes_path))?;
        let assets_objects_path = self.instance_dir.join("assets").join("objects");
        std::fs::create_dir_all(&assets_objects_path)
            .map_err(|err| LauncherError::IoError(err, assets_objects_path.clone()))?;

        let asset_index =
            GameDownloader::download_json(&self.network_client, &self.version_json.assetIndex.url)
                .await?;

        let assets_indexes_json_path = self
            .instance_dir
            .join("assets")
            .join("indexes")
            .join(format!("{}.json", self.version_json.assetIndex.id));

        std::fs::write(
            &assets_indexes_json_path,
            asset_index.to_string().as_bytes(),
        )
        .map_err(|err| LauncherError::IoError(err, assets_indexes_json_path))?;

        let objects = asset_index["objects"]
            .as_object()
            .ok_or(LauncherError::SerdeFieldNotFound("asset_index.objects"))?;
        let objects_len = objects.len();

        for (object_number, (_, object_data)) in objects.iter().enumerate() {
            let obj_hash =
                object_data["hash"]
                    .as_str()
                    .ok_or(LauncherError::SerdeFieldNotFound(
                        "asset_index.objects[].hash",
                    ))?;

            let obj_id = &obj_hash[0..2];

            println!("[info] Downloading asset {object_number}/{objects_len}");
            self.send_progress(DownloadProgress::DownloadingAssets {
                progress: object_number,
                out_of: objects_len,
            })?;

            let obj_folder = assets_objects_path.join(obj_id);
            std::fs::create_dir_all(&obj_folder)
                .map_err(|err| LauncherError::IoError(err, obj_folder.clone()))?;

            let obj_data = file_utils::download_file_to_bytes(
                &self.network_client,
                &format!("{}/{}/{}", OBJECTS_URL, obj_id, obj_hash),
            )
            .await?;

            let obj_file_path = obj_folder.join(obj_hash);

            std::fs::write(&obj_file_path, &obj_data)
                .map_err(|err| LauncherError::IoError(err, obj_file_path))?;
        }
        Ok(())
    }

    pub async fn download_json(network_client: &Client, url: &str) -> LauncherResult<Value> {
        let json = file_utils::download_file_to_string(network_client, url).await?;
        let result = serde_json::from_str::<serde_json::Value>(&json);
        match result {
            Ok(n) => Ok(n),
            Err(err) => Err(LauncherError::from(err)),
        }
    }

    pub fn create_profiles_json(&self) -> LauncherResult<()> {
        let profile_json = ProfileJson::default();

        let profile_json = serde_json::to_string(&profile_json)?;
        let profile_json_path = self
            .instance_dir
            .join(".minecraft")
            .join("launcher_profiles.json");
        std::fs::write(&profile_json_path, profile_json)
            .map_err(|err| LauncherError::IoError(err, profile_json_path))?;

        Ok(())
    }

    pub fn create_version_json(&self) -> LauncherResult<()> {
        let json_file_path = self.instance_dir.join("details.json");

        std::fs::write(
            &json_file_path,
            serde_json::to_string(&self.version_json)?.as_bytes(),
        )
        .map_err(|err| LauncherError::IoError(err, json_file_path.clone()))?;
        Ok(())
    }

    pub fn create_config_json(&self) -> LauncherResult<()> {
        let config_json = InstanceConfigJson {
            java_override: None,
            ram_in_mb: DEFAULT_RAM_MB_FOR_INSTANCE,
            mod_type: "Vanilla".to_owned(),
        };
        let config_json = serde_json::to_string(&config_json)?;

        let config_json_path = self.instance_dir.join("config.json");
        std::fs::write(&config_json_path, config_json)
            .map_err(|err| LauncherError::IoError(err, config_json_path))?;

        Ok(())
    }

    async fn new_download_version_json(
        network_client: &Client,
        version: &str,
        sender: &Option<Sender<DownloadProgress>>,
    ) -> LauncherResult<VersionDetails> {
        println!("[info] Started downloading version manifest JSON.");
        if let Some(sender) = sender {
            sender.send(DownloadProgress::DownloadingJsonManifest)?;
        }
        let manifest_json =
            file_utils::download_file_to_string(network_client, VERSIONS_JSON).await?;
        let manifest: Manifest = serde_json::from_str(&manifest_json)?;

        let version = match manifest.versions.iter().find(|n| n.id == version) {
            Some(n) => n,
            None => return Err(LauncherError::VersionNotFoundInManifest(version.to_owned())),
        };

        println!("[info] Started downloading version details JSON.");
        if let Some(sender) = sender {
            sender.send(DownloadProgress::DownloadingVersionJson)?;
        }
        let version_json =
            file_utils::download_file_to_string(network_client, &version.url).await?;
        let version_json = serde_json::from_str(&version_json)?;
        Ok(version_json)
    }

    fn new_get_instance_dir(instance_name: &str) -> LauncherResult<PathBuf> {
        println!("[info] Initializing instance folder.");
        let launcher_dir = file_utils::get_launcher_dir()?;
        let instances_dir = launcher_dir.join("instances");
        std::fs::create_dir_all(&instances_dir)
            .map_err(|err| LauncherError::IoError(err, instances_dir.clone()))?;

        let current_instance_dir = instances_dir.join(instance_name);
        if current_instance_dir.exists() {
            return Err(LauncherError::InstanceAlreadyExists);
        }
        std::fs::create_dir_all(&current_instance_dir)
            .map_err(|err| LauncherError::IoError(err, current_instance_dir.clone()))?;

        Ok(current_instance_dir)
    }

    fn send_progress(&self, progress: DownloadProgress) -> Result<(), SendError<DownloadProgress>> {
        if let Some(ref sender) = self.sender {
            sender.send(progress)?;
        }
        Ok(())
    }
}
