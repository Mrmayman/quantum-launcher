use ql_core::{
    IntoIoError, IntoJsonError, LAUNCHER_DIR, ListEntry, file_utils, info, json::VersionDetails, pt,
};
use std::path::PathBuf;
use tokio::fs;
use zip_extract::extract;

use crate::InstanceInfo;

use super::InstancePackageError;

/// Imports a Minecraft instance from a `.zip` file exported by the launcher.
///
/// This function performs the following:
/// 1. Extracts the ZIP archive to a temporary directory.
/// 2. Reads the `quantum-config.json` from the extracted directory to get instance metadata.
/// 3. Creates a new instance using the extracted configuration.
/// 4. Copies the extracted files to the main instances directory.
///
/// # Parameters
/// - `zip_path`: The path to the `.zip` archive to import. It must contain a `quantum-config.json` file inside the root of the zipped instance folder.
/// - `assets`: Whether to include additional assets during instance creation.
/// # Returns
/// A `Result` indicating success or containing an error if anything fails.
///
/// # Errors
/// - Returns an error if the ZIP file can't be opened or extracted.
/// - Returns an error if `quantum-config.json` is missing or malformed.
/// - Returns an error if instance creation or file copying fails.
///
/// # Safety Note
/// - Currently uses `.unwrap()` for the zip file stem extraction, which will panic on malformed file names.
///   Consider replacing with proper error handling in the future.
///
/// # Example
/// ```no_run
/// import_instance(Path::new("/path/to/exported_instance.zip"), true).await?;
/// ```
pub async fn import_instance(zip_path: PathBuf, assets: bool) -> Result<(), InstancePackageError> {
    info!("Importing QuantumLauncher instance...");
    let temp_dir_obj =
        tempdir::TempDir::new("ql_instance_import").map_err(InstancePackageError::TempDir)?;
    let temp_dir = temp_dir_obj.path();

    pt!("Extracting zip to temp dir: {temp_dir:?}");
    let zip_file = std::fs::File::open(&zip_path).path(&zip_path)?;
    extract(zip_file, &temp_dir, true)?;

    let instance_info: InstanceInfo = {
        let path = temp_dir.join("quantum-config.json");
        let file = fs::read_to_string(&path).await.path(path)?;
        serde_json::from_str(&file).json(file)?
    };

    let version_json: VersionDetails = {
        let path = temp_dir.join("details.json");
        let file = fs::read_to_string(&path).await.path(&path)?;
        serde_json::from_str(&file).json(file)?
    };

    // let config_json: InstanceConfigJson = {
    //     let path = temp_dir.join("config.json");
    //     let file = fs::read_to_string(&path).await.path(&path)?;
    //     serde_json::from_str(&file).json(file)?
    // };

    pt!("Name: {} ", instance_info.instance_name);
    pt!("Version : {}", version_json.id);
    pt!("Exceptions : {:?} ", instance_info.exceptions);

    let version = ListEntry {
        name: version_json.id.clone(),
        is_classic_server: instance_info.is_server && version_json.id.starts_with("c0."),
    };

    if instance_info.is_server {
        ql_servers::create_server(instance_info.instance_name, version, None).await?;
    } else {
        ql_instances::create_instance(instance_info.instance_name, version, None, assets).await?;
    }
    pt!("Copying packaged");
    file_utils::copy_dir_recursive(&temp_dir, &LAUNCHER_DIR.join("instances")).await?;
    pt!("Cleaning temporary files");
    fs::remove_dir_all(&temp_dir).await.path(temp_dir)?;
    info!("Finished importing instance");
    Ok(())
}
