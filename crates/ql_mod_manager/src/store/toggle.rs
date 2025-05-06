use std::path::Path;

use ql_core::{err, file_utils, InstanceSelection, IoError};

use crate::store::ModIndex;

use super::ModError;

#[must_use]
pub fn flip_filename(name: &str) -> String {
    if let Some(n) = name.strip_suffix(".disabled") {
        n.to_owned()
    } else {
        format!("{name}.disabled")
    }
}

pub async fn toggle_mods_local(
    names: Vec<String>,
    instance_name: InstanceSelection,
) -> Result<(), ModError> {
    let mods_dir = file_utils::get_dot_minecraft_dir(&instance_name)?.join("mods");

    for file in names {
        let flipped = flip_filename(&file);
        rename_file(&mods_dir.join(&file), &mods_dir.join(flipped)).await?;
    }
    Ok(())
}

pub async fn toggle_mods(
    id: Vec<String>,
    instance_name: InstanceSelection,
) -> Result<(), ModError> {
    let mut index = ModIndex::get(&instance_name).await?;

    let mods_dir = file_utils::get_dot_minecraft_dir(&instance_name)?.join("mods");

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
                error: err.to_string(),
                path: a.to_owned(),
            };
            Err(err)?;
        }
    }
    Ok(())
}
