use ql_core::{info, io_err, json::version::LibraryDownloads};

use crate::{download::GameDownloader, LAUNCHER_VERSION};

use super::launch::{error::GameLaunchError, GameLauncher};

impl GameLauncher {
    pub async fn migrate_old_instances(&self) -> Result<(), GameLaunchError> {
        let launcher_version_path = self.instance_dir.join("launcher_version.txt");
        let mut version = if launcher_version_path.exists() {
            std::fs::read_to_string(&launcher_version_path)
                .map_err(io_err!(launcher_version_path))?
        } else {
            std::fs::write(&launcher_version_path, "0.1")
                .map_err(io_err!(launcher_version_path))?;
            "0.1".to_owned()
        };
        if version.split('.').count() == 2 {
            version.push_str(".0");
        }

        let version = semver::Version::parse(&version)?;

        let client = reqwest::Client::new();
        if version < LAUNCHER_VERSION {
            self.migrate_download_missing_native_libs(&client).await?;
        }

        Ok(())
    }

    async fn migrate_download_missing_native_libs(
        &self,
        client: &reqwest::Client,
    ) -> Result<(), GameLaunchError> {
        info!("Downloading missing native libraries");
        let bar = indicatif::ProgressBar::new(self.version_json.libraries.len() as u64);

        for library in &self.version_json.libraries {
            if !GameDownloader::download_libraries_library_is_allowed(library) {
                continue;
            }

            if let Some(LibraryDownloads::Normal { artifact, .. }) = &library.downloads {
                let library_path = self.instance_dir.join("libraries").join(&artifact.path);
                let library_file = std::fs::read(&library_path).map_err(io_err!(library_path))?;

                GameDownloader::extract_native_library(
                    &self.instance_dir,
                    client,
                    library,
                    &library_file,
                    artifact,
                    &bar,
                )
                .await?;
            }
        }

        Ok(())
    }
}
