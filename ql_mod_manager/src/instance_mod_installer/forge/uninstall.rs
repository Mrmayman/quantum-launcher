use ql_instances::{file_utils, io_err};

use crate::{instance_mod_installer::change_instance_type, mod_manager::Loader};

use super::error::ForgeInstallError;

pub async fn uninstall(instance: &str) -> Result<(), ForgeInstallError> {
    let launcher_dir = file_utils::get_launcher_dir()?;
    let instance_dir = launcher_dir.join("instances").join(instance);
    change_instance_type(&instance_dir, "Vanilla".to_owned())?;

    let forge_dir = instance_dir.join("forge");
    std::fs::remove_dir_all(&forge_dir).map_err(io_err!(forge_dir))?;
    Ok(())
}

pub async fn uninstall_wrapped(instance: String) -> Result<Loader, String> {
    uninstall(&instance)
        .await
        .map_err(|err| err.to_string())
        .map(|_| Loader::Forge)
}
