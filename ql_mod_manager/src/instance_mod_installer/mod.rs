use std::{fmt::Display, path::Path};

use ql_instances::{
    error::IoError, io_err, json_structs::json_instance_config::InstanceConfigJson,
};

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

fn change_instance_type(
    instance_dir: &Path,
    instance_type: String,
) -> Result<(), ChangeConfigError> {
    let config_path = instance_dir.join("config.json");
    let config = std::fs::read_to_string(&config_path).map_err(io_err!(config_path))?;
    let mut config: InstanceConfigJson = serde_json::from_str(&config)?;

    config.mod_type = instance_type;

    let config = serde_json::to_string(&config)?;
    std::fs::write(&config_path, config).map_err(io_err!(config_path))?;
    Ok(())
}

#[derive(Debug)]
pub enum ChangeConfigError {
    Serde(serde_json::Error),
    Io(IoError),
}

impl Display for ChangeConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChangeConfigError::Serde(err) => write!(f, "{err}"),
            ChangeConfigError::Io(err) => write!(f, "{err}"),
        }
    }
}

impl From<serde_json::Error> for ChangeConfigError {
    fn from(value: serde_json::Error) -> Self {
        Self::Serde(value)
    }
}

impl From<IoError> for ChangeConfigError {
    fn from(value: IoError) -> Self {
        Self::Io(value)
    }
}
