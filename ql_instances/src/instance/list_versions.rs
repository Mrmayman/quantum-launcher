use ql_core::JsonDownloadError;

use crate::json_structs::json_manifest::Manifest;

async fn list() -> Result<Vec<String>, JsonDownloadError> {
    let manifest = Manifest::download().await?;
    Ok(manifest.versions.iter().map(|n| n.id.clone()).collect())
}

/// Returns a list of all available versions of the game.
pub async fn list_versions() -> Result<Vec<String>, String> {
    list().await.map_err(|n| n.to_string())
}
