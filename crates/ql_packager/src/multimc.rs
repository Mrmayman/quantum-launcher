use std::{
    path::Path,
    sync::{mpsc::Sender, Arc},
};

use crate::{import::pipe_progress, import::OUT_OF, InstancePackageError};
use ql_core::{
    err, file_utils, info, json::InstanceConfigJson, GenericProgress, InstanceSelection,
    IntoIoError, IntoJsonError, ListEntry,
};
use serde::{Deserialize, Serialize};
use tokio::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MmcPack {
    pub components: Vec<MmcPackComponent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct MmcPackComponent {
    pub cachedName: String,
    pub cachedVersion: String,
}

pub async fn import(
    download_assets: bool,
    temp_dir: &Path,
    mmc_pack: String,
    sender: Option<Arc<Sender<GenericProgress>>>,
) -> Result<InstanceSelection, InstancePackageError> {
    info!("Importing MultiMC instance...");
    let mmc_pack: MmcPack = serde_json::from_str(&mmc_pack).json(mmc_pack)?;

    let ini_path = temp_dir.join("instance.cfg");
    let ini = fs::read_to_string(&ini_path).await.path(ini_path)?;
    let ini = ini::Ini::load_from_str(&filter_bytearray(&ini))?;

    let instance_name = ini
        .get_from(Some("General"), "name")
        .or(ini.get_from(None::<String>, "name"))
        .ok_or_else(|| {
            InstancePackageError::IniFieldMissing("General".to_owned(), "name".to_owned())
        })?
        .to_owned();
    let instance_selection = InstanceSelection::new(&instance_name, false);

    for component in &mmc_pack.components {
        match component.cachedName.as_str() {
            "Minecraft" => {
                mmc_minecraft(download_assets, sender.clone(), &instance_name, component).await?;
            }

            name @ ("Forge" | "NeoForge") => {
                mmc_forge(
                    sender.clone(),
                    &instance_selection,
                    component,
                    name == "NeoForge",
                )
                .await?;
            }
            name @ ("Fabric Loader" | "Quilt Loader") => {
                ql_mod_manager::loaders::fabric::install(
                    Some(component.cachedVersion.clone()),
                    instance_selection.clone(),
                    sender.as_deref(),
                    name == "Quilt",
                )
                .await?;
            }

            "Intermediary Mappings" | "LWJGL 2" | "LWJGL 3" => {}
            name => err!("Unknown component (in MultiMC instance): {name}"),
        }
    }

    copy_files(temp_dir, sender, &instance_selection).await?;

    let mut config = InstanceConfigJson::read(&instance_selection).await?;
    if let Some(jvmargs) = ini.get_from(Some("General"), "JvmArgs") {
        let mut java_args = config.java_args.clone().unwrap_or_default();
        java_args.extend(jvmargs.split_whitespace().map(str::to_owned));
        config.java_args = Some(java_args);
    }
    config.save(&instance_selection).await?;
    info!("Finished importing MultiMC instance");
    Ok(instance_selection)
}

async fn copy_files(
    temp_dir: &Path,
    sender: Option<Arc<Sender<GenericProgress>>>,
    instance_selection: &InstanceSelection,
) -> Result<(), InstancePackageError> {
    let src = temp_dir.join("minecraft");
    if src.is_dir() {
        let dst = instance_selection.get_dot_minecraft_path();
        if let Some(sender) = sender.as_deref() {
            _ = sender.send(GenericProgress {
                done: 2,
                total: OUT_OF,
                message: Some("Copying files...".to_owned()),
                has_finished: false,
            });
        }
        file_utils::copy_dir_recursive(&src, &dst).await?;
    }

    copy_folder_over(temp_dir, instance_selection, "jarmods").await?;
    copy_folder_over(temp_dir, instance_selection, "patches").await?;

    Ok(())
}

async fn copy_folder_over(
    temp_dir: &Path,
    instance_selection: &InstanceSelection,
    path: &'static str,
) -> Result<(), InstancePackageError> {
    let src = temp_dir.join(path);
    if src.is_dir() {
        let dst = instance_selection.get_instance_path().join(path);
        file_utils::copy_dir_recursive(&src, &dst).await?;
    }
    Ok(())
}

async fn mmc_minecraft(
    download_assets: bool,
    sender: Option<Arc<Sender<GenericProgress>>>,
    instance_name: &str,
    component: &MmcPackComponent,
) -> Result<(), InstancePackageError> {
    let version = ListEntry {
        name: component.cachedVersion.clone(),
        is_classic_server: false,
    };
    let (d_send, d_recv) = std::sync::mpsc::channel();
    if let Some(sender) = sender.clone() {
        std::thread::spawn(move || {
            pipe_progress(d_recv, &sender);
        });
    }
    ql_instances::create_instance(
        instance_name.to_owned(),
        version,
        Some(d_send),
        download_assets,
    )
    .await?;
    Ok(())
}

async fn mmc_forge(
    sender: Option<Arc<Sender<GenericProgress>>>,
    instance_selection: &InstanceSelection,
    component: &MmcPackComponent,
    is_neoforge: bool,
) -> Result<(), InstancePackageError> {
    let (f_send, f_recv) = std::sync::mpsc::channel();
    if let Some(sender) = sender.clone() {
        std::thread::spawn(move || {
            pipe_progress(f_recv, &sender);
        });
    }
    if is_neoforge {
        ql_mod_manager::loaders::neoforge::install(
            instance_selection.clone(),
            Some(f_send),
            None, // TODO: Java install progress
        )
        .await?;
    } else {
        ql_mod_manager::loaders::forge::install(
            Some(component.cachedVersion.clone()),
            instance_selection.clone(),
            Some(f_send),
            None, // TODO: Java install progress
        )
        .await?;
    }
    Ok(())
}

fn filter_bytearray(input: &str) -> String {
    // PrismLauncher puts some weird ByteArray
    // field in the INI config file, that our pookie little ini parser
    // doesn't understand. So we gotta filter it out.
    input
        .lines()
        .filter(|n| !n.starts_with("mods_Page\\Columns"))
        .collect::<Vec<_>>()
        .join("\n")
}
