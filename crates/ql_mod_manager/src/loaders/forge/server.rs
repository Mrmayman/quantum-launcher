use ql_core::{InstanceSelection, IntoIoError};

use crate::loaders::{change_instance_type, forge::ForgeInstaller};

use super::{error::ForgeInstallError, ForgeInstallProgress};

pub async fn install_server_w(
    instance_name: String,
    j_progress: Option<std::sync::mpsc::Sender<ql_core::GenericProgress>>,
    f_progress: Option<std::sync::mpsc::Sender<ForgeInstallProgress>>,
) -> Result<(), String> {
    install_server(instance_name, j_progress, f_progress)
        .await
        .map_err(|e| e.to_string())
}

pub async fn install_server(
    instance_name: String,
    j_progress: Option<std::sync::mpsc::Sender<ql_core::GenericProgress>>,
    f_progress: Option<std::sync::mpsc::Sender<ForgeInstallProgress>>,
) -> Result<(), ForgeInstallError> {
    if let Some(progress) = &f_progress {
        let _ = progress.send(ForgeInstallProgress::P1Start);
    }

    let installer =
        ForgeInstaller::new(f_progress, InstanceSelection::Server(instance_name)).await?;

    let (_, installer_name, installer_path) = installer.download_forge_installer().await?;

    installer.run_installer(j_progress, &installer_name).await?;

    tokio::fs::remove_file(&installer_path)
        .await
        .path(installer_path)?;
    let delete_path = installer.forge_dir.join("ClientInstaller.java");
    tokio::fs::remove_file(&delete_path)
        .await
        .path(delete_path)?;
    let delete_path = installer.forge_dir.join("ClientInstaller.class");
    tokio::fs::remove_file(&delete_path)
        .await
        .path(delete_path)?;

    change_instance_type(&installer.instance_dir, "Forge".to_owned()).await?;
    installer.remove_lock()?;

    Ok(())
}
