use std::path::Path;

use ql_core::{json::InstanceConfigJson, IntoIoError, JsonFileError};

pub mod fabric;
pub mod forge;
pub mod neoforge;
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
    let mut config = InstanceConfigJson::read_from_path(instance_dir).await?;

    config.mod_type = instance_type;

    let config = serde_json::to_string(&config)?;
    let config_path = instance_dir.join("config.json");
    tokio::fs::write(&config_path, config)
        .await
        .path(config_path)?;
    Ok(())
}
