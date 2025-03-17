use std::{
    fmt::Display,
    path::{Path, PathBuf},
    process::Command,
    sync::mpsc::Sender,
};

use ql_core::{
    file_utils, info,
    json::{optifine::JsonOptifine, VersionDetails},
    GenericProgress, IntoIoError, IoError, JsonFileError, Loader, Progress, RequestError,
    CLASSPATH_SEPARATOR,
};
use ql_java_handler::{get_java_binary, JavaInstallError, JavaVersion};
use thiserror::Error;

use super::change_instance_type;

// javac -cp OptiFine_1.21.1_HD_U_J1.jar OptifineInstaller.java -d .
// java -cp OptiFine_1.21.1_HD_U_J1.jar:. OptifineInstaller

#[derive(Default)]
pub enum OptifineInstallProgress {
    #[default]
    P1Start,
    P2CompilingHook,
    P3RunningHook,
    P4DownloadingLibraries {
        done: usize,
        total: usize,
    },
    P5Done,
}

impl Display for OptifineInstallProgress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OptifineInstallProgress::P1Start => write!(f, "Starting installation."),
            OptifineInstallProgress::P2CompilingHook => write!(f, "Compiling hook."),
            OptifineInstallProgress::P3RunningHook => write!(f, "Running hook."),
            OptifineInstallProgress::P4DownloadingLibraries { done, total } => {
                write!(f, "Downloading libraries ({done}/{total}).")
            }
            OptifineInstallProgress::P5Done => write!(f, "Done."),
        }
    }
}

impl Progress for OptifineInstallProgress {
    fn get_num(&self) -> f32 {
        match self {
            OptifineInstallProgress::P1Start => 0.0,
            OptifineInstallProgress::P2CompilingHook => 1.0,
            OptifineInstallProgress::P3RunningHook => 2.0,
            OptifineInstallProgress::P4DownloadingLibraries { done, total } => {
                2.0 + (*done as f32 / *total as f32)
            }
            OptifineInstallProgress::P5Done => 3.0,
        }
    }

    fn get_message(&self) -> Option<String> {
        Some(self.to_string())
    }

    fn total() -> f32 {
        3.0
    }
}

pub async fn install(
    instance_name: String,
    path_to_installer: PathBuf,
    progress_sender: Option<Sender<OptifineInstallProgress>>,
    java_progress_sender: Option<Sender<GenericProgress>>,
) -> Result<(), OptifineError> {
    if !path_to_installer.exists() || !path_to_installer.is_file() {
        return Err(OptifineError::InstallerDoesNotExist);
    }

    let progress_sender = progress_sender.as_ref();

    info!("Started installing OptiFine");
    send_progress(progress_sender, OptifineInstallProgress::P1Start);

    let instance_path = file_utils::get_launcher_dir()
        .await?
        .join("instances")
        .join(&instance_name);

    create_details_json(&instance_path).await?;

    let dot_minecraft_path = instance_path.join(".minecraft");

    let optifine_path = instance_path.join("optifine");
    tokio::fs::create_dir_all(&optifine_path)
        .await
        .path(&optifine_path)?;

    create_hook_java_file(&dot_minecraft_path, &optifine_path).await?;

    let new_installer_path = optifine_path.join("OptiFine.jar");
    tokio::fs::copy(&path_to_installer, &new_installer_path)
        .await
        .path(path_to_installer)?;

    info!("Compiling OptifineInstaller.java");
    send_progress(progress_sender, OptifineInstallProgress::P2CompilingHook);
    compile_hook(
        &new_installer_path,
        &optifine_path,
        java_progress_sender.as_ref(),
    )
    .await?;

    info!("Running OptifineInstaller.java");
    send_progress(progress_sender, OptifineInstallProgress::P3RunningHook);
    run_hook(&new_installer_path, &optifine_path).await?;

    download_libraries(&instance_name, &dot_minecraft_path, progress_sender).await?;
    change_instance_type(&instance_path, "OptiFine".to_owned()).await?;
    send_progress(progress_sender, OptifineInstallProgress::P5Done);
    info!("Finished installing OptiFine");

    Ok(())
}

fn send_progress(
    progress_sender: Option<&Sender<OptifineInstallProgress>>,
    prog: OptifineInstallProgress,
) {
    if let Some(progress) = progress_sender {
        _ = progress.send(prog);
    }
}

pub async fn uninstall(instance_name: String) -> Result<Loader, OptifineError> {
    let instance_path = file_utils::get_launcher_dir()
        .await?
        .join("instances")
        .join(&instance_name);

    let optifine_path = instance_path.join("optifine");

    tokio::fs::remove_dir_all(&optifine_path)
        .await
        .path(optifine_path)?;
    change_instance_type(&instance_path, "Vanilla".to_owned()).await?;

    let dot_minecraft_path = instance_path.join(".minecraft");
    let libraries_path = dot_minecraft_path.join("libraries");
    tokio::fs::remove_dir_all(&libraries_path)
        .await
        .path(libraries_path)?;

    let versions_path = dot_minecraft_path.join("versions");
    let mut entries = tokio::fs::read_dir(&versions_path)
        .await
        .path(versions_path)?;
    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        // Check if the entry is a directory and contains the keyword
        if !path.is_dir() {
            continue;
        }

        if let Some(Some(file_name)) = path.file_name().map(|n| n.to_str()) {
            if file_name.to_lowercase().contains("Opti") {
                tokio::fs::remove_dir_all(&path).await.path(path)?;
            }
        }
    }
    Ok(Loader::OptiFine)
}

