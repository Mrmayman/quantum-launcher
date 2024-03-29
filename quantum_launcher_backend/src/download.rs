use std::{
    fs::File,
    io::Write,
    path::PathBuf,
    sync::mpsc::{SendError, Sender},
};

use reqwest::blocking::Client;
use serde_json::Value;

use crate::{
    error::{LauncherError, LauncherResult},
    file_utils::{self, create_dir_if_not_exists},
    json_structs::{
        json_manifest::Manifest,
        json_version::{self, VersionDetails},
    },
};

pub(crate) const VERSIONS_JSON: &str =
    "https://launchermeta.mojang.com/mc/game/version_manifest.json";

#[cfg(target_os = "linux")]
const OS_NAME: &str = "linux";

#[cfg(target_os = "windows")]
const OS_NAME: &str = "windows";

#[cfg(target_os = "macos")]
const OS_NAME: &str = "osx";

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
const OS_NAME: &str = "unknown";

#[derive(Debug)]
pub enum Progress {
    Started,
    DownloadingJsonManifest,
    DownloadingVersionJson,
    DownloadingAssets { progress: usize, out_of: usize },
    DownloadingLibraries { progress: usize, out_of: usize },
    DownloadingJar,
    DownloadingLoggingConfig,
}

/// A struct that helps download a Minecraft instance.
///
/// # Example
/// ```
/// // progress_sender: Option<mspc::Sender<Progress>>
/// // Btw don't run this doctest! It will burn 600 MB of disk space.
/// let game_downloader = GameDownloader::new("1.20.4", &version, progress_sender)?;
/// game_downloader.download_jar()?;
/// game_downloader.download_libraries()?;
/// game_downloader.download_logging_config()?;
/// game_downloader.download_assets()?;
///
/// let mut json_file = File::create(game_downloader.instance_dir.join("details.json"))?;
/// json_file.write_all(serde_json::to_string(&game_downloader.version_json)?.as_bytes())?;
/// ```
pub struct GameDownloader {
    pub instance_dir: PathBuf,
    pub version_json: VersionDetails,
    network_client: Client,
    sender: Option<Sender<Progress>>,
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
    pub fn new(
        instance_name: &str,
        version: &str,
        sender: Option<Sender<Progress>>,
    ) -> LauncherResult<GameDownloader> {
        let instance_dir = GameDownloader::new_get_instance_dir(instance_name)?;
        let network_client = Client::new();
        let version_json =
            GameDownloader::new_download_version_json(&network_client, version, &sender)?;

        Ok(Self {
            instance_dir,
            network_client,
            version_json,
            sender,
        })
    }

    pub fn download_libraries(&self) -> Result<(), LauncherError> {
        println!("[info] Starting download of libraries.");
        create_dir_if_not_exists(&self.instance_dir.join("libraries"))?;

        let number_of_libraries = self.version_json.libraries.len();

        for (library_number, library) in self.version_json.libraries.iter().enumerate() {
            self.send_progress(Progress::DownloadingLibraries {
                progress: library_number,
                out_of: number_of_libraries,
            })?;

            if !GameDownloader::download_libraries_library_is_allowed(library) {
                println!(
                    "[info] Skipping library {}",
                    serde_json::to_string_pretty(&library)?
                );
                continue;
            }

            let lib_file_path = self
                .instance_dir
                .join("libraries")
                .join(PathBuf::from(&library.downloads.artifact.path));
            let lib_dir_path = lib_file_path
                .parent()
                .expect(
                    "Downloaded java library does not have parent module like the sun in com.sun.java",
                )
                .to_path_buf();

            println!(
                "[info] Downloading library {library_number}/{number_of_libraries}: {}",
                library.downloads.artifact.path
            );
            create_dir_if_not_exists(&lib_dir_path)?;
            let library_downloaded = file_utils::download_file_to_bytes(
                &self.network_client,
                &library.downloads.artifact.url,
            )?;

            let mut file = File::create(lib_file_path)?;
            file.write_all(&library_downloaded)?;

            // According to the reference implementation, I also download natives.
            // At library.natives field.
            // However this field doesn't exist for the versions I tried so I'm skipping this.
        }
        Ok(())
    }

    pub fn download_jar(&self) -> LauncherResult<()> {
        println!("[info] Downloading game jar file.");
        self.send_progress(Progress::DownloadingJar)?;

        let jar_bytes = file_utils::download_file_to_bytes(
            &self.network_client,
            &self.version_json.downloads.client.url,
        )?;
        let mut jar_file = File::create(self.instance_dir.join("version.jar"))?;
        jar_file.write_all(&jar_bytes)?;

        Ok(())
    }

