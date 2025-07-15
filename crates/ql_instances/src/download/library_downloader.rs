use std::{
    collections::BTreeMap,
    io::Cursor,
    path::{Path, PathBuf},
    sync::Mutex,
};

use ql_core::{
    do_jobs, err, file_utils, info,
    json::version::{
        Library, LibraryClassifier, LibraryDownloadArtifact, LibraryDownloads, LibraryExtract,
    },
    pt, DownloadProgress, IntoIoError, IoError,
};
use zip_extract::ZipExtractError;

#[allow(clippy::wildcard_imports)]
use crate::download::constants::*;

use super::{DownloadError, GameDownloader};

const MACOS_ARM_LWJGL_294_1: &str = "https://libraries.minecraft.net/org/lwjgl/lwjgl/lwjgl-platform/2.9.4-nightly-20150209/lwjgl-platform-2.9.4-nightly-20150209-natives-osx.jar";
const MACOS_ARM_LWJGL_294_2: &str = "https://github.com/Dungeons-Guide/lwjgl/releases/download/2.9.4-20150209-mmachina.2-syeyoung.1/lwjgl-platform-2.9.4-nightly-20150209-natives-osx-arm64.jar";

impl GameDownloader {
    pub async fn download_libraries(&mut self) -> Result<(), DownloadError> {
        info!("Starting download of libraries.");

        self.prepare_library_directories().await?;

        let total_libraries = self.version_json.libraries.len();

        let num_library = Mutex::new(0);

        let results = self
            .version_json
            .libraries
            .iter()
            .map(|lib| self.download_library_fn(lib, &num_library, total_libraries));

        // Uncomment for synchronous downloads. WAY slower,
        // but easier to debug/inspect logs of,
        // if you're working on the library downloader

        // for job in results {
        //     job.await?;
        // }

        // The one below is the concurrent downloader, downloading multiple
        // libraries at the same time. If you uncomment the above one, make sure
        // to comment this below one out.
        // This is WAY faster but harder to debug/inspect
        _ = do_jobs(results).await?;

        Ok(())
    }

    async fn download_library_fn(
        &self,
        library: &Library,
        library_i: &Mutex<usize>,
        library_len: usize,
    ) -> Result<(), DownloadError> {
        if !GameDownloader::download_libraries_library_is_allowed(library) {
            info!("Skipping library:\n{library:#?}\n",);
            return Ok(());
        }

        self.download_library(library).await?;

        {
            let mut library_i = library_i.lock().unwrap();
            self.send_progress(
                DownloadProgress::DownloadingLibraries {
                    progress: *library_i,
                    out_of: library_len,
                },
                true,
            );
            *library_i += 1;
        }

        Ok(())
    }

    async fn prepare_library_directories(&self) -> Result<(), IoError> {
        let library_path = self.instance_dir.join("libraries");
        tokio::fs::create_dir_all(&library_path)
            .await
            .path(&library_path)?;
        let natives_path = library_path.join("natives");
        tokio::fs::create_dir_all(&natives_path)
            .await
            .path(natives_path)?;
        Ok(())
    }

