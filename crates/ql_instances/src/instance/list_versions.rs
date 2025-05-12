use std::sync::{mpsc::Sender, Arc};

use omniarchive_api::{ListEntry, ListError};
use ql_core::{err, json::Manifest};

/// Returns a list of every downloadable version of Minecraft.
/// Sources the list from Mojang and Omniarchive (combined).
///
/// # Errors
/// - If the version [manifest](https://launchermeta.mojang.com/mc/game/version_manifest.json)
///   couldn't be downloaded
/// - If the version manifest couldn't be parsed into JSON
///
/// Note: If Omniarchive list download for old versions fails,
/// an error will be logged but not returned (for smoother user experience),
/// and instead the official (inferior) old version list will be downloaded
/// from Mojang.
pub async fn list_versions(sender: Option<Arc<Sender<()>>>) -> Result<Vec<ListEntry>, ListError> {
    let manifest = Manifest::download().await?;
    let mut version_list: Vec<ListEntry> = manifest
        .versions
        .iter()
        .filter_map(|n| {
            (n.r#type == "release" || n.r#type == "snapshot")
                .then_some(ListEntry::Normal(n.id.clone()))
        })
        .collect();

    if let Err(err) = add_omniarchive_versions(&mut version_list, sender).await {
        err!("error getting omniarchive version list: {err}");
        // Since Omniarchive old versions couldn't be loaded,
        // let's just load the normal (inferior) old versions
        // from Mojang.
        version_list.extend(manifest.versions.iter().filter_map(|n| {
            (!(n.r#type == "release" || n.r#type == "snapshot"))
                .then_some(ListEntry::Normal(n.id.clone()))
        }));
    }

    Ok(version_list)
}

async fn add_omniarchive_versions(
    normal_list: &mut Vec<ListEntry>,
    progress: Option<Arc<Sender<()>>>,
) -> Result<(), ListError> {
    let versions = omniarchive_api::download_all(progress.clone(), false).await?;

    for entry in versions {
        let name = if let Some(name) = entry
            .url
            .strip_prefix("https://vault.omniarchive.uk/archive/java/client-")
            .and_then(|n| n.strip_suffix(".jar"))
        {
            name.to_owned()
        } else {
            entry.url.clone()
        };
        let nice_name = name
            .split('/')
            .next_back()
            .map(str::to_owned)
            .unwrap_or(name.clone());
        normal_list.push(ListEntry::Omniarchive {
            category: entry.category,
            name,
            nice_name,
            url: entry.url,
        });
    }

    Ok(())
}
