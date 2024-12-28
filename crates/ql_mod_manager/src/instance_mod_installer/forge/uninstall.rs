use std::path::{Path, PathBuf};

use ql_core::{file_utils, InstanceSelection, IntoIoError};

use crate::{instance_mod_installer::change_instance_type, mod_manager::Loader};

use super::error::ForgeInstallError;

pub async fn uninstall_w(instance: InstanceSelection) -> Result<Loader, String> {
    match instance {
        InstanceSelection::Instance(instance) => uninstall_client(&instance).await,
        InstanceSelection::Server(instance) => uninstall_server(&instance).await,
    }
    .map_err(|err| err.to_string())
    .map(|()| Loader::Forge)
}

pub async fn uninstall_server_w(instance: String) -> Result<Loader, String> {
    uninstall_server(&instance)
        .await
        .map_err(|err| err.to_string())
        .map(|()| Loader::Forge)
}

pub async fn uninstall_client(instance: &str) -> Result<(), ForgeInstallError> {
    let launcher_dir = file_utils::get_launcher_dir()?;
    let instance_dir = launcher_dir.join("instances").join(instance);
    change_instance_type(&instance_dir, "Vanilla".to_owned()).await?;

    let forge_dir = instance_dir.join("forge");
    std::fs::remove_dir_all(&forge_dir).path(forge_dir)?;
    Ok(())
}

pub async fn uninstall_client_w(instance: String) -> Result<Loader, String> {
    uninstall_client(&instance)
        .await
        .map_err(|err| err.to_string())
        .map(|()| Loader::Forge)
}

pub async fn uninstall_server(instance: &str) -> Result<(), ForgeInstallError> {
    let launcher_dir = file_utils::get_launcher_dir()?;
    let instance_dir = launcher_dir.join("servers").join(instance);
    change_instance_type(&instance_dir, "Vanilla".to_owned()).await?;

    if let Some(forge_shim_file) = find_forge_shim_file(&instance_dir) {
        tokio::fs::remove_file(&forge_shim_file)
            .await
            .path(forge_shim_file)?;
    }

    let libraries_dir = instance_dir.join("libraries");
    tokio::fs::remove_dir_all(&libraries_dir)
        .await
        .path(libraries_dir)?;

    delete_file(&instance_dir.join("run.sh")).await?;
    delete_file(&instance_dir.join("run.bat")).await?;
    delete_file(&instance_dir.join("user_jvm_args.txt")).await?;
    delete_file(&instance_dir.join("README.txt")).await?;

    Ok(())
}

async fn delete_file(run_sh_path: &Path) -> Result<(), ForgeInstallError> {
    if run_sh_path.exists() {
        tokio::fs::remove_file(&run_sh_path)
            .await
            .path(run_sh_path)?;
    }
    Ok(())
}

fn find_forge_shim_file(dir: &Path) -> Option<PathBuf> {
    if !dir.is_dir() {
        return None; // Ensure the path is a directory
    }

    for entry in (std::fs::read_dir(dir).ok()?).flatten() {
        let path = entry.path();

        if path.is_file() {
            if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
                if file_name.starts_with("forge-") && file_name.ends_with("-shim.jar") {
                    return Some(path);
                }
            }
        }
    }

    None // Return None if no matching file is found
}
