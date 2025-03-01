use std::{
    path::{Path, PathBuf},
    sync::{mpsc::Sender, Mutex},
};

use corretto::install_amazon_corretto_java;
use thiserror::Error;

use java_files::{JavaFile, JavaFilesJson};
use java_list::JavaListJson;
use ql_core::{
    do_jobs, err, file_utils, info, GenericProgress, IntoIoError, IoError, JsonDownloadError,
    RequestError,
};

mod compression;
mod corretto;
pub use compression::extract_tar_gz;

mod java_files;
mod java_list;

pub use java_list::JavaVersion;
use zip_extract::ZipExtractError;

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
/// use ql_java_handler::{get_java_binary, JavaVersion};
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
    install_normal_java(version, java_install_progress_sender, install_dir).await?;

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
    let Some(java_files_url) = java_list_json.get_url(version) else {
        return install_amazon_corretto_java(version, java_install_progress_sender, &install_dir)
            .await;
    };

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
    let file_num = {
        let mut file_num = file_num.lock().unwrap();
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
        *file_num
    } - 1;

    let file_path = install_dir.join(file_name);
    match file {
        JavaFile::file {
            downloads,
            executable,
        } => {
            info!("Installing file ({file_num}/{num_files}): {file_name}");
            let file_bytes = download_file(downloads).await?;
            tokio::fs::write(&file_path, &file_bytes)
                .await
                .path(file_path.clone())?;
            if *executable {
                #[cfg(target_family = "unix")]
                file_utils::set_executable(&file_path).await?;
            }
        }
        JavaFile::directory {} => {
            info!("Installing dir  ({file_num}/{num_files}): {file_name}");
            tokio::fs::create_dir_all(&file_path)
                .await
                .path(file_path)?;
        }
        JavaFile::link { target } => {
            // TODO: Deal with java install symlink.
            // file_utils::create_symlink(src, dest)
            err!("FIXME: Deal with symlink {file_name} -> {target}");
        }
    }
    Ok(())
}

async fn download_file(
    downloads: &java_files::JavaFileDownload,
) -> Result<Vec<u8>, JavaInstallError> {
    async fn normal_download(
        downloads: &java_files::JavaFileDownload,
    ) -> Result<Vec<u8>, JavaInstallError> {
        Ok(file_utils::download_file_to_bytes(&downloads.raw.url, false).await?)
    }

    let Some(lzma) = &downloads.lzma else {
        return normal_download(downloads).await;
    };
    let mut lzma = std::io::BufReader::new(std::io::Cursor::new(
        file_utils::download_file_to_bytes(&lzma.url, false).await?,
    ));

    let mut out = Vec::new();
    match lzma_rs::lzma_decompress(&mut lzma, &mut out) {
        Ok(()) => Ok(out),
        Err(err) => {
            err!(
                "Could not decompress lzma file: {err} ({})",
                downloads.raw.url
            );
            Ok(normal_download(downloads).await?)
        }
    }
}

#[derive(Debug, Error)]
pub enum JavaInstallError {
    #[error("couldn't install java: {0}")]
    JsonDownload(#[from] JsonDownloadError),
    #[error("couldn't install java: {0}")]
    Request(#[from] RequestError),
    #[error("couldn't extract java tar.gz: {0}")]
    TarGzExtract(std::io::Error),
    #[error("couldn't install java: json error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("couldn't install java: {0}")]
    Io(#[from] IoError),
    #[error("couldn't find java binary")]
    NoJavaBinFound,
    #[error("couldn't install java: zip extract error: {0}")]
    ZipExtract(#[from] ZipExtractError),
}
