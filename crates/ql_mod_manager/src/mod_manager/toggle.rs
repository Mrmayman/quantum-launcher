use std::path::Path;

use ql_core::{err, file_utils, InstanceSelection, IoError};

use crate::mod_manager::ModIndex;

use super::ModError;

pub async fn toggle_mods(
    id: Vec<String>,
    instance_name: InstanceSelection,
) -> Result<(), ModError> {
    let mut index = ModIndex::get(&instance_name).await?;

    let mods_dir = file_utils::get_dot_minecraft_dir(&instance_name)
        .await?
        .join("mods");

    for id in id {
        if let Some(info) = index.mods.get_mut(&id) {
            for file in &info.files {
                let enabled_path = mods_dir.join(&file.filename);
                let disabled_path = mods_dir.join(format!("{}.disabled", file.filename));

                if info.enabled {
                    rename_file(&enabled_path, &disabled_path).await?;
                } else {
                    rename_file(&disabled_path, &enabled_path).await?;
                }
            }
            info.enabled = !info.enabled;
        }
    }

    index.save(&instance_name).await?;
    Ok(())
}

async fn rename_file(a: &Path, b: &Path) -> Result<(), ModError> {
    if let Err(err) = tokio::fs::rename(a, b).await {
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
