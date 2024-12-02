use std::{
    error::Error,
    fmt::Display,
    path::{Path, PathBuf},
    sync::{mpsc::Sender, Mutex},
};

use crate::{
    download::do_jobs,
    err,
    error::IoError,
    file_utils::{self, RequestError},
    info, io_err,
    json_structs::{
        json_java_files::{JavaFile, JavaFilesJson},
        json_java_list::{JavaListJson, JavaVersion},
        JsonDownloadError,
    },
};

pub enum JavaInstallProgress {
    P1Started,
    P2 {
        done: usize,
        out_of: usize,
        name: String,
    },
    P3Done,
}

/// Returns a `PathBuf` pointing to the Java executable.
/// You can select which Java version you want through the `version` argument.
///
/// The name argument can be made "java" for launching the game, unless you want something else like "javac"
/// (the java compiler).
///
/// This downloads and installs Java if not already installed,
/// and if already installed, uses the existing installation.
///
/// If you want, you can hook this up to a progress bar, by using a
/// `std::sync::mpsc::channel::<JavaInstallMessage>()`, giving the
/// sender to this function and polling the receiver frequently.
/// If not needed, simply pass `None` to the function.
pub async fn get_java_binary(
    version: JavaVersion,
    name: &str,
    java_install_progress_sender: Option<Sender<JavaInstallProgress>>,
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
    } else {
        format!("bin/{name}")
    });

    Ok(java_dir.canonicalize().map_err(io_err!(java_dir))?)
}

async fn install_java(
    version: JavaVersion,
    java_install_progress_sender: Option<&Sender<JavaInstallProgress>>,
) -> Result<(), JavaInstallError> {
    if let Some(java_install_progress_sender) = &java_install_progress_sender {
        if let Err(err) = java_install_progress_sender.send(JavaInstallProgress::P1Started) {
            err!("Error sending java install progress: {err}\nThis should probably be safe to ignore");
        }
    }

    info!("Started installing {}", version.to_string());
    let java_list_json = JavaListJson::download().await?;
    let java_files_url = java_list_json
        .get_url(version)
        .ok_or(JavaInstallError::NoUrlForJavaFiles)?;

    let client = reqwest::Client::new();
    let json = file_utils::download_file_to_string(&client, &java_files_url, false).await?;
    let json: JavaFilesJson = serde_json::from_str(&json)?;

    let launcher_dir = file_utils::get_launcher_dir()?;

    let java_installs_dir = launcher_dir.join("java_installs");
    std::fs::create_dir_all(&java_installs_dir).map_err(io_err!(java_installs_dir.clone()))?;

    let install_dir = java_installs_dir.join(version.to_string());
    std::fs::create_dir_all(&install_dir).map_err(io_err!(java_installs_dir.clone()))?;

    let lock_file = install_dir.join("install.lock");
    std::fs::write(
        &lock_file,
        "If you see this, java hasn't finished installing.",
    )
    .map_err(io_err!(lock_file.clone()))?;

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

    std::fs::remove_file(&lock_file).map_err(io_err!(lock_file.clone()))?;

    info!("Finished installing {}", version.to_string());

    if let Some(java_install_progress_sender) = java_install_progress_sender {
        if let Err(err) = java_install_progress_sender.send(JavaInstallProgress::P3Done) {
            err!("Error sending java install progress: {err}\nThis should probably be safe to ignore");
        }
    }
    Ok(())
}

async fn java_install_fn(
    java_install_progress_sender: Option<&Sender<JavaInstallProgress>>,
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
            std::fs::write(&file_path, &file_bytes).map_err(io_err!(file_path.clone()))?;
            if *executable {
                file_utils::set_executable(&file_path)?;
            }
        }
        JavaFile::directory {} => {
            std::fs::create_dir_all(&file_path).map_err(io_err!(file_path))?;
        }
        JavaFile::link { target } => {
            // TODO: Deal with java install symlink.
            println!("[fixme:install_java] Deal with symlink {file_name} -> {target}");
        }
    }
    {
        let mut file_num = file_num.lock().unwrap();
        info!("Installing file ({file_num}/{num_files}): {file_name}");
        if let Some(java_install_progress_sender) = java_install_progress_sender {
            let _ = java_install_progress_sender.send(JavaInstallProgress::P2 {
                done: *file_num,
                out_of: num_files,
                name: file_name.to_owned(),
            });
        }
        *file_num += 1;
    }
    Ok(())
}

#[derive(Debug)]
pub enum JavaInstallError {
    JsonDownload(JsonDownloadError),
    Request(RequestError),
    NoUrlForJavaFiles,
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
        match self {
            JavaInstallError::JsonDownload(err) => write!(f, "{err}"),
            JavaInstallError::NoUrlForJavaFiles => write!(f, "could not find url to download java"),
            JavaInstallError::Request(err) => write!(f, "{err}"),
            JavaInstallError::Serde(err) => write!(f, "{err}"),
            JavaInstallError::Io(err) => write!(f, "{err}"),
        }
    }
}

impl Error for JavaInstallError {}
