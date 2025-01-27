use std::path::Path;

use ql_core::{json::InstanceConfigJson, IntoIoError, JsonFileError};

pub mod fabric;
pub mod forge;
pub mod optifine;
pub mod paper;

pub enum CoreMod {
    None,
    Fabric,
    Forge,
    Quilt,
    Optifine,
}

async fn change_instance_type(
    instance_dir: &Path,
    instance_type: String,
) -> Result<(), JsonFileError> {
    let config_path = instance_dir.join("config.json");
    let config = tokio::fs::read_to_string(&config_path)
        .await
        .path(&config_path)?;
    let mut config: InstanceConfigJson = serde_json::from_str(&config)?;

    config.mod_type = instance_type;

    let config = serde_json::to_string(&config)?;
    tokio::fs::write(&config_path, config)
        .await
        .path(config_path)?;
    Ok(())
}
