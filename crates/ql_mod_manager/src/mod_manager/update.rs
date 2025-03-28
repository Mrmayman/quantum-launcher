use std::sync::mpsc::Sender;

use chrono::DateTime;
use ql_core::{
    info, info_no_log, json::VersionDetails, GenericProgress, InstanceSelection, Loader,
};

use crate::mod_manager::{get_latest_version_date, get_loader};

use super::{delete_mods, download_mod, ModError, ModId, ModIndex};

pub async fn apply_updates(
    selected_instance: InstanceSelection,
    updates: Vec<ModId>,
    progress: Option<Sender<GenericProgress>>,
) -> Result<(), ModError> {
    delete_mods(&updates, &selected_instance).await?;
    let updates_len = updates.len();
    for (i, id) in updates.into_iter().enumerate() {
        if let Some(progress) = &progress {
            progress
                .send(GenericProgress {
                    done: i,
                    total: updates_len,
                    message: None,
                    has_finished: false,
                })
                .ok();
        }
        download_mod(&id, &selected_instance).await?;
    }
    if let Some(progress) = &progress {
        progress.send(GenericProgress::finished()).ok();
    }
    Ok(())
}

pub async fn check_for_updates(
    selected_instance: InstanceSelection,
) -> Result<Vec<(ModId, String)>, ModError> {
    info_no_log!("Checking for mod updates");
    let index = ModIndex::get(&selected_instance).await?;

    let version_json = VersionDetails::load(&selected_instance).await?;

    let loader = get_loader(&selected_instance).await?;
    if let Some(Loader::OptiFine) = loader {
        return Ok(Vec::new());
    }

    let mut updated_mods = Vec::new();

    for (id, installed_mod) in index.mods {
        let mod_id = ModId::from_index_str(&id);

        let version = &version_json.id;
        let (download_version_time, download_version) =
            get_latest_version_date(loader, &mod_id, version).await?;

        let installed_version_time =
            DateTime::parse_from_rfc3339(&installed_mod.version_release_time)?;

        if download_version_time > installed_version_time {
            updated_mods.push((mod_id, download_version));
        }
    }

    if updated_mods.is_empty() {
        info!("No mod updates found");
    } else {
        info!("Found mod updates");
    }

    Ok(updated_mods)
}
