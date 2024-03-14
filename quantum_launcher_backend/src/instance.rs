use std::{
    fs::{self, File},
    io::Write,
    path::PathBuf,
};

use crate::{
    download,
    error::{LauncherError, LauncherResult},
    file_utils::{self, create_dir_if_not_exists},
    get,
};

use reqwest::blocking::Client;

const OBJECTS_URL: &str = "https://resources.download.minecraft.net";

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

    let instance_dir = get_instance_dir(instance_name)?;
    let network_client = Client::new();

    let version_json = download::version_json(&network_client, version)?;

    download::logging_config(&version_json, &network_client, &instance_dir)?;

    download::libraries(&instance_dir, &version_json, &network_client)?;

    let asset_index_url = if let Some(url) = version_json["assetIndex"]["url"].as_str() {
        url
    } else {
        return Err(LauncherError::SerdeFieldNotFound("version.assetIndex.url"));
    };
    let asset_index = download::json(&network_client, asset_index_url)?;

    create_dir_if_not_exists(&instance_dir.join("assets").join("indexes"))?;

    let object_folder = instance_dir.join("assets").join("objects");
    create_dir_if_not_exists(&object_folder)?;

    let objects = get!(asset_index["objects"].as_array(), "asset_index.objects");

    for object in objects {
        let obj_hash = get!(object["hash"].as_str(), "asset_index.objects[].hash");
        let obj_id = &obj_hash[0..2];

        let obj_folder = object_folder.join(obj_id);
        create_dir_if_not_exists(&obj_folder)?;

        let obj_data = file_utils::download_file_to_bytes(
            &network_client,
            &format!("{}/{}/{}", OBJECTS_URL, obj_id, obj_hash),
        )?;
        let mut file = File::create(obj_folder.join(&obj_hash))?;
        file.write_all(&obj_data)?;
    }

    todo!()
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
