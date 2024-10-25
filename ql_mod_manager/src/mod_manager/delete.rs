use crate::mod_manager::ModIndex;
use crate::mod_manager::ModrinthError;
use ql_instances::{error::IoError, file_utils, info};
use std::{collections::HashSet, path::Path};

pub async fn delete_mods_wrapped(
    id: Vec<String>,
    instance_name: String,
) -> Result<Vec<String>, String> {
    delete_mods(&id, &instance_name)
        .await
        .map_err(|err| err.to_string())
        .map(|()| id)
}

pub async fn delete_mods(id: &[String], instance_name: &str) -> Result<(), ModrinthError> {
    let mut index = ModIndex::get(instance_name)?;

    let launcher_dir = file_utils::get_launcher_dir()?;
    let mods_dir = launcher_dir
        .join("instances")
        .join(instance_name)
        .join(".minecraft/mods");

    let mut downloaded_mods = HashSet::new();

    for id in id {
        delete_item(id, None, &mut index, &mods_dir, &mut downloaded_mods)?;
    }

    index.save()?;
    Ok(())
}

pub async fn delete_mod_wrapped(id: String, instance_name: String) -> Result<String, String> {
    delete_mod(&id, instance_name)
        .await
        .map_err(|err| err.to_string())
        .map(|()| id)
}

pub async fn delete_mod(id: &str, instance_name: String) -> Result<(), ModrinthError> {
    let mut index = ModIndex::get(&instance_name)?;
    let mut downloaded_mods = HashSet::new();

    let launcher_dir = file_utils::get_launcher_dir()?;
    let mods_dir = launcher_dir
        .join("instances")
        .join(&instance_name)
        .join(".minecraft/mods");
    delete_item(id, None, &mut index, &mods_dir, &mut downloaded_mods)?;

    index.save()?;
    Ok(())
}

fn delete_item(
    id: &str,
    parent: Option<&str>,
    index: &mut ModIndex,
    mods_dir: &Path,
    downloaded_mods: &mut HashSet<String>,
) -> Result<(), ModrinthError> {
    info!("Deleting mod {id}");
    let already_deleted = !downloaded_mods.insert(id.to_owned());
    if already_deleted {
        println!("- Already deleted, skipping");
        return Ok(());
    }

    if let Some(mod_info) = index.mods.get_mut(id) {
        if let Some(parent) = parent {
            mod_info.dependents = mod_info
                .dependents
                .iter()
                .filter_map(|n| {
                    if n.as_str() == parent {
                        None
                    } else {
                        Some(n.clone())
                    }
                })
                .collect();

            if !mod_info.dependents.is_empty() {
                return Ok(());
            }
        }
    } else {
        eprintln!("[warning] Could not find mod in index: {id}");
    }
    if let Some(mod_info) = index.mods.get(id).cloned() {
        for file in &mod_info.files {
            let path = mods_dir.join(&file.filename);
            if let Err(err) = std::fs::remove_file(&path) {
                if let std::io::ErrorKind::NotFound = err.kind() {
                    eprintln!("[warning] File does not exist, skipping: {path:?}");
                } else {
                    let err = IoError::Io {
                        error: err,
                        path: path.to_owned(),
                    };
                    Err(err)?;
                }
            }
        }

        for dependency in &mod_info.dependencies {
            delete_item(dependency, Some(id), index, mods_dir, downloaded_mods)?;
        }
    }
    index.mods.remove(id);
    Ok(())
}
