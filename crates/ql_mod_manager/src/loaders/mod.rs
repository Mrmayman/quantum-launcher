use std::{
    path::Path,
    sync::{
        mpsc::{Receiver, Sender},
        Arc,
    },
};

use forge::ForgeInstallProgress;
use ql_core::{
    json::InstanceConfigJson, GenericProgress, InstanceSelection, IntoIoError, IntoJsonError,
    IntoStringError, JsonFileError, Loader, Progress,
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
    let mut config = InstanceConfigJson::read_from_dir(instance_dir).await?;

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
    progress: Option<Arc<Sender<GenericProgress>>>,
    specified_version: Option<String>,
) -> Result<LoaderInstallResult, String> {
    match loader {
        Loader::Fabric => {
            fabric::install(specified_version, instance, progress.as_deref(), false)
                .await
                .strerr()?;
        }
        Loader::Quilt => {
            fabric::install(specified_version, instance, progress.as_deref(), true)
                .await
                .strerr()?;
        }

        Loader::Forge => {
            let (send, recv) = std::sync::mpsc::channel();
            if let Some(progress) = progress {
                std::thread::spawn(move || {
                    pipe_progress(recv, &progress);
                });
            }

            // TODO: Java install progress
            forge::install(specified_version, instance, Some(send), None)
                .await
                .strerr()?;
        }
        Loader::Neoforge => {
            let (send, recv) = std::sync::mpsc::channel();
            if let Some(progress) = progress {
                std::thread::spawn(move || {
                    pipe_progress(recv, &progress);
                });
            }

            // TODO: Specified version
            neoforge::install(instance, Some(send), None)
                .await
                .strerr()?;
        }

        Loader::Paper => {
            debug_assert!(instance.is_server());
            // TODO: Specified version
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

fn pipe_progress(rec: Receiver<ForgeInstallProgress>, snd: &Sender<GenericProgress>) {
    for item in rec {
        _ = snd.send(item.into_generic());
    }
}
