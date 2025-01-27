use ql_core::{file_utils, info, json::FabricJSON, InstanceSelection, IntoIoError};

use crate::{loaders::change_instance_type, mod_manager::Loader};

use super::error::FabricInstallError;

pub async fn uninstall_server_w(server_name: String) -> Result<(), String> {
    uninstall_server(&server_name)
        .await
        .map_err(|err| err.to_string())
}

pub async fn uninstall_server(server_name: &str) -> Result<(), FabricInstallError> {
    let server_dir = file_utils::get_launcher_dir()?
        .join("servers")
        .join(server_name);

    info!("Uninstalling fabric from server: {server_name}");

    let json_path = server_dir.join("fabric.json");
    let json = tokio::fs::read_to_string(&json_path)
        .await
        .path(&json_path)?;
    let json: FabricJSON = serde_json::from_str(&json)?;
    tokio::fs::remove_file(&json_path).await.path(&json_path)?;

    let launch_jar_path = server_dir.join("fabric-server-launch.jar");
    tokio::fs::remove_file(&launch_jar_path)
        .await
        .path(&launch_jar_path)?;

    let libraries_dir = server_dir.join("libraries");
    for library in &json.libraries {
        let library_path = libraries_dir.join(library.get_path());
        tokio::fs::remove_file(&library_path)
            .await
            .path(&library_path)?;
    }

    let properties_path = server_dir.join("fabric-server-launcher.properties");
    if properties_path.exists() {
        tokio::fs::remove_file(&properties_path)
            .await
            .path(&properties_path)?;
    }

    change_instance_type(&server_dir, "Vanilla".to_owned()).await?;
    info!("Finished uninstalling fabric");

    Ok(())
}

pub async fn uninstall_client(instance_name: &str) -> Result<(), FabricInstallError> {
    let launcher_dir = file_utils::get_launcher_dir()?;
    let instance_dir = launcher_dir.join("instances").join(instance_name);

    let lock_path = instance_dir.join("fabric_uninstall.lock");
    tokio::fs::write(
        &lock_path,
        "If you see this, fabric was not uninstalled correctly.",
    )
    .await
    .path(&lock_path)?;

    let fabric_json_path = instance_dir.join("fabric.json");
    let fabric_json = tokio::fs::read_to_string(&fabric_json_path)
        .await
        .path(&fabric_json_path)?;
    let fabric_json: FabricJSON = serde_json::from_str(&fabric_json)?;

    tokio::fs::remove_file(&fabric_json_path)
        .await
        .path(fabric_json_path)?;

    let libraries_dir = instance_dir.join("libraries");

    for library in &fabric_json.libraries {
        let library_path = libraries_dir.join(library.get_path());
        tokio::fs::remove_file(&library_path)
            .await
            .path(library_path)?;
    }

    change_instance_type(&instance_dir, "Vanilla".to_owned()).await?;

    tokio::fs::remove_file(&lock_path).await.path(lock_path)?;
    Ok(())
}

pub async fn uninstall_client_w(instance_name: String) -> Result<Loader, String> {
    uninstall_client(&instance_name)
        .await
        .map_err(|err| err.to_string())
        .map(|()| Loader::Fabric)
}

pub async fn uninstall_w(instance_name: InstanceSelection) -> Result<Loader, String> {
    match instance_name {
        InstanceSelection::Instance(n) => uninstall_client(&n).await,
        InstanceSelection::Server(n) => uninstall_server(&n).await,
    }
    .map_err(|err| err.to_string())
    .map(|()| Loader::Fabric)
}
