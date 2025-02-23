use std::{
    io::Cursor,
    path::{Path, PathBuf},
    sync::{mpsc::Sender, Mutex},
};

use flate2::read::GzDecoder;
use tar::Archive;
use thiserror::Error;

use crate::{
    do_jobs, err, file_utils, info,
    json::{JavaFile, JavaFilesJson, JavaListJson, JavaVersion},
    GenericProgress, IntoIoError, IoError, JsonDownloadError, RequestError, IS_ARM_LINUX,
};

/// Returns a `PathBuf` pointing to a Java executable of your choice.
///
/// This downloads and installs Java if not already installed,
/// and if already installed, uses the existing installation.
///
/// # Arguments
/// - `version`: The version of Java you want to use ([`JavaVersion`]).
/// - `name`: The name of the executable you want to use.
///   For example, "java" for the Java runtime, or "javac" for the Java compiler.
/// - `java_install_progress_sender`: An optional `Sender<GenericProgress>`
///   to send progress updates to. If not needed, simply pass `None` to the function.
///   If you want, you can hook this up to a progress bar, by using a
///   `std::sync::mpsc::channel::<JavaInstallMessage>()`,
///   giving the sender to this function and polling the receiver frequently.
///
/// # Errors
/// If the Java installation fails, this function returns a [`JavaInstallError`].
/// There's a lot of possible errors, so I'm not going to list them all here.
///
/// # Example
/// ```no_run
/// # async fn get() -> Result<(), Box<dyn std::error::Error>> {
/// use ql_core::{get_java_binary, json::JavaVersion};
/// use std::path::PathBuf;
///
/// let java_binary: PathBuf = get_java_binary(JavaVersion::Java16, "java", None).await?;
///
/// let command = std::process::Command::new(java_binary).arg("-version").output()?;
///
/// let java_compiler_binary: PathBuf = get_java_binary(JavaVersion::Java16, "javac", None).await?;
///
/// let command = std::process::Command::new(java_compiler_binary)
///     .args(&["MyApp.java", "-d", "."])
///     .output()?;
/// # Ok(())
/// # }
/// ```
///
/// # Side notes
/// - On aarch64 linux, this function installs Amazon Corretto Java.
/// - On all other platforms, this function installs Java from Mojang.
pub async fn get_java_binary(
    version: JavaVersion,
    name: &str,
    java_install_progress_sender: Option<&Sender<GenericProgress>>,
) -> Result<PathBuf, JavaInstallError> {
    let launcher_dir = file_utils::get_launcher_dir().await?;

    let java_dir = launcher_dir.join("java_installs").join(version.to_string());

    let is_incomplete_install = java_dir.join("install.lock").exists();

    if !java_dir.exists() || is_incomplete_install {
        info!("Installing Java: {version}");
        install_java(version, java_install_progress_sender).await?;
        info!("Finished installing Java {version}");
    }

    let normal_name = format!("bin/{name}");
    let java_dir = java_dir.join(if java_dir.join(&normal_name).exists() {
        normal_name
    } else if cfg!(target_os = "windows") {
        format!("bin/{name}.exe")
    } else if cfg!(target_os = "macos") {
        format!("jre.bundle/Contents/Home/bin/{name}")
    } else {
        return Err(JavaInstallError::NoJavaBinFound);
    });

    Ok(java_dir.canonicalize().path(java_dir)?)
}

/// Extracts a `.tar.gz` file from a `&[u8]` buffer into the given directory.
/// Does not create a top-level directory,
/// extracting files directly into the target directory.
///
/// # Arguments
/// - `data`: A reference to the `.tar.gz` file as a byte slice.
/// - `output_dir`: Path to the directory where the contents will be extracted.
///
/// # Returns
/// - `Result<()>` on success, or an error otherwise.
pub fn extract_tar_gz(archive: &[u8], output_dir: &Path) -> std::io::Result<()> {
    // For extracting the `.gz`
    let decoder = GzDecoder::new(Cursor::new(archive));
    // For extracting the `.tar`
    let mut tar = Archive::new(decoder);

    // Get the first entry path to determine the top-level directory
    let mut entries = tar.entries()?;
    let top_level_dir = if let Some(entry) = entries.next() {
        let entry = entry?;
        let path = entry
            .path()?
            .components()
            .next()
            .map(|c| c.as_os_str().to_os_string());
        path
    } else {
        None
    };

    // Rewind the archive to process all entries
    let decoder = GzDecoder::new(Cursor::new(archive));
    let mut tar = Archive::new(decoder);

    // Extract files while flattening the top-level directory
    for entry in tar.entries()? {
        let mut entry = entry?;

        // Get the path of the file in the archive
        let entry_path = entry.path()?;

        // Remove the top-level directory from the path
        let new_path = match top_level_dir.as_ref() {
            Some(top_level) if entry_path.starts_with(top_level) => entry_path
                .strip_prefix(top_level)
                .map_err(|_| {
                    std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        format!("Could not strip prefix {entry_path:?}, {top_level:?}"),
                    )
                })?
                .to_path_buf(),
            _ => entry_path.to_path_buf(),
        };

        // Resolve the full output path
        let full_path = output_dir.join(new_path);

        // Ensure parent directories exist
        if let Some(parent) = full_path.parent() {
            // Not using async due to some weird thread safety error
            std::fs::create_dir_all(parent)?;
        }

        // Unpack the file or directory
        entry.unpack(full_path)?;
    }

    Ok(())
}

