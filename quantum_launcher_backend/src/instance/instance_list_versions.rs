use std::sync::Arc;

use crate::{error::LauncherResult, file_utils, json_structs::json_manifest::Manifest};

async fn list() -> LauncherResult<Vec<String>> {
    let network_client = reqwest::Client::new();
    let manifest = Manifest::download().await?;
    Ok(manifest.versions.iter().map(|n| n.id.clone()).collect())
}

pub async fn list_versions() -> Result<Arc<Vec<String>>, String> {
    list().await.map_err(|n| n.to_string()).map(Arc::new)
}
