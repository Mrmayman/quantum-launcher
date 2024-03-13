use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};

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

pub fn libraries(
    instance_dir: &Path,
    version_json: &Value,
    network_client: &Client,
) -> Result<(), LauncherError> {
    println!("[info] Starting download of libraries.");
    create_dir_if_not_exists(&instance_dir.join("libraries"))?;

    let libraries = get!(version_json["libraries"].as_array(), "version.libraries");
    let number_of_libraries = libraries.len();

    for (library_number, library) in libraries.iter().enumerate() {
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

        if !allowed {
            continue;
        }

        let lib_name = get!(
            library["downloads"]["artifact"]["path"].as_str(),
            "version.libraries[].downloads.artifact.path"
        );

        let lib_file_path = instance_dir.join(PathBuf::from(lib_name));

        let lib_path = lib_file_path
            .parent()
            .expect(
                "Downloaded java library does not have parent module like the sun in com.sun.java",
            )
            .to_path_buf();

        let lib_url = get!(
            library["downloads"]["artifact"]["url"].as_str(),
            "version.libraries[].downloads.artifact.url"
        );

        println!("[info] Downloading library {lib_name}: {library_number} / {number_of_libraries}");

        create_dir_if_not_exists(&lib_path)?;
        let library_downloaded = file_utils::download_file_to_bytes(network_client, lib_url)?;

        let mut file = File::create(lib_file_path)?;
        file.write_all(&library_downloaded)?;

        // According to the reference implementation, I also download natives.
        // At library.natives field.
        // However this field doesn't exist for the versions I tried so I'm skipping this.
    }
    Ok(())
}

pub fn version_json(network_client: &Client, version: String) -> Result<Value, LauncherError> {
    println!("[info] Started downloading version manifest JSON.");
    let version_manifest_json = json(network_client, VERSIONS_JSON)?;
    let version = find_required_version(&version_manifest_json, &version)?;
    println!("[info] Started downloading version details JSON.");
    let version_json_url = get!(version["url"].as_str(), "manifest.versions[].url");
    let version_json = json(network_client, version_json_url)?;
    Ok(version_json)
}

pub fn logging_config(
    version_json: &Value,
    network_client: &Client,
    instance_dir: &Path,
) -> Result<(), LauncherError> {
    println!("[info] Downloading logging configuration.");
    let log_file_name = get!(
        version_json["logging"]["client"]["file"]["id"].as_str(),
        "version.logging.client.file.id"
    );
    let log_config_name = format!("logging-{log_file_name}");
    let log_file_url = get!(
        version_json["logging"]["client"]["file"]["url"].as_str(),
        "version.logging.client.file.url"
    );

    let log_config = file_utils::download_file_to_string(network_client, log_file_url)?;
    let mut file = File::create(instance_dir.join(log_config_name))?;
    file.write_all(log_config.as_bytes())?;
    Ok(())
}

pub fn json(network_client: &Client, url: &str) -> LauncherResult<Value> {
    let json = file_utils::download_file_to_string(network_client, url)?;
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
