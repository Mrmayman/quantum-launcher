use crate::{file_utils, IntoJsonError, JsonDownloadError};
use serde::Deserialize;

/// An official Minecraft version manifest
/// (list of all versions and their download links)
#[derive(Deserialize, Clone, Debug)]
pub struct Manifest {
    latest: Latest,
    pub versions: Vec<Version>,
}

impl Manifest {
    /// Downloads a complete manifest by combining:
    /// - A *curated, but outdated* manifest
    ///   ([BetterJSONs](https://mcphackers.org/BetterJSONs/version_manifest_v2.json)).
    /// - An *up-to-date but unpolished* manifest:
    ///   Platform-dependent URLs (see below)
    ///
    /// This ensures a consistent, high-quality manifest by preserving curated data
    /// for older versions (up to `25w14craftmine`) and appending newer versions
    /// from the official or forked manifests.
    ///
    /// # Platform-specific URLs
    /// - ARM64 linux: <https://raw.githubusercontent.com/theofficialgman/piston-meta-arm64/refs/heads/main/mc/game/version_manifest_v2.json>
    /// - ARM32 linux: <https://raw.githubusercontent.com/theofficialgman/piston-meta-arm32/refs/heads/main/mc/game/version_manifest_v2.json>
    /// - Other platforms: <https://launchermeta.mojang.com/mc/game/version_manifest_v2.json>
    ///
    /// # Errors
    /// Returns an error if either file cannot be downloaded or parsed into JSON.
    pub async fn download() -> Result<Manifest, JsonDownloadError> {
        // An out-of-date but curated manifest
        const OLDER_VERSIONS_JSON: &str =
            "https://mcphackers.org/BetterJSONs/version_manifest_v2.json";

        // An up-to-date manifest that lacks some fixes/polish
        #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
        const NEWER_VERSIONS_JSON: &str = "https://raw.githubusercontent.com/theofficialgman/piston-meta-arm64/refs/heads/main/mc/game/version_manifest_v2.json";
        #[cfg(all(target_os = "linux", target_arch = "arm"))]
        const NEWER_VERSIONS_JSON: &str = "https://raw.githubusercontent.com/theofficialgman/piston-meta-arm32/refs/heads/main/mc/game/version_manifest_v2.json";
        #[cfg(not(all(target_os = "linux", any(target_arch = "aarch64", target_arch = "arm"))))]
        const NEWER_VERSIONS_JSON: &str =
            "https://launchermeta.mojang.com/mc/game/version_manifest_v2.json";

        let (older_manifest, newer_manifest) = tokio::try_join!(
            file_utils::download_file_to_string(OLDER_VERSIONS_JSON, false),
            file_utils::download_file_to_string(NEWER_VERSIONS_JSON, false)
        )?;
        let mut older_manifest: Self =
            serde_json::from_str(&older_manifest).json(older_manifest)?;
        let newer_manifest: Self = serde_json::from_str(&newer_manifest).json(newer_manifest)?;

        // Removes newer versions from out-of-date manifest
        // if it ever gets updated, to not mess up the list.
        older_manifest.versions =
            exclude_versions_after(&older_manifest.versions, |n| n.id == "25w14craftmine");
        // Add newer versions (that lack fixes/polish) to the manifest
        older_manifest.versions.splice(
            0..0,
            include_versions_after(&newer_manifest.versions, |n| n.id == "25w14craftmine"),
        );

        Ok(older_manifest)
    }

    /// Looks up a version by its name.
    /// This searches for an *exact match*.
    #[must_use]
    pub fn find_name(&self, name: &str) -> Option<&Version> {
        self.versions.iter().find(|n| n.id == name)
    }

    /// Gets the latest stable release
    ///
    /// This only returns a `None` if the .latest field's
    /// data is *wrong* (impossible normally, if you just
    /// [`Manifest::download`] it). So it's mostly safe
    /// to unwrap.
    #[must_use]
    pub fn get_latest_release(&self) -> Option<&Version> {
        self.find_name(&self.latest.release)
    }

    /// Gets the latest snapshot (experimental) release.
    ///
    /// This only returns a `None` if the .latest field's
    /// data is *wrong* (impossible normally, if you just
    /// [`Manifest::download`] it). So it's mostly safe
    /// to unwrap.
    #[must_use]
    pub fn get_latest_snapshot(&self) -> Option<&Version> {
        self.find_name(&self.latest.snapshot)
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct Latest {
    pub release: String,
    pub snapshot: String,
}

#[allow(non_snake_case)]
#[derive(Deserialize, Clone, Debug)]
pub struct Version {
    pub id: String,
    pub r#type: String,
    pub url: String,
    pub time: String,
    pub releaseTime: String,
}

fn exclude_versions_after<T, F>(vec: &[T], predicate: F) -> Vec<T>
where
    T: Clone,
    F: FnMut(&T) -> bool,
{
    if let Some(pos) = vec.iter().position(predicate) {
        vec[pos..].to_vec()
    } else {
        Vec::new()
    }
}

fn include_versions_after<T, F>(vec: &[T], predicate: F) -> Vec<T>
where
    T: Clone,
    F: FnMut(&T) -> bool,
{
    if let Some(pos) = vec.iter().position(predicate) {
        vec[..pos].to_vec()
    } else {
        vec.to_owned()
    }
}