    pub fn download_logging_config(&self) -> Result<(), LauncherError> {
        if let Some(ref logging) = self.version_json.logging {
            println!("[info] Downloading logging configuration.");
            self.send_progress(Progress::DownloadingLoggingConfig)?;

            let log_config_name = format!("logging-{}", logging.client.file.id);

            let log_config = file_utils::download_file_to_string(
                &self.network_client,
                &logging.client.file.url,
            )?;
            let mut file = File::create(self.instance_dir.join(log_config_name))?;
            file.write_all(log_config.as_bytes())?;
        }
        Ok(())
    }

    pub fn download_assets(&self) -> Result<(), LauncherError> {
        const OBJECTS_URL: &str = "https://resources.download.minecraft.net";

        println!("[info] Downloading assets.");
        create_dir_if_not_exists(&self.instance_dir.join("assets").join("indexes"))?;
        let object_folder = self.instance_dir.join("assets").join("objects");
        create_dir_if_not_exists(&object_folder)?;

        let asset_index =
            GameDownloader::download_json(&self.network_client, &self.version_json.assetIndex.url)?;
        let mut file = File::create(
            self.instance_dir
                .join("assets")
                .join("indexes")
                .join(format!("{}.json", self.version_json.assetIndex.id)),
        )?;
        file.write_all(asset_index.to_string().as_bytes())?;

        let objects = if let Some(value) = asset_index["objects"].as_object() {
            value
        } else {
            return Err(LauncherError::SerdeFieldNotFound("asset_index.objects"));
        };
        let objects_len = objects.len();

        for (object_number, (_, object_data)) in objects.iter().enumerate() {
            let obj_hash = if let Some(value) = object_data["hash"].as_str() {
                value
            } else {
                return Err(LauncherError::SerdeFieldNotFound(
                    "asset_index.objects[].hash",
                ));
            };
            let obj_id = &obj_hash[0..2];

            println!("[info] Downloading asset {object_number}/{objects_len}");
            self.send_progress(Progress::DownloadingAssets {
                progress: object_number,
                out_of: objects_len,
            })?;

            let obj_folder = object_folder.join(obj_id);
            create_dir_if_not_exists(&obj_folder)?;

            let obj_data = file_utils::download_file_to_bytes(
                &self.network_client,
                &format!("{}/{}/{}", OBJECTS_URL, obj_id, obj_hash),
            )?;
            let mut file = File::create(obj_folder.join(obj_hash))?;
            file.write_all(&obj_data)?;
        }
        Ok(())
    }

    pub fn download_json(network_client: &Client, url: &str) -> LauncherResult<Value> {
        let json = file_utils::download_file_to_string(network_client, url)?;
        let result = serde_json::from_str::<serde_json::Value>(&json);
        match result {
            Ok(n) => Ok(n),
            Err(err) => Err(LauncherError::from(err)),
        }
    }
}

impl GameDownloader {
    fn new_download_version_json(
        network_client: &Client,
        version: &str,
        sender: &Option<Sender<Progress>>,
    ) -> LauncherResult<VersionDetails> {
        println!("[info] Started downloading version manifest JSON.");
        if let Some(sender) = sender {
            sender.send(Progress::DownloadingJsonManifest)?;
        }
        let manifest_json = file_utils::download_file_to_string(network_client, VERSIONS_JSON)?;
        let manifest: Manifest = serde_json::from_str(&manifest_json)?;

        let version = match manifest.versions.iter().find(|n| n.id == version) {
            Some(n) => n,
            None => return Err(LauncherError::VersionNotFoundInManifest(version.to_owned())),
        };

        println!("[info] Started downloading version details JSON.");
        if let Some(sender) = sender {
            sender.send(Progress::DownloadingVersionJson)?;
        }
        let version_json = file_utils::download_file_to_string(network_client, &version.url)?;
        let version_json = serde_json::from_str(&version_json)?;
        Ok(version_json)
    }

    fn new_get_instance_dir(instance_name: &str) -> LauncherResult<PathBuf> {
        println!("[info] Initializing instance folder.");
        let launcher_dir = file_utils::get_launcher_dir()?;
        let instances_dir = launcher_dir.join("instances");
        file_utils::create_dir_if_not_exists(&instances_dir)?;

        let current_instance_dir = instances_dir.join(instance_name);
        if current_instance_dir.exists() {
            return Err(LauncherError::InstanceAlreadyExists);
        }
        std::fs::create_dir_all(&current_instance_dir)?;

        Ok(current_instance_dir)
    }

    fn download_libraries_library_is_allowed(library: &json_version::Library) -> bool {
        let mut allowed: bool = true;

        if let Some(ref rules) = library.rules {
            allowed = false;

            for rule in rules {
                if rule.os.name == OS_NAME {
                    allowed = rule.action == "allow";
                }
            }
        }
        allowed
    }

    fn send_progress(&self, progress: Progress) -> Result<(), SendError<Progress>> {
        if let Some(ref sender) = self.sender {
            sender.send(progress)?;
        }
        Ok(())
    }
}