    pub async fn download_library(&self, library: &Library) -> Result<(), DownloadError> {
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

                let natives_path = self.instance_dir.join("libraries/natives");
                extractlib_natives_field(
                    library,
                    classifiers.as_ref(),
                    &jar_file,
                    &natives_path,
                    artifact,
                )
                .await?;
                extractlib_name_natives(library, artifact, natives_path).await?;
            }
            if let Some(classifiers) = classifiers {
                self.download_library_native(classifiers, &libraries_dir, library.extract.as_ref())
                    .await?;
            }
        }
        Ok(())
    }

    /// Simplified function to extract native libraries.
    ///
    /// This is only used to migrate from QuantumLauncher
    /// v0.1/0.2 to 0.3 or above.
    ///
    /// This function only supports Windows and Linux for x86_64
    /// since it doesn't have special library handling logic for
    /// other platforms, because the old versions being migrated from
    /// didn't support other platforms in the first place.
    ///
    /// For "real" library downloading when creating an instance
    /// see [`GameDownloader::download_library_fn`]
    #[allow(clippy::doc_markdown)]
    pub async fn migrate_extract_native_library(
        instance_dir: &Path,
        library: &Library,
        jar_file: &[u8],
        artifact: &LibraryDownloadArtifact,
    ) -> Result<(), DownloadError> {
        let natives_path = instance_dir.join("libraries/natives");

        // Why 2 functions? Because unfortunately there are multiple formats
        // natives can come in, and we need to support all of them.
        extractlib_natives_field(
            library,
            Some(&BTreeMap::new()),
            jar_file,
            &natives_path,
            artifact,
        )
        .await?;

        extractlib_name_natives(library, artifact, natives_path).await?;

        Ok(())
    }

    async fn download_library_normal(
        &self,
        artifact: &LibraryDownloadArtifact,
        libraries_dir: &Path,
    ) -> Result<Vec<u8>, DownloadError> {
        let lib_file_path = libraries_dir.join(PathBuf::from(artifact.get_path()));

        let lib_dir_path = lib_file_path
            .parent()
            .expect(
                "Downloaded java library does not have parent module like the sun in com.sun.java",
            )
            .to_path_buf();

        tokio::fs::create_dir_all(&lib_dir_path)
            .await
            .path(lib_dir_path)?;
        let library_downloaded = file_utils::download_file_to_bytes(&artifact.url, false).await?;

        tokio::fs::write(&lib_file_path, &library_downloaded)
            .await
            .path(lib_file_path)?;

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
            #[allow(unused)]
            if !(OS_NAMES.iter().any(|os_name| {
                #[cfg(all(target_os = "windows", target_arch = "x86"))]
                let matches = os == "natives-windows-32";
                #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
                let matches = (os == "natives-windows-64") || (os == "natives-windows");

                #[cfg(any(
                    all(target_os = "linux", target_arch = "aarch64"),
                    feature = "simulate_linux_arm64"
                ))]
                let matches = os == "natives-linux-arm64";
                #[cfg(all(target_os = "linux", target_arch = "arm"))]
                let matches = os == "natives-linux-arm32";

                #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
                let matches = os == "natives-osx-arm64";

                #[cfg(not(any(
                    all(
                        target_os = "windows",
                        any(target_arch = "x86_64", target_arch = "x86")
                    ),
                    all(
                        target_os = "linux",
                        any(
                            target_arch = "aarch64",
                            target_arch = "arm",
                            feature = "simulate_linux_arm64"
                        )
                    ),
                    all(target_os = "macos", target_arch = "aarch64")
                )))]
                let matches = *os == format!("natives-{os_name}");

                matches
            })) {
                pt!("Skipping OS: {os}");
                continue;
            }

            let url = if download.url == MACOS_ARM_LWJGL_294_1 {
                info!("Patching LWJGL 2.9.4 20150209 natives for OSX ARM64 (classifiers)");
                MACOS_ARM_LWJGL_294_2
            } else {
                &download.url
            };
            info!("Downloading natives (classifiers): {url}");

            let library = file_utils::download_file_to_bytes(url, false).await?;

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
                        tokio::fs::remove_dir_all(&exclusion_path)
                            .await
                            .path(exclusion_path)?;
                    } else {
                        tokio::fs::remove_file(&exclusion_path)
                            .await
                            .path(exclusion_path)?;
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
                    #[cfg(any(
                        target_arch = "aarch64",
                        target_arch = "arm",
                        target_arch = "x86",
                        feature = "simulate_linux_arm64"
                    ))]
                    let target = format!("{OS_NAME}-{ARCH}");

                    #[cfg(not(any(
                        target_arch = "aarch64",
                        target_arch = "arm",
                        target_arch = "x86",
                        feature = "simulate_linux_arm64"
                    )))]
                    let target = OS_NAME;

                    if os.name == target {
                        allowed = rule.action == "allow";
                    }
                } else {
                    allowed = rule.action == "allow";
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

async fn extractlib_name_natives(
    library: &Library,
    artifact: &LibraryDownloadArtifact,
    natives_path: PathBuf,
) -> Result<(), DownloadError> {
    let Some(name) = &library.name else {
        return Ok(());
    };

    if !name.contains("native") {
        return Ok(());
    }

    #[cfg(target_arch = "arm")]
    let is_compatible = name.contains("arm32");
    #[cfg(target_arch = "x86")]
    let is_compatible = name.contains("x86") && !name.contains("x86_64");
    #[cfg(any(target_arch = "aarch64", feature = "simulate_linux_arm64"))]
    let is_compatible = name.contains("aarch") || name.contains("arm64");
    #[cfg(not(any(
        target_arch = "aarch64",
        target_arch = "arm",
        target_arch = "x86",
        feature = "simulate_linux_arm64"
    )))]
    let is_compatible = !(name.contains("aarch")
        || name.contains("arm")
        || (name.contains("x86") && !name.contains("x86_64")));

    if is_compatible {
        info!("Downloading native (2): {name}");
        let jar_file = file_utils::download_file_to_bytes(&artifact.url, false).await?;
        pt!("Extracting native: {name}");
        extract_zip_file(&jar_file, &natives_path).map_err(DownloadError::NativesExtractError)?;
    }

    Ok(())
}

