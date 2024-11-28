use std::path::Path;

use ql_instances::{err, error::IoError, file_utils, io_err};

use crate::mod_manager::ModIndex;

use super::ModrinthError;

pub async fn toggle_mods_wrapped(id: Vec<String>, instance_name: String) -> Result<(), String> {
    toggle_mods(&id, &instance_name)
        .await
        .map_err(|err| err.to_string())
}

async fn toggle_mods(id: &[String], instance_name: &str) -> Result<(), ModrinthError> {
    let mut index = ModIndex::get(instance_name)?;

    let launcher_dir = file_utils::get_launcher_dir()?;
    let mods_dir = launcher_dir
        .join("instances")
        .join(instance_name)
        .join(".minecraft/mods");

    for id in id {
        if let Some(info) = index.mods.get_mut(id) {
            for file in &info.files {
                let enabled_path = mods_dir.join(&file.filename);
                let disabled_path = mods_dir.join(&format!("{}.disabled", file.filename));

                if info.enabled {
                    rename_file(&enabled_path, &disabled_path)?;
                } else {
                    rename_file(&disabled_path, &enabled_path)?;
                }
            }
            info.enabled = !info.enabled;
        }
    }

    index.save()?;
    Ok(())
}

fn rename_file(a: &Path, b: &Path) -> Result<(), ModrinthError> {
    if let Err(err) = std::fs::rename(&a, &b) {
        if let std::io::ErrorKind::NotFound = err.kind() {
            err!("Cannot find file for renaming, skipping: {a:?} -> {b:?}");
        } else {
            let err = IoError::Io {
                error: err,
                path: a.to_owned(),
            };
            Err(err)?;
        }
    }
    Ok(())
}
