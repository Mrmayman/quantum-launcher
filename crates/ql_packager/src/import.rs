use ql_core::{
    InstanceSelection, IntoIoError, IntoJsonError, ListEntry, Loader, err, file_utils, info,
    json::{InstanceConfigJson, VersionDetails},
    pt,
};
use std::path::{Path, PathBuf};
use tokio::fs;
use zip_extract::extract;

use crate::{InstanceInfo, multimc::MmcPack};

use super::InstancePackageError;

/// Imports a Minecraft instance from a `.zip` file exported by the launcher.
///
/// This function performs the following:
/// 1. Extracts the ZIP archive to a temporary directory.
/// 2. Reads the `quantum-config.json` from the extracted directory to get instance metadata.
/// 3. Creates a new instance using the extracted configuration.
/// 4. Copies the extracted files to the main instances directory.
///
/// Finally, it returns a bool indicating whether the file
/// was an actual packaged instance or not. You can use this
/// for fuzzy file detection, running this function and running
/// something else if it's `false`.
///
/// # Parameters
/// - `zip_path`: The path to the `.zip` archive to import. It must contain a `quantum-config.json` file inside the root of the zipped instance folder.
/// - `assets`: Whether to include additional assets during instance creation.
/// # Returns
/// A `Result` indicating success or containing an error if anything fails.
///
/// # Errors
/// - if ZIP file can't be opened or extracted
/// - if `quantum-config.json` or `details.json` are missing or malformed
/// - if instance creation (downloading) fails
/// - if user doesn't have permission to access launcher dir
pub async fn import_instance(
    zip_path: PathBuf,
    download_assets: bool,
) -> Result<bool, InstancePackageError> {
    let temp_dir_obj = tempfile::TempDir::new().map_err(InstancePackageError::TempDir)?;
    let temp_dir = temp_dir_obj.path();

    pt!("Extracting zip: {temp_dir:?}");
    let zip_file = std::fs::File::open(&zip_path).path(&zip_path)?;
    extract(zip_file, temp_dir, true)?;

    let try_ql = temp_dir.join("quantum-config.json");
    let try_mmc = temp_dir.join("mmc-pack.json");

    let mut is_instance = true;

    if let Ok(instance_info) = fs::read_to_string(&try_ql).await {
        import_quantumlauncher(download_assets, temp_dir, instance_info).await?;
    } else if let Ok(mmc_pack) = fs::read_to_string(&try_mmc).await {
        import_multimc(download_assets, temp_dir, mmc_pack).await?;
    } else {
        is_instance = false;
    }

    fs::remove_dir_all(&temp_dir).await.path(temp_dir)?;

    Ok(is_instance)
}

async fn import_quantumlauncher(
    download_assets: bool,
    temp_dir: &Path,
    instance_info: String,
) -> Result<(), InstancePackageError> {
    info!("Importing QuantumLauncher instance...");

    let instance_info: InstanceInfo = serde_json::from_str(&instance_info).json(instance_info)?;
    let version_json: VersionDetails = {
        let path = temp_dir.join("details.json");
        let file = fs::read_to_string(&path).await.path(&path)?;
        serde_json::from_str(&file).json(file)?
    };
    let config_json: InstanceConfigJson = {
        let path = temp_dir.join("config.json");
        let file = fs::read_to_string(&path).await.path(&path)?;
        serde_json::from_str(&file).json(file)?
    };

    let instance = InstanceSelection::new(&instance_info.instance_name, instance_info.is_server);

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
        ql_instances::create_instance(
            instance_info.instance_name,
            version,
            None, // TODO
            download_assets,
        )
        .await?;
    }

    let instance_path = instance.get_instance_path();

    if let Ok(loader) = Loader::try_from(config_json.mod_type.as_str()) {
        ql_mod_manager::loaders::install_specified_loader(instance, loader)
            .await
            .map_err(InstancePackageError::Loader)?;
    }

    pt!("Copying packaged");
    file_utils::copy_dir_recursive(temp_dir, &instance_path).await?;
    Ok(())
}

async fn import_multimc(
    download_assets: bool,
    temp_dir: &Path,
    mmc_pack: String,
) -> Result<(), InstancePackageError> {
    info!("Importing MultiMC instance...");
    let mmc_pack: MmcPack = serde_json::from_str(&mmc_pack).json(mmc_pack)?;
    let ini_path = temp_dir.join("instance.cfg");
    let ini = ini::Ini::load_from_file(&ini_path)?;
    let instance_name = ini
        .get_from(Some("General"), "name")
        .ok_or_else(|| {
            InstancePackageError::IniFieldMissing("General".to_owned(), "name".to_owned())
        })?
        .to_owned();
    let instance_selection = InstanceSelection::new(&instance_name, false);

    for component in &mmc_pack.components {
        match component.cachedName.as_str() {
            "Minecraft" => {
                let version = ListEntry {
                    name: component.version.clone(),
                    is_classic_server: false,
                };

                ql_instances::create_instance(
                    instance_name.clone(),
                    version,
                    None, // TODO
                    download_assets,
                )
                .await?;
            }
            "LWJGL 2" | "LWJGL 3" => {}
            name => err!("Unknown component (in MultiMC instance): {name}"),
        }
    }

    let src = temp_dir.join("minecraft");
    let dst = instance_selection.get_dot_minecraft_path();
    file_utils::copy_dir_recursive(&src, &dst).await?;

    Ok(())
}
