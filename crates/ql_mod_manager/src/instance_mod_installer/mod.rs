use std::path::Path;

use ql_core::{io_err, json::instance_config::InstanceConfigJson, JsonFileError};

pub mod fabric;
pub mod forge;
pub mod optifine;

pub enum CoreMod {
    None,
    Fabric,
    Forge,
    Quilt,
    Optifine,
}

fn change_instance_type(instance_dir: &Path, instance_type: String) -> Result<(), JsonFileError> {
    let config_path = instance_dir.join("config.json");
    let config = std::fs::read_to_string(&config_path).map_err(io_err!(config_path))?;
    let mut config: InstanceConfigJson = serde_json::from_str(&config)?;

    config.mod_type = instance_type;

    let config = serde_json::to_string(&config)?;
    std::fs::write(&config_path, config).map_err(io_err!(config_path))?;
    Ok(())
}