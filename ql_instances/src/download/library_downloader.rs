use std::{
    io::Cursor,
    path::{Path, PathBuf},
    sync::Mutex,
};

use reqwest::Client;
use zip_extract::ZipExtractError;

use crate::{
    download::{do_jobs, progress::DownloadProgress},
    error::IoError,
    file_utils, info, io_err,
    json_structs::json_version::{
        Library, LibraryClassifier, LibraryDownloadArtifact, LibraryDownloads, LibraryExtract,
    },
};

use super::{constants::OS_NAME, DownloadError, GameDownloader};

impl GameDownloader {
    pub async fn download_libraries(&self) -> Result<(), DownloadError> {
        info!("Starting download of libraries.");

        self.prepare_library_directories()?;

        let total_libraries = self.version_json.libraries.len();

        let bar = indicatif::ProgressBar::new(total_libraries as u64);

        let num_library = Mutex::new(0);

        let results = self
            .version_json
            .libraries
            .iter()
            .map(|lib| self.download_library_fn(&bar, lib, &num_library, total_libraries));

        let outputs = do_jobs(results).await;

        if let Some(err) = outputs.into_iter().find_map(Result::err) {
            return Err(err);
        }
        Ok(())
    }

    async fn download_library_fn(
        &self,
        bar: &indicatif::ProgressBar,
        library: &Library,
        library_i: &Mutex<usize>,
        library_len: usize,
    ) -> Result<(), DownloadError> {
        if !GameDownloader::download_libraries_library_is_allowed(library) {
            bar.println(format!("Skipping library:\n{library:#?}\n",));
            return Ok(());
        }

        self.download_library(library, bar).await?;

        {
            let mut library_i = library_i.lock().unwrap();
            self.send_progress(DownloadProgress::DownloadingLibraries {
                progress: *library_i,
                out_of: library_len,
            })?;
            *library_i += 1;
        }

        bar.inc(1);

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
        bar: &indicatif::ProgressBar,
    ) -> Result<(), DownloadError> {
        let libraries_dir = self.instance_dir.join("libraries");

        if let Some(downloads) = library.downloads.as_ref() {
            match downloads {
                LibraryDownloads::Normal { artifact, .. } => {
                    let jar_file = self
                        .download_library_normal(artifact, &libraries_dir)
                        .await?;

                    GameDownloader::extract_native_library(
                        &self.instance_dir,
                        &self.network_client,
                        library,
                        &jar_file,
                        artifact,
                        bar,
                    )
                    .await?;
                }
                LibraryDownloads::Native { classifiers } => {
                    self.download_library_native(
                        classifiers,
                        &libraries_dir,
                        library.extract.as_ref(),
                    )
                    .await?;
                }
            }
        }
        Ok(())
    }

    pub async fn extract_native_library(
        instance_dir: &Path,
        client: &Client,
        library: &Library,
        jar_file: &[u8],
        artifact: &LibraryDownloadArtifact,
        bar: &indicatif::ProgressBar,
    ) -> Result<(), DownloadError> {
        if let Some(natives) = &library.natives {
            if let Some(natives_name) = natives.get(OS_NAME) {
                bar.println("- Extracting natives: Extracting main jar");
                let natives_path = instance_dir.join("libraries/natives");

                extract_zip_file(jar_file, &natives_path)
                    .map_err(DownloadError::NativesExtractError)?;

                let url = &artifact.url[..artifact.url.len() - 4];
                let url = format!("{url}-{natives_name}.jar");
                bar.println("- Extracting natives: Downloading native jar");
                let native_jar = file_utils::download_file_to_bytes(client, &url, false).await?;

                bar.println("- Extracting natives: Extracting native jar");
                extract_zip_file(&native_jar, &natives_path)
                    .map_err(DownloadError::NativesExtractError)?;
            }
        }
        Ok(())
    }

    async fn download_library_normal(
        &self,
        artifact: &LibraryDownloadArtifact,
        libraries_dir: &Path,
    ) -> Result<Vec<u8>, DownloadError> {
        let lib_file_path = libraries_dir.join(PathBuf::from(&artifact.path));

        let lib_dir_path = lib_file_path
            .parent()
            .expect(
                "Downloaded java library does not have parent module like the sun in com.sun.java",
            )
            .to_path_buf();

        std::fs::create_dir_all(&lib_dir_path).map_err(io_err!(lib_dir_path))?;
        let library_downloaded =
            file_utils::download_file_to_bytes(&self.network_client, &artifact.url, false).await?;

        std::fs::write(&lib_file_path, &library_downloaded).map_err(io_err!(lib_file_path))?;

        Ok(library_downloaded)
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
                file_utils::download_file_to_bytes(&self.network_client, &download.url, false)
                    .await?;

            zip_extract::extract(Cursor::new(&library), &natives_dir, true)
                .map_err(DownloadError::NativesExtractError)?;
        }

        if let Some(extract) = extract {
            for exclusion in &extract.exclude {
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

    pub fn download_libraries_library_is_allowed(library: &Library) -> bool {
        let mut allowed: bool = true;

        if let Some(ref rules) = library.rules {
            allowed = false;

            for rule in rules {
                if let Some(ref os) = rule.os {
                    if os.name == OS_NAME {
                        allowed = rule.action == "allow";
                        if rule.action == "disallow" {
                            break;
                        }
                    }
                } else {
                    allowed = rule.action == "allow";
                    if rule.action == "disallow" {
                        break;
                    }
                }
            }
        }

        if !allowed {
            if let Some(LibraryDownloads::Native { classifiers }) = &library.downloads {
                if classifiers.contains_key(&format!("natives-{OS_NAME}")) {
                    allowed = true;
                }
            }
        }

        allowed
    }
}

pub fn extract_zip_file(archive: &[u8], target_dir: &Path) -> Result<(), ZipExtractError> {
    zip_extract::extract(std::io::Cursor::new(archive), target_dir, true)?;
    Ok(())
}
