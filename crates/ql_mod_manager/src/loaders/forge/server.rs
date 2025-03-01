use ql_core::{InstanceSelection, IntoIoError};

use crate::loaders::{change_instance_type, forge::ForgeInstaller};

use super::{error::ForgeInstallError, ForgeInstallProgress};

pub async fn install_server(
    instance_name: String,
    j_progress: Option<std::sync::mpsc::Sender<ql_core::GenericProgress>>,
    f_progress: Option<std::sync::mpsc::Sender<ForgeInstallProgress>>,
) -> Result<(), ForgeInstallError> {
    if let Some(progress) = &f_progress {
        _ = progress.send(ForgeInstallProgress::P1Start);
    }

    let installer =
        ForgeInstaller::new(f_progress, InstanceSelection::Server(instance_name)).await?;

    let (_, installer_name, installer_path) = installer.download_forge_installer().await?;

    installer
        .run_installer(j_progress.as_ref(), &installer_name)
        .await?;

    tokio::fs::remove_file(&installer_path)
        .await
        .path(installer_path)?;

    installer.delete("ClientInstaller.java").await?;
    installer.delete("ClientInstaller.class").await?;
    installer.delete("ForgeInstaller.java").await?;
    installer.delete("ForgeInstaller.class").await?;

    change_instance_type(&installer.instance_dir, "Forge".to_owned()).await?;
    installer.remove_lock().await?;

    Ok(())
}
