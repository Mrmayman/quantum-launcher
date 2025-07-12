use ql_core::{
    file_utils, info,
    json::{InstanceConfigJson, VersionDetails},
    pt, GenericProgress, InstanceSelection, IntoIoError, IntoJsonError, ListEntry, Loader,
    Progress,
};
use std::{
    path::{Path, PathBuf},
    sync::{
        mpsc::{Receiver, Sender},
        Arc,
    },
};
use tokio::fs;
use zip_extract::extract;

use crate::InstanceInfo;

use super::InstancePackageError;

pub const OUT_OF: usize = 4;

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
    sender: Option<Sender<GenericProgress>>,
) -> Result<Option<InstanceSelection>, InstancePackageError> {
    let temp_dir_obj = tempfile::TempDir::new().map_err(InstancePackageError::TempDir)?;
    let temp_dir = temp_dir_obj.path();

    pt!("Extracting zip to {temp_dir:?}");
    let zip_file = std::fs::File::open(&zip_path).path(&zip_path)?;
    if let Some(sender) = &sender {
        _ = sender.send(GenericProgress {
            done: 0,
            total: OUT_OF,
            message: Some("Extracting Archive...".to_owned()),
            has_finished: false,
        });
    }
    extract(zip_file, temp_dir, true)?;

    let try_ql = temp_dir.join("quantum-config.json");
    let try_mmc = temp_dir.join("mmc-pack.json");

    let instance = if let Ok(instance_info) = fs::read_to_string(&try_ql).await {
        Some(
            import_quantumlauncher(
                download_assets,
                temp_dir,
                instance_info,
                sender.map(Arc::new),
            )
            .await?,
        )
    } else if let Ok(mmc_pack) = fs::read_to_string(&try_mmc).await {
        Some(
            crate::multimc::import(download_assets, temp_dir, mmc_pack, sender.map(Arc::new))
                .await?,
        )
    } else {
        None
    };

    fs::remove_dir_all(&temp_dir).await.path(temp_dir)?;

    Ok(instance)
}

async fn import_quantumlauncher(
    download_assets: bool,
    temp_dir: &Path,
    instance_info: String,
    sender: Option<Arc<Sender<GenericProgress>>>,
) -> Result<InstanceSelection, InstancePackageError> {
    info!("Importing QuantumLauncher instance...");

    let instance_info: InstanceInfo = serde_json::from_str(&instance_info).json(instance_info)?;
    let version_json: VersionDetails = VersionDetails::load_from_path(temp_dir).await?;
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
        ql_servers::create_server(instance_info.instance_name, version, sender.as_deref()).await?;
    } else {
        let (d_send, d_recv) = std::sync::mpsc::channel();

        if let Some(sender) = sender.clone() {
            std::thread::spawn(|| {
                pipe_progress(d_recv, sender);
            });
        }

        ql_instances::create_instance(
            instance_info.instance_name,
            version,
            Some(d_send),
            download_assets,
        )
        .await?;
    }

    let instance_path = instance.get_instance_path();

    if let Ok(loader) = Loader::try_from(config_json.mod_type.as_str()) {
        ql_mod_manager::loaders::install_specified_loader(
            instance.clone(),
            loader,
            sender.clone(),
            None,
        )
        .await
        .map_err(InstancePackageError::Loader)?;
    }

    pt!("Copying packaged files");
    if let Some(sender) = &sender {
        _ = sender.send(GenericProgress {
            done: 2,
            total: OUT_OF,
            message: Some("Copying files...".to_owned()),
            has_finished: false,
        });
    }
    file_utils::copy_dir_recursive(temp_dir, &instance_path).await?;
    info!("Finished importing QuantumLauncher instance");
    Ok(instance)
}

pub fn pipe_progress<T: Progress>(rec: Receiver<T>, snd: Arc<Sender<GenericProgress>>) {
    for item in rec {
        _ = snd.send(item.into_generic());
    }
}
