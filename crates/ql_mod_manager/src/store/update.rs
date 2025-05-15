use std::sync::mpsc::Sender;

use chrono::DateTime;
use ql_core::{
    do_jobs, info_no_log, json::VersionDetails, GenericProgress, InstanceSelection, Loader,
};

use crate::store::{get_latest_version_date, get_loader};

use super::{delete_mods, download_mods_bulk, ModError, ModId, ModIndex};

pub async fn apply_updates(
    selected_instance: InstanceSelection,
    updates: Vec<ModId>,
    progress: Option<Sender<GenericProgress>>,
) -> Result<(), ModError> {
    // It's as simple as that!
    delete_mods(updates.clone(), selected_instance.clone()).await?;
    download_mods_bulk(updates, selected_instance, progress).await?;
    Ok(())
}

pub async fn check_for_updates(
    selected_instance: InstanceSelection,
) -> Result<Vec<(ModId, String)>, ModError> {
    let index = ModIndex::get(&selected_instance).await?;

    let version_json = VersionDetails::load(&selected_instance).await?;

    let loader = get_loader(&selected_instance).await?;
    if let Some(Loader::OptiFine) = loader {
        return Ok(Vec::new());
    }
    info_no_log!(
        "Checking for mod updates (loader: {})",
        loader.map_or("Vanilla".to_owned(), |n| format!("{n:?}"))
    );

    let version = &version_json.id;

    let updated_mods: Result<Vec<Option<(ModId, String)>>, ModError> = do_jobs(
        index
            .mods
            .into_iter()
            .map(|(id, installed_mod)| async move {
                let mod_id = ModId::from_index_str(&id);

                let (download_version_time, download_version) =
                    get_latest_version_date(loader, &mod_id, version).await?;

                let installed_version_time =
                    DateTime::parse_from_rfc3339(&installed_mod.version_release_time)?;

                Ok((download_version_time > installed_version_time)
                    .then_some((mod_id, download_version)))
            }),
    )
    .await;
    let updated_mods: Vec<(ModId, String)> = updated_mods?.into_iter().flatten().collect();

    if updated_mods.is_empty() {
        info_no_log!("No mod updates found");
    } else {
        info_no_log!("Found mod updates");
    }

    Ok(updated_mods)
}
