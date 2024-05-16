use std::{
    io::Cursor,
    path::{Path, PathBuf},
};

use crate::{
    download::progress::DownloadProgress,
    error::IoError,
    file_utils, io_err,
    json_structs::json_version::{
        Library, LibraryClassifier, LibraryDownloadArtifact, LibraryDownloads, LibraryExtract,
    },
};

use super::{constants::OS_NAME, DownloadError, GameDownloader};

impl GameDownloader {
    pub async fn download_libraries(&self) -> Result<(), DownloadError> {
        println!("[info] Starting download of libraries.");

        self.prepare_library_directories()?;

        let total_libraries = self.version_json.libraries.len();

        for (library_number, library) in self.version_json.libraries.iter().enumerate() {
            self.send_progress(DownloadProgress::DownloadingLibraries {
                progress: library_number,
                out_of: total_libraries,
            })?;

            if !GameDownloader::download_libraries_library_is_allowed(library) {
                println!(
                    "[info] Skipping library {}",
                    serde_json::to_string_pretty(&library)?
                );
                continue;
            }

            self.download_library(library, (library_number, total_libraries))
                .await?;
        }
        Ok(())
    }

    fn prepare_library_directories(&self) -> Result<(), IoError> {
        let library_path = self.instance_dir.join("libraries");
        std::fs::create_dir_all(&library_path).map_err(io_err!(library_path))?;
        let natives_path = library_path.join("natives");
        std::fs::create_dir_all(&natives_path).map_err(io_err!(natives_path))?;
        Ok(())
    }

    async fn download_library(
        &self,
        library: &Library,
        (library_number, number_of_libraries): (usize, usize),
    ) -> Result<(), DownloadError> {
        let libraries_dir = self.instance_dir.join("libraries");

        if let Some(downloads) = library.downloads.as_ref() {
            match downloads {
                LibraryDownloads::Normal { artifact, .. } => {
                    self.download_library_normal(
                        artifact,
                        &libraries_dir,
                        (library_number, number_of_libraries),
                    )
                    .await?
                }
                LibraryDownloads::Native { classifiers } => {
                    self.download_library_native(
                        classifiers,
                        &libraries_dir,
                        library.extract.as_ref(),
                    )
                    .await?
                }
            }
        }
        Ok(())
    }

    async fn download_library_normal(
        &self,
        artifact: &LibraryDownloadArtifact,
        libraries_dir: &Path,
        (library_number, number_of_libraries): (usize, usize),
    ) -> Result<(), DownloadError> {
        let lib_file_path = libraries_dir.join(PathBuf::from(&artifact.path));

        let lib_dir_path = lib_file_path
            .parent()
            .expect(
                "Downloaded java library does not have parent module like the sun in com.sun.java",
            )
            .to_path_buf();

        println!(
            "[info] Downloading library {library_number}/{number_of_libraries}: {}",
            artifact.path
        );

        std::fs::create_dir_all(&lib_dir_path).map_err(io_err!(lib_dir_path))?;
        let library_downloaded =
            file_utils::download_file_to_bytes(&self.network_client, &artifact.url).await?;

        std::fs::write(&lib_file_path, library_downloaded).map_err(io_err!(lib_file_path))?;

        Ok(())
    }

    async fn download_library_native(
        &self,
        classifiers: &std::collections::BTreeMap<String, LibraryClassifier>,
        libraries_dir: &Path,
        extract: Option<&LibraryExtract>,
    ) -> Result<(), DownloadError> {
        let natives_dir = libraries_dir.join("natives");

        for (os, download) in classifiers {
            if *os != format!("natives-{OS_NAME}") {
                continue;
            }

            let library =
                file_utils::download_file_to_bytes(&self.network_client, &download.url).await?;

            zip_extract::extract(Cursor::new(&library), &natives_dir, true)
                .map_err(DownloadError::NativesExtractError)?;
        }

        if let Some(extract) = extract {
            for exclusion in extract.exclude.iter() {
                let exclusion_path = natives_dir.join(exclusion);

                if !exclusion_path.starts_with(&natives_dir) {
                    return Err(DownloadError::NativesOutsideDirRemove);
                }

                if exclusion_path.exists() {
                    if exclusion_path.is_dir() {
                        std::fs::remove_dir_all(&exclusion_path)
                            .map_err(io_err!(exclusion_path))?;
                    } else {
                        std::fs::remove_file(&exclusion_path).map_err(io_err!(exclusion_path))?;
                    }
                }
            }
        }

        Ok(())
    }

    fn download_libraries_library_is_allowed(library: &Library) -> bool {
        let mut allowed: bool = true;

        if let Some(ref rules) = library.rules {
            allowed = false;

            for rule in rules.iter() {
                if let Some(ref os) = rule.os {
                    if os.name == OS_NAME {
                        allowed = rule.action == "allow";
                    }
                } else {
                    allowed = rule.action == "allow";
                }
            }
        }
        allowed
    }
}
