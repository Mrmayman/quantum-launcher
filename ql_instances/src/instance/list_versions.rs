use std::sync::Arc;

use crate::json_structs::{json_manifest::Manifest, JsonDownloadError};

async fn list() -> Result<Vec<String>, JsonDownloadError> {
    let manifest = Manifest::download().await?;
    Ok(manifest.versions.iter().map(|n| n.id.clone()).collect())
}

pub async fn list_versions() -> Result<Arc<Vec<String>>, String> {
    list().await.map_err(|n| n.to_string()).map(Arc::new)
}
