use std::{collections::HashSet, ffi::OsStr, path::PathBuf, sync::mpsc::Sender};

use ql_core::{err, pt, GenericProgress, InstanceSelection, IntoIoError};

use crate::presets;

use super::{
    modpack::{self, PackError},
    CurseforgeNotAllowed,
};

pub async fn add_files(
    instance: InstanceSelection,
    paths: Vec<PathBuf>,
    progress: Option<Sender<GenericProgress>>,
) -> Result<HashSet<CurseforgeNotAllowed>, PackError> {
    let mods_dir = instance.get_dot_minecraft_path().join("mods");

    let mut not_allowed = HashSet::new();

    send_progress(progress.as_ref(), GenericProgress::default());

    let len = paths.len();
    for (i, path) in paths.into_iter().enumerate() {
        let (Some(extension), Some(filename)) =
            (path.extension().and_then(OsStr::to_str), path.file_name())
        else {
            continue;
        };

        let extension = extension.to_lowercase();

        let file_type = match extension.as_str() {
            "jar" => "mod",
            "zip" | "mrpack" => "modpack",
            "qmp" => "QuantumLauncher mod preset",
            _ => "Unknown file (ERROR)",
        };
        send_progress(
            progress.as_ref(),
            GenericProgress {
                done: i,
                total: len,
                message: Some(format!("Installing {file_type}: ({}/{len})", i + 1)),
                has_finished: false,
            },
        );

        match extension.as_str() {
            "jar" => {
                tokio::fs::copy(&path, mods_dir.join(filename))
                    .await
                    .path(&path)?;
            }
            "zip" | "mrpack" => {
                let file = tokio::fs::read(&path).await.path(&path)?;
                let not_allowed_new =
                    modpack::install_modpack(file, instance.clone(), progress.as_ref()).await?;
                not_allowed.extend(not_allowed_new);
            }
            "qmp" => {
                let file = tokio::fs::read(&path).await.path(&path)?;
                presets::PresetJson::load(instance.clone(), file).await?;
            }
            extension => {
                err!("While adding mod files: Unrecognized extension: .{extension}");
            }
        }
    }

    send_progress(progress.as_ref(), GenericProgress::finished());

    Ok(not_allowed)
}

fn send_progress(sender: Option<&Sender<GenericProgress>>, progress: GenericProgress) {
    if let Some(sender) = sender {
        if sender.send(progress.clone()).is_ok() {
            return;
        }
    }
    if let Some(msg) = &progress.message {
        pt!("{msg}");
    }
}
