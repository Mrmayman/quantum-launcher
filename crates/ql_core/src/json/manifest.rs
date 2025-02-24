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
        const VERSIONS_JSON: &str = "https://launchermeta.mojang.com/mc/game/version_manifest.json";

        let manifest = file_utils::download_file_to_string(VERSIONS_JSON, false).await?;
        Ok(serde_json::from_str(&manifest)?)
    }

    /// Looks up a version by its name.
    pub fn find_name(&self, name: &str) -> Option<&Version> {
        self.versions.iter().find(|n| n.id == name)
    }

    /// Looks up a version by its name, but allows for a fuzzy search.
    ///
    /// # Arguments
    /// - `name`: The name of the version to search for
    ///   (fuzzy, can be approximate).
    /// - `filter`: A filter such that the version must start with this string.
    ///
    /// # Returns
    /// - `Some(_)`: The version that is closest to the name
    /// - `None`: No version was found with the filter, or the manifest is empty.
    pub fn find_fuzzy(&self, name: &str, filter: &str) -> Option<&Version> {
        self.versions
            .iter()
            .filter(|n| n.id.starts_with(filter))
            .min_by_key(|choice| strsim::levenshtein(name, &choice.id))
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
