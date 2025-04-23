use std::sync::mpsc::Sender;

use chrono::DateTime;
use ql_core::{info_no_log, json::VersionDetails, GenericProgress, InstanceSelection, Loader};

use crate::store::{get_latest_version_date, get_loader};

use super::{delete_mods, download_mods_bulk, ModError, ModId, ModIndex};

pub async fn apply_updates(
    selected_instance: InstanceSelection,
    updates: Vec<ModId>,
    progress: Option<Sender<GenericProgress>>,
) -> Result<(), ModError> {
    // It's as simple as that!
    delete_mods(&updates, &selected_instance).await?;
    download_mods_bulk(updates, selected_instance, progress).await?;
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
        info_no_log!("No mod updates found");
    } else {
        info_no_log!("Found mod updates");
    }

    Ok(updated_mods)
}
