use crate::{file_utils, JsonDownloadError};
use serde::Deserialize;

/// An official Minecraft version manifest
/// (list of all versions and their download links)
/// from Mojang's servers.
#[derive(Deserialize)]
pub struct Manifest {
    pub latest: Latest,
    pub versions: Vec<Version>,
}

impl Manifest {
    /// Downloads the manifest from the Mojang servers.
    ///
    /// # Errors
    /// If the file cannot be downloaded
    /// (server error or bad internet) or parsed into JSON.
    pub async fn download() -> Result<Manifest, JsonDownloadError> {
        const VERSIONS_JSON: &str = "https://mcphackers.org/BetterJSONs/version_manifest_v2.json";
        file_utils::download_file_to_json(VERSIONS_JSON, false).await
    }

    /// Looks up a version by its name.
    #[must_use]
    pub fn find_name(&self, name: &str) -> Option<&Version> {
        self.versions.iter().find(|n| n.id == name)
    }
}

#[derive(Deserialize)]
pub struct Latest {
    pub release: String,
    pub snapshot: String,
}

#[allow(non_snake_case)]
#[derive(Deserialize)]
pub struct Version {
    pub id: String,
    pub r#type: String,
    pub url: String,
    pub time: String,
    pub releaseTime: String,
}
