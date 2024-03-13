use std::{
    fs::{self, File},
    io::Write,
    path::PathBuf,
};

use crate::{
    error::{LauncherError, LauncherResult},
    file_utils::{self, create_dir_if_not_exists},
};

use reqwest::blocking::Client;
use serde_json::Value;

const VERSIONS_JSON: &str = "https://launchermeta.mojang.com/mc/game/version_manifest.json";

pub fn launch(instance_name: &str) -> LauncherResult<()> {
    let launcher_dir = file_utils::get_launcher_dir()?;

    let instances_dir = launcher_dir.join("instances");
    file_utils::create_dir_if_not_exists(&instances_dir)?;

    if !instances_dir.join(instance_name).exists() {
        return Err(LauncherError::InstanceNotFound);
    }

    todo!()
}

pub fn create(instance_name: &str, version: String) -> LauncherResult<()> {
    println!("[info] Started creating instance.");

    let instance_dir = get_instance_dir(&instance_name)?;
    let network_client = Client::new();

    let version_json = download_version_json(&network_client, version)?;

    let asset_index_url = version_json["assetIndex"]["url"]
        .as_str()
        .expect("Could not find url field in version.assetIndex.url");
    let asset_index = download_json(&network_client, asset_index_url)?;
    create_dir_if_not_exists(&instance_dir.join("assets").join("indexes"))?;

    download_logging_config(&version_json, &network_client, &instance_dir)?;

    create_dir_if_not_exists(&instance_dir.join("libraries"))?;

    let libraries = version_json["libraries"]
        .as_array()
        .expect("Could not find libraries field in version.libraries");

    for library in libraries {
        let lib_name = library["downloads"]["artifact"]["path"]
            .as_str()
            .expect("Could not find path field in version.libraries[].downloads.artifact.path");

        let lib_file_path = instance_dir.join(PathBuf::from(lib_name));

        let lib_path = lib_file_path
            .parent()
            .expect(
                "Downloaded java library does not have parent module like the sun in com.sun.java",
            )
            .to_path_buf();

        let lib_url = library["downloads"]["artifact"]["url"]
            .as_str()
            .expect("Could not find field url in version.libraries[].downloads.artifact.url");

        let mut allowed: bool = true;

        if let Value::Array(ref rules_array) = library["rules"] {
            println!("Debug");
        }
    }

    todo!()
}

fn download_version_json(network_client: &Client, version: String) -> Result<Value, LauncherError> {
    println!("[info] Started downloading version manifest JSON.");
    let version_manifest_json = download_json(network_client, VERSIONS_JSON)?;
    let version = find_required_version(&version_manifest_json, &version)?;
    println!("[info] Started downloading version details JSON.");
    let version_json_url = version["url"]
        .as_str()
        .expect("Could not find url field in manifest.versions[n].url");
    let version_json = download_json(network_client, version_json_url)?;
    Ok(version_json)
}

fn download_logging_config(
    version_json: &Value,
    network_client: &Client,
    instance_dir: &PathBuf,
) -> Result<(), LauncherError> {
    println!("[info] Downloading logging configuration.");
    let log_file_name = version_json["logging"]["client"]["file"]["id"]
        .as_str()
        .expect("Could not find field id in version.logging.client.file.id");
    let log_config_name = format!("logging-{log_file_name}");
    let log_file_url = version_json["logging"]["client"]["file"]["url"]
        .as_str()
        .expect("Could not find field id in version.logging.client.file.url");
    let log_config = file_utils::download_file(&network_client, log_file_url)?;
    let mut file = File::create(instance_dir.join(log_config_name))?;
    file.write_all(log_config.as_bytes())?;
    Ok(())
}

fn get_instance_dir(instance_name: &str) -> LauncherResult<PathBuf> {
    println!("[info] Initializing instance folder.");
    let launcher_dir = file_utils::get_launcher_dir()?;
    let instances_dir = launcher_dir.join("instances");
    file_utils::create_dir_if_not_exists(&instances_dir)?;

    let current_instance_dir = instances_dir.join(instance_name);
    if current_instance_dir.exists() {
        return Err(LauncherError::InstanceAlreadyExists);
    }
    fs::create_dir_all(&current_instance_dir)?;

    Ok(current_instance_dir)
}

fn download_json(network_client: &Client, url: &str) -> LauncherResult<Value> {
    let json = file_utils::download_file(network_client, url)?;
    let result = serde_json::from_str::<serde_json::Value>(&json);
    match result {
        Ok(n) => Ok(n),
        Err(err) => Err(LauncherError::from(err)),
    }
}

fn find_required_version<'json>(
    manifest_json: &'json Value,
    version: &str,
) -> LauncherResult<&'json Value> {
    match manifest_json["versions"]
        .as_array()
        .expect("No versions array in version manifest")
        .iter()
        .find_map(|n| {
            let value = n["id"].as_str().expect("No id field in version manifest");
            if *value == *version {
                Some(n)
            } else {
                None
            }
        }) {
        Some(n) => Ok(n),
        None => Err(LauncherError::VersionNotFoundInManifest(version.to_owned())),
    }
}
