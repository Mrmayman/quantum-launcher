use std::{
    error::Error,
    fmt::Display,
    io::Cursor,
    path::{Path, PathBuf},
    sync::{mpsc::Sender, Mutex},
};

use flate2::read::GzDecoder;
use tar::Archive;

use crate::{
    do_jobs, err, file_utils, info,
    json::{
        java_files::{JavaFile, JavaFilesJson},
        java_list::{JavaListJson, JavaVersion},
    },
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
/// # Side notes
/// - On aarch64 linux, this function installs Amazon Corretto Java.
/// - On all other platforms, this function installs Java from Mojang.
pub async fn get_java_binary(
    version: JavaVersion,
    name: &str,
    java_install_progress_sender: Option<Sender<GenericProgress>>,
) -> Result<PathBuf, JavaInstallError> {
    let launcher_dir = file_utils::get_launcher_dir()?;

    let java_dir = launcher_dir.join("java_installs").join(version.to_string());

    let is_incomplete_install = java_dir.join("install.lock").exists();

    if !java_dir.exists() || is_incomplete_install {
        info!("Installing Java: {version}");
        install_java(version, java_install_progress_sender.as_ref()).await?;
        info!("Finished installing Java {version}");
    }

    let java_dir = java_dir.join(if cfg!(windows) {
        format!("bin/{name}.exe")
    } else if cfg!(target_os = "macos") {
        if java_dir.join("bin/{name}").exists() {
            format!("bin/{name}")
        } else {
            format!("jre.bundle/Contents/Home/bin/{name}")
        }
    } else {
        format!("bin/{name}")
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
    // Create a GzDecoder to handle the .gz decompression
    let decoder = GzDecoder::new(Cursor::new(archive));

    // Create a TAR archive to handle the .tar extraction
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
    let client = reqwest::Client::new();
    let install_dir = get_install_dir(version)?;

    let lock_file = install_dir.join("install.lock");
    std::fs::write(
        &lock_file,
        "If you see this, java hasn't finished installing.",
    )
    .path(lock_file.clone())?;

    info!("Started installing {}", version.to_string());

    send_progress(java_install_progress_sender, GenericProgress::default());

    // Special case for linux aarch64
    if IS_ARM_LINUX {
        install_aarch64_linux_java(version, java_install_progress_sender, &client, &install_dir)
            .await?;
    } else {
        install_normal_java(version, client, java_install_progress_sender, install_dir).await?;
    }

    std::fs::remove_file(&lock_file).path(lock_file.clone())?;

    send_progress(java_install_progress_sender, GenericProgress::finished());

    info!("Finished installing {}", version.to_string());

    Ok(())
}

async fn install_normal_java(
    version: JavaVersion,
    client: reqwest::Client,
    java_install_progress_sender: Option<&Sender<GenericProgress>>,
    install_dir: PathBuf,
) -> Result<(), JavaInstallError> {
    let java_list_json = JavaListJson::download().await?;
    let java_files_url = java_list_json
        .get_url(version)
        .ok_or(JavaInstallError::NoUrlForJavaFiles)?;
    let json = file_utils::download_file_to_string(&client, &java_files_url, false).await?;
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
            &client,
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
    client: &reqwest::Client,
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
    let file_bytes = file_utils::download_file_to_bytes(client, url, false).await?;
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

fn get_install_dir(version: JavaVersion) -> Result<PathBuf, JavaInstallError> {
    let launcher_dir = file_utils::get_launcher_dir()?;
    let java_installs_dir = launcher_dir.join("java_installs");
    std::fs::create_dir_all(&java_installs_dir).path(java_installs_dir.clone())?;
    let install_dir = java_installs_dir.join(version.to_string());
    std::fs::create_dir_all(&install_dir).path(java_installs_dir.clone())?;
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
    client: &reqwest::Client,
) -> Result<(), JavaInstallError> {
    let file_path = install_dir.join(file_name);
    match file {
        JavaFile::file {
            downloads,
            executable,
        } => {
            let file_bytes =
                file_utils::download_file_to_bytes(client, &downloads.raw.url, false).await?;
            std::fs::write(&file_path, &file_bytes).path(file_path.clone())?;
            if *executable {
                #[cfg(target_family = "unix")]
                file_utils::set_executable(&file_path)?;
            }
        }
        JavaFile::directory {} => {
            std::fs::create_dir_all(&file_path).path(file_path)?;
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

#[derive(Debug)]
pub enum JavaInstallError {
    JsonDownload(JsonDownloadError),
    Request(RequestError),
    NoUrlForJavaFiles,
    TarGzExtract(std::io::Error),
    Serde(serde_json::Error),
    Io(IoError),
}

impl From<JsonDownloadError> for JavaInstallError {
    fn from(value: JsonDownloadError) -> Self {
        Self::JsonDownload(value)
    }
}

impl From<RequestError> for JavaInstallError {
    fn from(value: RequestError) -> Self {
        Self::Request(value)
    }
}

impl From<serde_json::Error> for JavaInstallError {
    fn from(value: serde_json::Error) -> Self {
        Self::Serde(value)
    }
}

impl From<IoError> for JavaInstallError {
    fn from(value: IoError) -> Self {
        Self::Io(value)
    }
}

impl Display for JavaInstallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "could not install java: ")?;
        match self {
            JavaInstallError::JsonDownload(err) => write!(f, "{err}"),
            JavaInstallError::NoUrlForJavaFiles => write!(f, "could not find url to download java"),
            JavaInstallError::Request(err) => write!(f, "{err}"),
            JavaInstallError::Serde(err) => write!(f, "(json) {err}"),
            JavaInstallError::Io(err) => write!(f, "(io) {err}"),
            JavaInstallError::TarGzExtract(error) => write!(f, "could not extract tar.gz: {error}"),
        }
    }
}

impl Error for JavaInstallError {}