async fn create_hook_java_file(
    dot_minecraft_path: &Path,
    optifine_path: &Path,
) -> Result<(), OptifineError> {
    let mc_path = dot_minecraft_path.to_str().unwrap().replace('\\', "\\\\");
    let hook = include_str!("../../../../assets/installers/OptifineInstaller.java")
        .replace("REPLACE_WITH_MC_PATH", &mc_path);
    let hook_path = optifine_path.join("OptifineInstaller.java");
    tokio::fs::write(&hook_path, hook).await.path(hook_path)?;
    Ok(())
}

async fn download_libraries(
    instance_name: &str,
    dot_minecraft_path: &Path,
    progress_sender: Option<&Sender<OptifineInstallProgress>>,
) -> Result<(), OptifineError> {
    let (optifine_json, _) = JsonOptifine::read(instance_name).await?;
    let libraries_path = dot_minecraft_path.join("libraries");

    let len = optifine_json.libraries.len();
    for (i, library) in optifine_json
        .libraries
        .iter()
        .filter_map(|lib| (!lib.name.starts_with("optifine")).then_some(&lib.name))
        .enumerate()
    {
        // l = com.mojang:netty:1.8.8
        // path = com/mojang/netty/1.8.8/netty-1.8.8.jar
        // url = https://libraries.minecraft.net/com/mojang/netty/1.8.8/netty-1.8.8.jar

        // Split in colon
        let parts: Vec<&str> = library.split(':').collect();

        info!("Downloading library ({i}/{len}): {}", library);

        let url_parent_path = format!("{}/{}/{}", parts[0].replace('.', "/"), parts[1], parts[2],);
        let url_final_part = format!("{url_parent_path}/{}-{}.jar", parts[1], parts[2],);

        let parent_path = libraries_path.join(&url_parent_path);
        tokio::fs::create_dir_all(&parent_path)
            .await
            .path(parent_path)?;

        let url = format!("https://libraries.minecraft.net/{url_final_part}");

        let jar_path = libraries_path.join(&url_final_part);

        if let Some(progress) = progress_sender {
            _ = progress.send(OptifineInstallProgress::P4DownloadingLibraries {
                done: i,
                total: len,
            });
        }

        if jar_path.exists() {
            continue;
        }
        let jar_bytes = file_utils::download_file_to_bytes(&url, false).await?;
        tokio::fs::write(&jar_path, jar_bytes)
            .await
            .path(jar_path)?;
    }
    Ok(())
}

async fn run_hook(new_installer_path: &Path, optifine_path: &Path) -> Result<(), OptifineError> {
    let java_path = get_java_binary(JavaVersion::Java21, "java", None).await?;
    let output = Command::new(&java_path)
        .args([
            "-cp",
            &format!(
                "{}{CLASSPATH_SEPARATOR}.",
                new_installer_path.to_str().unwrap()
            ),
            "OptifineInstaller",
        ])
        .current_dir(optifine_path)
        .output()
        .path(java_path)?;
    if !output.status.success() {
        return Err(OptifineError::JavaFail(
            String::from_utf8(output.stdout).unwrap(),
            String::from_utf8(output.stderr).unwrap(),
        ));
    }
    Ok(())
}

async fn compile_hook(
    new_installer_path: &Path,
    optifine_path: &Path,
    java_progress_sender: Option<&Sender<GenericProgress>>,
) -> Result<(), OptifineError> {
    let javac_path = get_java_binary(JavaVersion::Java21, "javac", java_progress_sender).await?;
    let output = Command::new(&javac_path)
        .args([
            "-cp",
            new_installer_path.to_str().unwrap(),
            "OptifineInstaller.java",
            "-d",
            ".",
        ])
        .current_dir(optifine_path)
        .output()
        .path(javac_path)?;
    if !output.status.success() {
        return Err(OptifineError::JavacFail(
            String::from_utf8(output.stdout).unwrap(),
            String::from_utf8(output.stderr).unwrap(),
        ));
    }
    Ok(())
}

async fn create_details_json(instance_path: &Path) -> Result<(), OptifineError> {
    let details_path = instance_path.join("details.json");
    let details = tokio::fs::read_to_string(&details_path)
        .await
        .path(&details_path)?;
    let details: VersionDetails = serde_json::from_str(&details)?;

    let new_details_path = instance_path
        .join(".minecraft/versions")
        .join(&details.id)
        .join(format!("{}.json", details.id));

    tokio::fs::copy(&details_path, &new_details_path)
        .await
        .path(details_path)?;

    Ok(())
}

#[derive(Debug, Error)]
pub enum OptifineError {
    #[error("could not install optifine: {0}")]
    Io(#[from] IoError),
    #[error("could not install optifine: {0}")]
    JavaInstall(#[from] JavaInstallError),
    #[error("optifine installer file does not exist")]
    InstallerDoesNotExist,
    #[error("could not compile optifine installer\n\nSTDOUT = {0}\n\nSTDERR = {1}")]
    JavacFail(String, String),
    #[error("could not run optifine installer\n\nSTDOUT = {0}\n\nSTDERR = {1}")]
    JavaFail(String, String),
    #[error("could not install optifine: {0}")]
    Request(#[from] RequestError),
    #[error("could not install optifine: json error: {0}")]
    Serde(#[from] serde_json::Error),
}

impl From<JsonFileError> for OptifineError {
    fn from(value: JsonFileError) -> Self {
        match value {
            JsonFileError::SerdeError(err) => err.into(),
            JsonFileError::Io(err) => err.into(),
        }
    }
}
