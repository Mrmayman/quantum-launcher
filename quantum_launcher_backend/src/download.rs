use std::{fs::File, io::Write, path::PathBuf};

use reqwest::blocking::Client;
use serde_json::Value;

use crate::{
    error::{LauncherError, LauncherResult},
    file_utils::{self, create_dir_if_not_exists},
    get,
};

const VERSIONS_JSON: &str = "https://launchermeta.mojang.com/mc/game/version_manifest.json";

#[cfg(target_os = "linux")]
const OS_NAME: &str = "linux";

#[cfg(target_os = "windows")]
const OS_NAME: &str = "windows";

#[cfg(target_os = "macos")]
const OS_NAME: &str = "osx";

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
const OS_NAME: &str = "unknown";

pub struct GameDownloader {
    pub instance_dir: PathBuf,
    pub version_json: Value,
    network_client: Client,
}

impl GameDownloader {
    pub fn new(instance_name: &str, version: &str) -> LauncherResult<GameDownloader> {
        let instance_dir = GameDownloader::new_get_instance_dir(instance_name)?;
        let network_client = Client::new();
        let version_json = GameDownloader::new_download_version_json(&network_client, version)?;

        Ok(Self {
            instance_dir,
            network_client,
            version_json,
        })
    }

    pub fn download_libraries(&self) -> Result<(), LauncherError> {
        println!("[info] Starting download of libraries.");
        create_dir_if_not_exists(&self.instance_dir.join("libraries"))?;

        let libraries = get!(
            self.version_json["libraries"].as_array(),
            "version.libraries"
        );
        let number_of_libraries = libraries.len();

        for (library_number, library) in libraries.iter().enumerate() {
            if !GameDownloader::download_libraries_library_is_allowed(library)? {
                continue;
            }

            let lib_name = get!(
                library["downloads"]["artifact"]["path"].as_str(),
                "version.libraries[].downloads.artifact.path"
            );
            let lib_file_path = self
                .instance_dir
                .join("libraries")
                .join(PathBuf::from(lib_name));
            let lib_dir_path = lib_file_path
                .parent()
                .expect(
                    "Downloaded java library does not have parent module like the sun in com.sun.java",
                )
                .to_path_buf();

            let lib_url = get!(
                library["downloads"]["artifact"]["url"].as_str(),
                "version.libraries[].downloads.artifact.url"
            );

            println!(
                "[info] Downloading library {library_number}/{number_of_libraries}: {lib_name}"
            );
            create_dir_if_not_exists(&lib_dir_path)?;
            let library_downloaded =
                file_utils::download_file_to_bytes(&self.network_client, lib_url)?;

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
        let jar_url = get!(
            self.version_json["downloads"]["client"]["url"].as_str(),
            "version.downloads.client.url"
        );
        let jar_bytes = file_utils::download_file_to_bytes(&self.network_client, jar_url)?;
        let mut jar_file = File::create(self.instance_dir.join("version.jar"))?;
        jar_file.write_all(&jar_bytes)?;

        Ok(())
    }

    pub fn download_logging_config(&self) -> Result<(), LauncherError> {
        println!("[info] Downloading logging configuration.");
        let log_file_name = get!(
            self.version_json["logging"]["client"]["file"]["id"].as_str(),
            "version.logging.client.file.id"
        );
        let log_config_name = format!("logging-{log_file_name}");
        let log_file_url = get!(
            self.version_json["logging"]["client"]["file"]["url"].as_str(),
            "version.logging.client.file.url"
        );

        let log_config = file_utils::download_file_to_string(&self.network_client, log_file_url)?;
        let mut file = File::create(self.instance_dir.join(log_config_name))?;
        file.write_all(log_config.as_bytes())?;
        Ok(())
    }

    pub fn download_assets(&self) -> Result<(), LauncherError> {
        const OBJECTS_URL: &str = "https://resources.download.minecraft.net";

        println!("[info] Downloading assets.");
        create_dir_if_not_exists(&self.instance_dir.join("assets").join("indexes"))?;
        let object_folder = self.instance_dir.join("assets").join("objects");
        create_dir_if_not_exists(&object_folder)?;

        let asset_index_url = get!(
            self.version_json["assetIndex"]["url"].as_str(),
            "version.assetIndex.url"
        );
        let asset_index = GameDownloader::download_json(&self.network_client, asset_index_url)?;

        let objects = get!(asset_index["objects"].as_object(), "asset_index.objects");
        let objects_len = objects.len();

        for (object_number, (_, object_data)) in objects.iter().enumerate() {
            let obj_hash = get!(object_data["hash"].as_str(), "asset_index.objects[].hash");
            let obj_id = &obj_hash[0..2];

            println!("[info] Downloading asset {object_number}/{objects_len}");
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
    fn new_download_version_json(network_client: &Client, version: &str) -> LauncherResult<Value> {
        println!("[info] Started downloading version manifest JSON.");
        let version_manifest_json = GameDownloader::download_json(network_client, VERSIONS_JSON)?;
        let version = GameDownloader::find_required_version(&version_manifest_json, version)?;

        println!("[info] Started downloading version details JSON.");
        let version_json_url = get!(version["url"].as_str(), "manifest.versions[].url");
        let version_json = GameDownloader::download_json(network_client, version_json_url)?;
        Ok(version_json)
    }

    fn find_required_version<'json>(
        manifest_json: &'json Value,
        version: &str,
    ) -> LauncherResult<&'json Value> {
        match get!(manifest_json["versions"].as_array(), "manifest.versions")
            .iter()
            .find(|n| {
                let value = n["id"].as_str().expect("No id field in version manifest");
                *value == *version
            }) {
            Some(n) => Ok(n),
            None => Err(LauncherError::VersionNotFoundInManifest(version.to_owned())),
        }
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

    fn download_libraries_library_is_allowed(library: &Value) -> LauncherResult<bool> {
        let mut allowed: bool = true;

        if let Value::Array(ref rules) = library["rules"] {
            allowed = false;

            for rule in rules {
                let os_name = get!(
                    rule["os"]["name"].as_str(),
                    "version.libraries[].rules[].os.name"
                );

                if os_name == OS_NAME {
                    let action = get!(
                        rule["action"].as_str(),
                        "version.libraries[].rules[].action"
                    );
                    allowed = action == "allow";
                }
            }
        }
        Ok(allowed)
    }
}
