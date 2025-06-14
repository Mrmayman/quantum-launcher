use std::path::Path;

use ql_core::{
    info, json::FabricJSON, InstanceSelection, IntoIoError, IntoJsonError, IoError, LAUNCHER_DIR,
};

use crate::loaders::change_instance_type;

use super::error::FabricInstallError;

async fn delete(server_dir: &Path, name: &str) -> Result<(), IoError> {
    let path = server_dir.join(name);
    if path.exists() {
        tokio::fs::remove_file(&path).await.path(&path)?;
    }

    Ok(())
}

pub async fn uninstall_server(server_name: String) -> Result<(), FabricInstallError> {
    let server_dir = LAUNCHER_DIR.join("servers").join(&server_name);

    info!("Uninstalling fabric from server: {server_name}");

    delete(&server_dir, "fabric-server-launch.jar").await?;
    delete(&server_dir, "fabric-server-launcher.properties").await?;

    let json_path = server_dir.join("fabric.json");
    if json_path.exists() {
        let json = tokio::fs::read_to_string(&json_path)
            .await
            .path(&json_path)?;
        let json: FabricJSON = serde_json::from_str(&json).json(json)?;
        tokio::fs::remove_file(&json_path).await.path(&json_path)?;

        let libraries_dir = server_dir.join("libraries");

        if libraries_dir.is_dir() {
            for library in &json.libraries {
                let library_path = libraries_dir.join(library.get_path());
                if library_path.exists() {
                    tokio::fs::remove_file(&library_path)
                        .await
                        .path(&library_path)?;
                }
            }
        }
    }

    change_instance_type(&server_dir, "Vanilla".to_owned()).await?;
    info!("Finished uninstalling fabric");

    Ok(())
}

pub async fn uninstall_client(instance_name: String) -> Result<(), FabricInstallError> {
    let instance_dir = LAUNCHER_DIR.join("instances").join(&instance_name);

    let libraries_dir = instance_dir.join("libraries");

    let fabric_json_path = instance_dir.join("fabric.json");
    if fabric_json_path.exists() {
        let fabric_json = tokio::fs::read_to_string(&fabric_json_path)
            .await
            .path(&fabric_json_path)?;
        let fabric_json: FabricJSON = serde_json::from_str(&fabric_json).json(fabric_json)?;

        tokio::fs::remove_file(&fabric_json_path)
            .await
            .path(fabric_json_path)?;

        for library in &fabric_json.libraries {
            let library_path = libraries_dir.join(library.get_path());
            if library_path.exists() {
                tokio::fs::remove_file(&library_path)
                    .await
                    .path(library_path)?;
            }
        }
    }

    change_instance_type(&instance_dir, "Vanilla".to_owned()).await?;
    Ok(())
}

pub async fn uninstall(instance: InstanceSelection) -> Result<(), FabricInstallError> {
    match instance {
        InstanceSelection::Instance(n) => uninstall_client(n).await,
        InstanceSelection::Server(n) => uninstall_server(n).await,
    }
}
