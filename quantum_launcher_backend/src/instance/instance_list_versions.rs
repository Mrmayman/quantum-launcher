use std::sync::Arc;

use crate::{
    download::VERSIONS_JSON, error::LauncherResult, file_utils,
    json_structs::json_manifest::Manifest,
};

fn list() -> LauncherResult<Vec<String>> {
    let network_client = reqwest::blocking::Client::new();
    let manifest_json = file_utils::download_file_to_string(&network_client, VERSIONS_JSON)?;
    let manifest: Manifest = serde_json::from_str(&manifest_json)?;
    Ok(manifest.versions.iter().map(|n| n.id.clone()).collect())
}

pub async fn list_versions() -> Result<Arc<Vec<String>>, String> {
    list().map_err(|n| n.to_string()).map(Arc::new)
}
