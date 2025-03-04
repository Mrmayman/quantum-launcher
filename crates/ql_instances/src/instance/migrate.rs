use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use ql_core::{
    info, json::version::LibraryDownloads, IntoIoError, CLASSPATH_SEPARATOR, LAUNCHER_VERSION_NAME,
};

use crate::{download::GameDownloader, LAUNCHER_VERSION};

use super::launch::{error::GameLaunchError, GameLauncher};

impl GameLauncher {
    pub async fn migrate_old_instances(&self) -> Result<(), GameLaunchError> {
        self.cleanup_junk_files().await?;

        let version = self.migrate_get_version().await?;

        self.migrate_natives(&version).await?;

        Ok(())
    }

    /// Gets the `semver::Version` of the launcher from
    /// the last time it was opened. Useful for migration.
    ///
    /// You can compare it with [`LAUNCHER_VERSION`] to
    /// see if the user upgraded to a new version.
    ///
    /// Note: after this call, the old version is only
    /// in-memory in the return value.
    /// The version file will have been updated.
    ///
    /// Essentially, you can only call this once.
    async fn migrate_get_version(&self) -> Result<semver::Version, GameLaunchError> {
        let launcher_version_path = self.instance_dir.join("launcher_version.txt");
        let mut version = if launcher_version_path.exists() {
            tokio::fs::read_to_string(&launcher_version_path)
                .await
                .path(&launcher_version_path)?
        } else {
            // launcher_version.txt was added in v0.2
            // so if it doesn't exist, this is v0.1
            tokio::fs::write(&launcher_version_path, "0.1")
                .await
                .path(&launcher_version_path)?;
            "0.1".to_owned()
        };
        if version.split('.').count() == 2 {
            version.push_str(".0");
        }
        let version = semver::Version::parse(&version)?;
        if version < LAUNCHER_VERSION {
            tokio::fs::write(&launcher_version_path, LAUNCHER_VERSION_NAME)
                .await
                .path(launcher_version_path)?;
        }
        Ok(version)
    }

    /// Download missing native libraries (affects launcher version 0.1 and 0.2)
    async fn migrate_natives(&self, version: &semver::Version) -> Result<(), GameLaunchError> {
        let v0_3 = semver::Version {
            major: 0,
            minor: 3,
            patch: 0,
            pre: semver::Prerelease::EMPTY,
            build: semver::BuildMetadata::EMPTY,
        };
        if version < &v0_3 {
            self.migrate_download_missing_native_libs().await?;
        }
        Ok(())
    }

    async fn migrate_download_missing_native_libs(&self) -> Result<(), GameLaunchError> {
        info!("Downloading missing native libraries");

        for library in &self.version_json.libraries {
            if !GameDownloader::download_libraries_library_is_allowed(library) {
                continue;
            }

            if let Some(LibraryDownloads {
                artifact: Some(artifact),
                ..
            }) = &library.downloads
            {
                let library_path = self.instance_dir.join("libraries").join(&artifact.path);
                let library_file = tokio::fs::read(&library_path).await.path(library_path)?;

                GameDownloader::extract_native_library(
                    &self.instance_dir,
                    library,
                    &library_file,
                    artifact,
                    &Vec::new(),
                )
                .await?;
            }
        }

        Ok(())
    }

    pub async fn migrate_create_forge_clean_classpath(
        &self,
        forge_classpath: String,
        classpath_entries: &mut HashSet<String>,
        classpath_entries_path: PathBuf,
    ) -> Result<(), GameLaunchError> {
        let forge_libs_dir = self.instance_dir.join("forge/libraries");
        let forge_libs_dir = forge_libs_dir
            .to_str()
            .ok_or(GameLaunchError::PathBufToString(forge_libs_dir.clone()))?;
        let mut temp_forge_classpath_entries = String::new();
        for entry in forge_classpath
            .split(CLASSPATH_SEPARATOR)
            .filter(|n| n.split_whitespace().any(|n| !n.is_empty()))
        {
            // /net/minecraftforge/forge/1.21.1-52.0.28/forge-1.21.1-52.0.28-universal.jar
            let entry = entry
                .strip_prefix(forge_libs_dir)
                .ok_or(GameLaunchError::ForgeInstallUpgradeStripPrefixError)?;

            // /.net.minecraftforge:forge
            let entry = transform_path(entry)
                .ok_or(GameLaunchError::ForgeInstallUpgradeTransformPathError)?;

            // net.minecraftforge:forge
            let entry = &entry[2..];

            classpath_entries.insert(entry.to_owned());
            temp_forge_classpath_entries.push_str(entry);
            temp_forge_classpath_entries.push('\n');
        }
        tokio::fs::write(&classpath_entries_path, temp_forge_classpath_entries)
            .await
            .path(classpath_entries_path)?;
        Ok(())
    }
}

/// Converts a path string into the desired format:
/// "/net/minecraftforge/forge/1.21.1-52.0.28/forge-1.21.1-52.0.28-universal.jar"
/// -> "net.minecraftforge:forge"
fn transform_path(input: &str) -> Option<String> {
    // Normalize the path separators for the current OS
    let path = Path::new(input);
    let components: Vec<&str> = path
        .iter()
        .map(|os_str| os_str.to_str().unwrap_or(""))
        .collect();

    if components.len() < 3 {
        // Ensure we have enough parts to remove the last two
        return None;
    }

    // Remove the last two parts
    let meaningful_parts = &components[..components.len() - 2];

    if meaningful_parts.is_empty() {
        return None;
    }

    // Join the parts into the desired format
    let mut result = meaningful_parts.join(".");
    if let Some(last_dot) = result.rfind('.') {
        result.replace_range(last_dot..=last_dot, ":");
    }

    Some(result)
}
