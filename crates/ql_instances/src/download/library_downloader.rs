use std::{
    io::Cursor,
    path::{Path, PathBuf},
    sync::Mutex,
};

use ql_core::{
    do_jobs, err, file_utils, info,
    json::version::{
        Library, LibraryClassifier, LibraryDownloadArtifact, LibraryDownloads, LibraryExtract,
    },
    pt, DownloadError, DownloadProgress, IntoIoError, IoError, RequestError,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use zip_extract::ZipExtractError;

use crate::json_natives::{JsonNatives, NativesEntry};

use super::{constants::*, GameDownloader};

#[derive(Serialize, Deserialize)]
struct LwjglLibrary {
    libraries: Vec<Library>,
}

impl GameDownloader {
    pub async fn download_libraries(&mut self) -> Result<(), DownloadError> {
        info!("Starting download of libraries.");

        self.prepare_library_directories()?;

        let total_libraries = self.version_json.libraries.len();

        let num_library = Mutex::new(0);

        #[allow(unused_mut)]
        let mut replaced_names = Vec::new();

        #[cfg(all(target_arch = "aarch64", target_os = "linux"))]
        self.aarch64_patch_libs(&mut replaced_names)?;

        let results = self.version_json.libraries.iter().map(|lib| {
            self.download_library_fn(lib, &num_library, total_libraries, &replaced_names)
        });

        let outputs = do_jobs(results).await;

        if let Some(err) = outputs.into_iter().find_map(Result::err) {
            return Err(err);
        }

        #[cfg(all(target_arch = "aarch64", target_os = "linux"))]
        {
            // We don't want any x64 libraries on ARM, do we?
            let dir = self.instance_dir.join("libraries/natives/linux/x64");
            if dir.exists() {
                std::fs::remove_dir_all(&dir).path(dir)?;
            }
        }
        Ok(())
    }

    #[cfg(all(target_arch = "aarch64", target_os = "linux"))]
    fn aarch64_patch_libs(
        &mut self,
        replaced_names: &mut Vec<String>,
    ) -> Result<(), DownloadError> {
        Ok(
            for lwjgl in [
                LWJGL_294, LWJGL_312, LWJGL_316, LWJGL_321, LWJGL_322, LWJGL_331, LWJGL_332,
                LWJGL_333,
            ] {
                let lib: LwjglLibrary = serde_json::from_str(lwjgl)?;
                for lib in lib.libraries {
                    if let Some(library) = self
                        .version_json
                        .libraries
                        .iter_mut()
                        .find(|n| n.name == lib.name)
                    {
                        if let Some(name) = lib.name.clone() {
                            info!("Patching {name}");
                            replaced_names.push(name);
                        }
                        *library = lib;
                    }
                }
            },
        )
    }

    async fn download_library_fn(
        &self,
        library: &Library,
        library_i: &Mutex<usize>,
        library_len: usize,
        replaced_libs: &[String],
    ) -> Result<(), DownloadError> {
        if !GameDownloader::download_libraries_library_is_allowed(library) {
            info!("Skipping library:\n{library:#?}\n",);
            return Ok(());
        }

        self.download_library(library, replaced_libs).await?;

        {
            let mut library_i = library_i.lock().unwrap();
            self.send_progress(DownloadProgress::DownloadingLibraries {
                progress: *library_i,
                out_of: library_len,
            })?;
            *library_i += 1;
        }

        Ok(())
    }

    fn prepare_library_directories(&self) -> Result<(), IoError> {
        let library_path = self.instance_dir.join("libraries");
        std::fs::create_dir_all(&library_path).path(&library_path)?;
        let natives_path = library_path.join("natives");
        std::fs::create_dir_all(&natives_path).path(natives_path)?;
        Ok(())
    }

    async fn download_library(
        &self,
        library: &Library,
        replaced_libs: &[String],
    ) -> Result<(), DownloadError> {
        let libraries_dir = self.instance_dir.join("libraries");

        if let Some(LibraryDownloads {
            artifact,
            classifiers,
            ..
        }) = library.downloads.as_ref()
        {
            if let Some(artifact) = artifact {
                if let Some(name) = &library.name {
                    info!("Downloading {name}: {}", artifact.url);
                } else {
                    info!("Downloading {}", artifact.url);
                }

                let jar_file = self
                    .download_library_normal(artifact, &libraries_dir)
                    .await?;

                GameDownloader::extract_native_library(
                    &self.instance_dir,
                    &self.network_client,
                    library,
                    &jar_file,
                    artifact,
                    replaced_libs,
                )
                .await?;
            }
            if let Some(classifiers) = classifiers {
                self.download_library_native(classifiers, &libraries_dir, library.extract.as_ref())
                    .await?;
            }
        }
        Ok(())
    }

    /// Function to extract native libraries for Minecraft 1.16+
    /// (which uses a different format).
    pub async fn extract_native_library(
        instance_dir: &Path,
        client: &Client,
        library: &Library,
        jar_file: &[u8],
        artifact: &LibraryDownloadArtifact,
        replaced_libs: &[String],
    ) -> Result<(), DownloadError> {
        let natives_path = instance_dir.join("libraries/natives");

        if let Some(natives) = &library.natives {
            let is_valid = if cfg!(target_arch = "aarch64") && cfg!(target_os = "linux") {
                if let Some(name) = &library.name {
                    if replaced_libs.contains(name) {
                        true
                    } else {
                        pt!("Didn't replace {name}");
                        false
                    }
                } else {
                    pt!("Library doesn't have a name!");
                    false
                }
            } else {
                true
            };

            if is_valid {
                if let Some(natives_name) = natives.get(OS_NAME) {
                    info!("Extracting natives (1) {:?}", library.name);
                    pt!("Extracting main jar");

                    extract_zip_file(jar_file, &natives_path)
                        .map_err(DownloadError::NativesExtractError)?;

                    #[cfg(all(target_arch = "aarch64", target_os = "linux"))]
                    if library.name.as_deref() == Some("org.lwjgl.lwjgl:lwjgl-platform:2.9.0") {
                        // TODO: Find a better way to do this
                        let liblwjgl64_path = natives_path.join("liblwjgl64.so");
                        if liblwjgl64_path.exists() {
                            std::fs::remove_file(&liblwjgl64_path).path(liblwjgl64_path)?;
                        }
                        let libopenal64_path = natives_path.join("libopenal64.so");
                        if libopenal64_path.exists() {
                            std::fs::remove_file(&libopenal64_path).path(libopenal64_path)?;
                        }
                    }

                    let url = &artifact.url[..artifact.url.len() - 4];
                    let mut natives_url = format!("{url}-{natives_name}.jar");
                    if natives_url == "https://github.com/theofficialgman/lwjgl3-binaries-arm64/raw/lwjgl-3.1.6/lwjgl-jemalloc-natives-linux.jar" {
                        natives_url = "https://github.com/theofficialgman/lwjgl3-binaries-arm64/raw/lwjgl-3.1.6/lwjgl-jemalloc-patched-natives-linux-arm64.jar".to_owned();
                    }
                    if natives_url == "https://github.com/theofficialgman/lwjgl3-binaries-arm64/raw/lwjgl-3.1.6/lwjgl-core-natives-linux.jar" {
                        natives_url = "https://github.com/theofficialgman/lwjgl3-binaries-arm64/raw/lwjgl-3.1.6/lwjgl-natives-linux-arm64.jar".to_owned();
                    }
                    pt!("Downloading native jar");
                    let native_jar =
                        match file_utils::download_file_to_bytes(client, &natives_url, false).await
                        {
                            Ok(n) => n,
                            Err(RequestError::DownloadError { code, url }) => {
                                if code.as_u16() == 404
                                    && cfg!(target_arch = "aarch64")
                                    && cfg!(target_os = "linux")
                                {
                                    file_utils::download_file_to_bytes(
                                        client,
                                        &natives_url.replace("linux.jar", "linux-arm64.jar"),
                                        false,
                                    )
                                    .await?
                                } else {
                                    return Err(RequestError::DownloadError { code, url }.into());
                                }
                            }
                            Err(err) => Err(err)?,
                        };

                    pt!("Extracting native jar");
                    extract_zip_file(&native_jar, &natives_path)
                        .map_err(DownloadError::NativesExtractError)?;
                }
            } else {
                info!("Skipping natives (1) {:?}", library.name);
            }
        }

        if let Some(name) = &library.name {
            if name.contains("native") {
                let is_arm = cfg!(target_arch = "aarch64") && cfg!(target_os = "linux");

                // theofficialgman provides arm natives
                // https://github.com/theofficialgman/piston-meta-arm64
                let is_from_theofficialgman = if let Some(downloads) =
                    library.downloads.as_ref().and_then(|n| n.artifact.as_ref())
                {
                    downloads.url.contains("theofficialgman")
                } else {
                    false
                };

                let is_arm_native =
                    name.contains("arm") || name.contains("aarch") || is_from_theofficialgman;

                let is_compatible = is_arm == is_arm_native;

                if is_compatible {
                    info!("Downloading native (2) {name}");
                    let jar_file =
                        file_utils::download_file_to_bytes(client, &artifact.url, false).await?;
                    pt!("Extracting native");
                    extract_zip_file(&jar_file, &natives_path)
                        .map_err(DownloadError::NativesExtractError)?;
                } else {
                    download_other_platform_natives(name, client, natives_path).await?;
                }
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

        std::fs::create_dir_all(&lib_dir_path).path(lib_dir_path)?;
        let library_downloaded =
            file_utils::download_file_to_bytes(&self.network_client, &artifact.url, false).await?;

        std::fs::write(&lib_file_path, &library_downloaded).path(lib_file_path)?;

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
            if !OS_NAMES
                .iter()
                .any(|os_name| os.starts_with(&format!("natives-{os_name}")))
            {
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
                        std::fs::remove_dir_all(&exclusion_path).path(exclusion_path)?;
                    } else {
                        std::fs::remove_file(&exclusion_path).path(exclusion_path)?;
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

        if let Some(classifiers) = library
            .downloads
            .as_ref()
            .and_then(|n| n.classifiers.as_ref())
        {
            if supports_os(classifiers) {
                allowed = true;
            }
        }

        allowed
    }
}

async fn download_other_platform_natives(
    name: &String,
    client: &Client,
    natives_path: PathBuf,
) -> Result<(), DownloadError> {
    let Some(entry) = NativesEntry::get(name) else {
        err!("Native library not recognised: {name}");
        return Ok(());
    };

    let json = JsonNatives::download(entry).await?;
    for library in json
        .libraries
        .iter()
        .filter(|n| custom_natives_is_allowed(n))
    {
        let jar_file =
            file_utils::download_file_to_bytes(client, &library.downloads.artifact.url, false)
                .await?;

        extract_zip_file(&jar_file, &natives_path).map_err(DownloadError::NativesExtractError)?;
    }
    Ok(())
}

fn custom_natives_is_allowed(library: &crate::json_natives::NativeLibrary) -> bool {
    let Some(rules) = &library.rules else {
        return true;
    };
    let mut allowed = !rules.iter().any(|n| n.action == "allow");
    for (os, action) in rules
        .iter()
        .filter_map(|n| n.os.as_ref().map(|m| (m, &n.action)))
    {
        for os_name in OS_NAMES.iter().filter_map(|n| os.name.strip_prefix(n)) {
            if os_name.is_empty()
                || ((cfg!(target_arch = "x86_64") && os_name.contains("x86_64"))
                    || (cfg!(target_arch = "aarch64") && os_name.contains("arm64")))
            {
                allowed = action == "allow";
                break;
            }
        }
    }
    allowed
}

fn supports_os(classifiers: &std::collections::BTreeMap<String, LibraryClassifier>) -> bool {
    classifiers.iter().any(|(k, _)| {
        OS_NAMES
            .iter()
            .any(|n| k.starts_with(&format!("natives-{n}")))
    })
}

pub fn extract_zip_file(archive: &[u8], target_dir: &Path) -> Result<(), ZipExtractError> {
    zip_extract::extract(std::io::Cursor::new(archive), target_dir, true)?;
    Ok(())
}
