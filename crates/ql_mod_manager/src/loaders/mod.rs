use std::path::Path;

use ql_core::{
    json::InstanceConfigJson, InstanceSelection, IntoIoError, IntoJsonError, IntoStringError,
    JsonFileError, Loader,
};

pub mod fabric;
pub mod forge;
pub mod neoforge;
pub mod optifine;
pub mod paper;

async fn change_instance_type(
    instance_dir: &Path,
    instance_type: String,
) -> Result<(), JsonFileError> {
    let mut config = InstanceConfigJson::read_from_path(instance_dir).await?;

    config.mod_type = instance_type;

    let config = serde_json::to_string(&config).json_to()?;
    let config_path = instance_dir.join("config.json");
    tokio::fs::write(&config_path, config)
        .await
        .path(config_path)?;
    Ok(())
}

#[derive(Debug, Clone, Copy)]
pub enum LoaderInstallResult {
    Ok,
    NeedsOptifine,
    Unsupported,
}

pub async fn install_specified_loader(
    instance: InstanceSelection,
    loader: Loader,
) -> Result<LoaderInstallResult, String> {
    match loader {
        // TODO: Progress Bar
        Loader::Fabric => {
            fabric::install(None, instance, None, false)
                .await
                .strerr()?;
        }
        Loader::Quilt => {
            fabric::install(None, instance, None, true).await.strerr()?;
        }

        Loader::Forge => forge::install(instance, None, None).await.strerr()?,
        Loader::Neoforge => neoforge::install(instance, None, None).await.strerr()?,

        Loader::Paper => {
            debug_assert!(instance.is_server());
            paper::install(instance.get_name().to_owned())
                .await
                .strerr()?;
        }

        Loader::OptiFine => return Ok(LoaderInstallResult::NeedsOptifine),

        Loader::Liteloader | Loader::Modloader | Loader::Rift => {
            return Ok(LoaderInstallResult::Unsupported)
        }
    }
    Ok(LoaderInstallResult::Ok)
}