async fn install_java(
    version: JavaVersion,
    java_install_progress_sender: Option<&Sender<GenericProgress>>,
) -> Result<(), JavaInstallError> {
    let install_dir = get_install_dir(version).await?;

    let lock_file = install_dir.join("install.lock");
    tokio::fs::write(
        &lock_file,
        "If you see this, java hasn't finished installing.",
    )
    .await
    .path(lock_file.clone())?;

    info!("Started installing {}", version.to_string());

    send_progress(java_install_progress_sender, GenericProgress::default());

    // Special case for linux aarch64
    if IS_ARM_LINUX {
        install_aarch64_linux_java(version, java_install_progress_sender, &install_dir).await?;
    } else {
        install_normal_java(version, java_install_progress_sender, install_dir).await?;
    }

    tokio::fs::remove_file(&lock_file)
        .await
        .path(lock_file.clone())?;

    send_progress(java_install_progress_sender, GenericProgress::finished());

    info!("Finished installing {}", version.to_string());

    Ok(())
}

async fn install_normal_java(
    version: JavaVersion,
    java_install_progress_sender: Option<&Sender<GenericProgress>>,
    install_dir: PathBuf,
) -> Result<(), JavaInstallError> {
    let java_list_json = JavaListJson::download().await?;
    let java_files_url = java_list_json
        .get_url(version)
        .ok_or(JavaInstallError::NoUrlForJavaFiles)?;

    let json = file_utils::download_file_to_string(&java_files_url, false).await?;
    let json: JavaFilesJson = serde_json::from_str(&json)?;

    let num_files = json.files.len();
    let file_num = Mutex::new(0);

    let results = json.files.iter().map(|(file_name, file)| {
        java_install_fn(
            java_install_progress_sender,
            &file_num,
            num_files,
            file_name,
            &install_dir,
            file,
        )
    });
    let outputs = do_jobs(results).await;

    if let Some(err) = outputs.into_iter().find_map(Result::err) {
        return Err(err);
    }

    Ok(())
}

async fn install_aarch64_linux_java(
    version: JavaVersion,
    java_install_progress_sender: Option<&Sender<GenericProgress>>,
    install_dir: &Path,
) -> Result<(), JavaInstallError> {
    let url = version.get_amazon_corretto_aarch64_url();
    send_progress(
        java_install_progress_sender,
        GenericProgress {
            done: 0,
            total: 2,
            message: Some("Getting tar.gz archive".to_owned()),
            has_finished: false,
        },
    );
    let file_bytes = file_utils::download_file_to_bytes(url, false).await?;
    send_progress(
        java_install_progress_sender,
        GenericProgress {
            done: 1,
            total: 2,
            message: Some("Extracting tar.gz archive".to_owned()),
            has_finished: false,
        },
    );
    extract_tar_gz(&file_bytes, install_dir).map_err(JavaInstallError::TarGzExtract)?;
    Ok(())
}

async fn get_install_dir(version: JavaVersion) -> Result<PathBuf, JavaInstallError> {
    let launcher_dir = file_utils::get_launcher_dir().await?;
    let java_installs_dir = launcher_dir.join("java_installs");
    tokio::fs::create_dir_all(&java_installs_dir)
        .await
        .path(java_installs_dir.clone())?;
    let install_dir = java_installs_dir.join(version.to_string());
    tokio::fs::create_dir_all(&install_dir)
        .await
        .path(java_installs_dir.clone())?;
    Ok(install_dir)
}

fn send_progress(
    java_install_progress_sender: Option<&Sender<GenericProgress>>,
    progress: GenericProgress,
) {
    if let Some(java_install_progress_sender) = java_install_progress_sender {
        if let Err(err) = java_install_progress_sender.send(progress) {
            err!("Error sending java install progress: {err}\nThis should probably be safe to ignore");
        }
    }
}

async fn java_install_fn(
    java_install_progress_sender: Option<&Sender<GenericProgress>>,
    file_num: &Mutex<usize>,
    num_files: usize,
    file_name: &str,
    install_dir: &Path,
    file: &JavaFile,
) -> Result<(), JavaInstallError> {
    let file_path = install_dir.join(file_name);
    match file {
        JavaFile::file {
            downloads,
            executable,
        } => {
            let file_bytes = file_utils::download_file_to_bytes(&downloads.raw.url, false).await?;
            tokio::fs::write(&file_path, &file_bytes)
                .await
                .path(file_path.clone())?;
            if *executable {
                #[cfg(target_family = "unix")]
                file_utils::set_executable(&file_path).await?;
            }
        }
        JavaFile::directory {} => {
            tokio::fs::create_dir_all(&file_path)
                .await
                .path(file_path)?;
        }
        JavaFile::link { target } => {
            // TODO: Deal with java install symlink.
            err!("[fixme:install_java] Deal with symlink {file_name} -> {target}");
        }
    }
    {
        let mut file_num = file_num.lock().unwrap();
        info!("Installing file ({file_num}/{num_files}): {file_name}");
        send_progress(
            java_install_progress_sender,
            GenericProgress {
                done: *file_num,
                total: num_files,
                message: Some(format!("Installing file: {file_name}")),
                has_finished: false,
            },
        );
        *file_num += 1;
    }
    Ok(())
}

#[derive(Debug, Error)]
pub enum JavaInstallError {
    #[error("couldn't install java: {0}")]
    JsonDownload(#[from] JsonDownloadError),
    #[error("couldn't install java: {0}")]
    Request(#[from] RequestError),
    #[error("could not find url to download java")]
    NoUrlForJavaFiles,
    #[error("could not extract java tar.gz: {0}")]
    TarGzExtract(std::io::Error),
    #[error("couldn't install java: json error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("couldn't install java: {0}")]
    Io(#[from] IoError),
    #[error("could not find java binary")]
    NoJavaBinFound,
}
