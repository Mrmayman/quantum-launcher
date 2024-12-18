use std::sync::mpsc::Sender;

use chrono::DateTime;
use ql_core::info;

use crate::mod_manager::{
    download::{get_instance_and_mod_dir, get_loader_type, get_version_json, version_sort},
    ModVersion,
};

use super::{delete_mods, download_mod, ModError, ModIndex};

pub enum ApplyUpdateProgress {
    P1DeleteMods,
    P2DownloadMod { done: usize, out_of: usize },
    P3Done,
}

pub async fn apply_updates_wrapped(
    selected_instance: String,
    updates: Vec<String>,
    progress: Option<Sender<ApplyUpdateProgress>>,
) -> Result<(), String> {
    apply_updates(selected_instance, updates, progress)
        .await
        .map_err(|err| err.to_string())
}

async fn apply_updates(
    selected_instance: String,
    updates: Vec<String>,
    progress: Option<Sender<ApplyUpdateProgress>>,
) -> Result<(), ModError> {
    if let Some(progress) = &progress {
        progress.send(ApplyUpdateProgress::P1DeleteMods).ok();
    }
    delete_mods(&updates, &selected_instance).await?;
    let updates_len = updates.len();
    for (i, id) in updates.into_iter().enumerate() {
        if let Some(progress) = &progress {
            progress
                .send(ApplyUpdateProgress::P2DownloadMod {
                    done: i,
                    out_of: updates_len,
                })
                .ok();
        }
        download_mod(id, selected_instance.clone()).await?;
    }
    if let Some(progress) = &progress {
        progress.send(ApplyUpdateProgress::P3Done).ok();
    }
    Ok(())
}

pub async fn check_for_updates(selected_instance: String) -> Option<Vec<(String, String)>> {
    let index = ModIndex::get(&selected_instance).ok()?;

    let (instance_dir, _) = get_instance_and_mod_dir(&selected_instance).ok()?;
    let version_json = get_version_json(&instance_dir).ok()?;

    let loader = get_loader_type(&instance_dir).ok()?;

    let mut updated_mods = Vec::new();

    for (id, installed_mod) in index.mods {
        let download_info = ModVersion::download(&id).await.ok()?;

        let mut download_versions: Vec<ModVersion> = download_info
            .iter()
            .filter(|v| v.game_versions.contains(&version_json.id))
            .filter(|v| {
                if let Some(loader) = &loader {
                    v.loaders.contains(loader)
                } else {
                    true
                }
            })
            .cloned()
            .collect();

        // Sort by date published
        download_versions.sort_by(version_sort);

        let Some(download_version) = download_versions.into_iter().last() else {
            continue;
        };

        let installed_version_time =
            DateTime::parse_from_rfc3339(&installed_mod.version_release_time).ok()?;
        let download_version_time =
            DateTime::parse_from_rfc3339(&download_version.date_published).ok()?;

        if download_version_time > installed_version_time {
            updated_mods.push((id, download_version.name));
        }
    }

    if updated_mods.is_empty() {
        info!("No mod updates found");
    } else {
        info!("Found mod updates");
    }

    Some(updated_mods)
}
