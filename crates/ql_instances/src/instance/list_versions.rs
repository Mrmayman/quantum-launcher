use ql_core::{json::Manifest, JsonDownloadError, ListEntry};

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
pub async fn list_versions() -> Result<Vec<ListEntry>, JsonDownloadError> {
    Ok(Manifest::download()
        .await?
        .versions
        .into_iter()
        .map(|n| ListEntry {
            name: n.id,
            is_classic_server: false,
        })
        .collect())
}
