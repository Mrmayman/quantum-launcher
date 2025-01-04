use ql_core::{json::manifest::Manifest, JsonDownloadError, ListEntry};

async fn list() -> Result<Vec<ListEntry>, JsonDownloadError> {
    let manifest = Manifest::download().await?;

    let version_list: Vec<ListEntry> = manifest
        .versions
        .iter()
        .map(|n| ListEntry(n.id.clone()))
        .collect();

    Ok(version_list)
}

/// Returns a list of all available versions of the game.
pub async fn list_versions() -> Result<Vec<ListEntry>, String> {
    list().await.map_err(|n| n.to_string())
}