async fn extractlib_natives_field(
    library: &Library,
    classifiers: Option<&std::collections::BTreeMap<String, LibraryClassifier>>,
    jar_file: &[u8],
    natives_path: &Path,
    artifact: &LibraryDownloadArtifact,
) -> Result<(), DownloadError> {
    let name = library.name.as_deref().unwrap_or_default();

    let Some(natives) = &library.natives else {
        return Ok(());
    };

    #[cfg(any(
        target_arch = "aarch64",
        target_arch = "arm",
        target_arch = "x86",
        feature = "simulate_linux_arm64"
    ))]
    let Some(natives_name) = natives.get(&format!("{OS_NAME}-{ARCH}")) else {
        return Ok(());
    };
    #[cfg(not(any(
        target_arch = "aarch64",
        target_arch = "arm",
        feature = "simulate_linux_arm64"
    )))]
    let Some(natives_name) = natives.get(OS_NAME) else {
        return Ok(());
    };

    info!("Extracting natives (1): {name}");
    pt!("Extracting main jar: {name}");

    extract_zip_file(jar_file, natives_path).map_err(DownloadError::NativesExtractError)?;

    let natives_url = if let Some(classifiers) = classifiers {
        if let Some(natives) = classifiers.get(natives_name) {
            if natives.url == "https://github.com/MinecraftMachina/lwjgl/releases/download/2.9.4-20150209-mmachina.2/lwjgl-platform-2.9.4-nightly-20150209-natives-osx.jar" {
                // Updated fork, fixes crash on macOS aarch64 when resizing windows
                "https://github.com/Dungeons-Guide/lwjgl/releases/download/2.9.4-20150209-mmachina.2-syeyoung.1/lwjgl-platform-2.9.4-nightly-20150209-natives-osx-arm64.jar".to_owned()
            } else {
                natives.url.clone()
            }
        } else {
            err!("{name}: No matching `classifiers.natives-*` entry found for {natives_name}");
            return Ok(());
        }
    } else {
        let url = &artifact.url[..artifact.url.len() - 4];
        let mut natives_url = format!("{url}-{natives_name}.jar");

        if natives_url == "https://github.com/theofficialgman/lwjgl3-binaries-arm64/raw/lwjgl-3.1.6/lwjgl-jemalloc-natives-linux.jar" {
            "https://github.com/theofficialgman/lwjgl3-binaries-arm64/raw/lwjgl-3.1.6/lwjgl-jemalloc-patched-natives-linux-arm64.jar".clone_into(&mut natives_url);
        }

        #[cfg(any(target_arch = "aarch64", feature = "simulate_linux_arm64"))]
        {
            if natives_url == MACOS_ARM_LWJGL_294_1 {
                info!("Patching LWJGL 2.9.4 20150209 natives for OSX ARM64");
                MACOS_ARM_LWJGL_294_2.clone_into(&mut natives_url);
            }
            if natives_url.ends_with("lwjgl-core-natives-linux.jar") {
                natives_url = natives_url.replace(
                    "lwjgl-core-natives-linux.jar",
                    "lwjgl-natives-linux-arm64.jar",
                );
            }
        }

        natives_url
    };

    pt!("Downloading native jar: {name}\n  ({natives_url})");
    let native_jar = match file_utils::download_file_to_bytes(&natives_url, false).await {
        Ok(n) => n,
        #[cfg(any(
            all(target_os = "linux", target_arch = "aarch64"),
            feature = "simulate_linux_arm64"
        ))]
        Err(ql_core::RequestError::DownloadError { code, .. }) if code.as_u16() == 404 => {
            file_utils::download_file_to_bytes(
                &natives_url.replace("linux.jar", "linux-arm64.jar"),
                false,
            )
            .await?
        }
        Err(err) => Err(err)?,
    };

    pt!("Extracting native jar: {name}");
    extract_zip_file(&native_jar, natives_path).map_err(DownloadError::NativesExtractError)?;

    Ok(())
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
