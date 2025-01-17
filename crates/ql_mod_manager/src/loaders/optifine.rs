use std::{
    fmt::Display,
    path::{Path, PathBuf},
    process::Command,
    sync::mpsc::Sender,
};

use ql_core::{
    file_utils, get_java_binary, info,
    json::{
        instance_config::InstanceConfigJson, java_list::JavaVersion, optifine::JsonOptifine,
        version::VersionDetails,
    },
    GenericProgress, IntoIoError, IoError, JavaInstallError, JsonFileError, Progress, RequestError,
};

use crate::mod_manager::Loader;

const CLASSPATH_SEPARATOR: char = if cfg!(unix) { ':' } else { ';' };

// javac -cp OptiFine_1.21.1_HD_U_J1.jar Hook.java -d .
// java -cp OptiFine_1.21.1_HD_U_J1.jar:. Hook

pub async fn install_optifine_w(
    instance_name: String,
    path_to_installer: PathBuf,
    progress_sender: Option<Sender<OptifineInstallProgress>>,
    java_progress_sender: Option<Sender<GenericProgress>>,
) -> Result<(), String> {
    install_optifine(
        &instance_name,
        &path_to_installer,
        progress_sender,
        java_progress_sender,
    )
    .await
    .map_err(|err| err.to_string())
}

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

pub async fn install_optifine(
    instance_name: &str,
    path_to_installer: &Path,
    progress_sender: Option<Sender<OptifineInstallProgress>>,
    java_progress_sender: Option<Sender<GenericProgress>>,
) -> Result<(), OptifineError> {
    if !path_to_installer.exists() || !path_to_installer.is_file() {
        return Err(OptifineError::InstallerDoesNotExist);
    }

    info!("Started installing OptiFine");
    if let Some(progress) = &progress_sender {
        progress.send(OptifineInstallProgress::P1Start).unwrap();
    }

    let instance_path = file_utils::get_launcher_dir()?
        .join("instances")
        .join(instance_name);

    create_details_json(&instance_path)?;

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
    if let Some(progress) = &progress_sender {
        progress
            .send(OptifineInstallProgress::P2CompilingHook)
            .unwrap();
    }
    compile_hook(&new_installer_path, &optifine_path, java_progress_sender).await?;

    info!("Running OptifineInstaller.java");
    if let Some(progress) = &progress_sender {
        progress
            .send(OptifineInstallProgress::P3RunningHook)
            .unwrap();
    }
    run_hook(&new_installer_path, &optifine_path).await?;

    download_libraries(instance_name, &dot_minecraft_path, progress_sender.as_ref()).await?;
    update_instance_config_json(&instance_path, "OptiFine".to_owned())?;
    if let Some(progress) = &progress_sender {
        progress.send(OptifineInstallProgress::P5Done).unwrap();
    }
    info!("Finished installing OptiFine");

    Ok(())
}

pub async fn uninstall_w(instance_name: String) -> Result<Loader, String> {
    uninstall(&instance_name)
        .await
        .map_err(|err| err.to_string())
        .map(|()| Loader::OptiFine)
}

pub async fn uninstall(instance_name: &str) -> Result<(), OptifineError> {
    let instance_path = file_utils::get_launcher_dir()?
        .join("instances")
        .join(instance_name);

    let optifine_path = instance_path.join("optifine");

    tokio::fs::remove_dir_all(&optifine_path)
        .await
        .path(optifine_path)?;
    update_instance_config_json(&instance_path, "Vanilla".to_owned())?;

    let dot_minecraft_path = instance_path.join(".minecraft");
    let libraries_path = dot_minecraft_path.join("libraries");
    tokio::fs::remove_dir_all(&libraries_path)
        .await
        .path(libraries_path)?;

    let versions_path = dot_minecraft_path.join("versions");
    let entries = std::fs::read_dir(&versions_path).path(versions_path)?;
    for entry in entries.into_iter().filter_map(Result::ok) {
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
    Ok(())
}

async fn create_hook_java_file(
    dot_minecraft_path: &Path,
    optifine_path: &Path,
) -> Result<(), OptifineError> {
    let mc_path = dot_minecraft_path.to_str().unwrap().replace('\\', "\\\\");
    let hook = include_str!("../../../../assets/installers/OptifineInstaller.java")
        .replace("REPLACE_WITH_MC_PATH", &mc_path);
    let hook_path = optifine_path.join("Hook.java");
    tokio::fs::write(&hook_path, hook).await.path(hook_path)?;
    Ok(())
}

async fn download_libraries(
    instance_name: &str,
    dot_minecraft_path: &Path,
    progress_sender: Option<&Sender<OptifineInstallProgress>>,
) -> Result<(), OptifineError> {
    let (optifine_json, _) = JsonOptifine::read(instance_name)?;
    let client = reqwest::Client::new();
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
            progress
                .send(OptifineInstallProgress::P4DownloadingLibraries {
                    done: i,
                    total: len,
                })
                .unwrap();
        }

        if jar_path.exists() {
            continue;
        }
        let jar_bytes = file_utils::download_file_to_bytes(&client, &url, false).await?;
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
                "{}{}.",
                new_installer_path.to_str().unwrap(),
                CLASSPATH_SEPARATOR
            ),
            "Hook",
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
    java_progress_sender: Option<Sender<GenericProgress>>,
) -> Result<(), OptifineError> {
    let javac_path = get_java_binary(JavaVersion::Java21, "javac", java_progress_sender).await?;
    let output = Command::new(&javac_path)
        .args([
            "-cp",
            new_installer_path.to_str().unwrap(),
            "Hook.java",
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

fn update_instance_config_json(
    instance_path: &Path,
    mod_type: String,
) -> Result<(), OptifineError> {
    let config_path = instance_path.join("config.json");
    let config = std::fs::read_to_string(&config_path).path(&config_path)?;
    let mut config: InstanceConfigJson = serde_json::from_str(&config)?;

    config.mod_type = mod_type;
    let config = serde_json::to_string(&config)?;
    std::fs::write(&config_path, config).path(config_path)?;
    Ok(())
}

fn create_details_json(instance_path: &Path) -> Result<(), OptifineError> {
    let details_path = instance_path.join("details.json");
    let details = std::fs::read_to_string(&details_path).path(&details_path)?;
    let details: VersionDetails = serde_json::from_str(&details)?;

    let new_details_path = instance_path
        .join(".minecraft/versions")
        .join(&details.id)
        .join(format!("{}.json", details.id));

    std::fs::copy(&details_path, &new_details_path).path(details_path)?;

    Ok(())
}

pub enum OptifineError {
    Io(IoError),
    JavaInstall(JavaInstallError),
    InstallerDoesNotExist,
    JavacFail(String, String),
    JavaFail(String, String),
    Request(RequestError),
    Serde(serde_json::Error),
}

impl Display for OptifineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "optifine install error: ")?;
        match self {
            OptifineError::Io(err) => write!(f, "(io) {err}"),
            OptifineError::JavaInstall(err) => write!(f, "(java install) {err}"),
            OptifineError::InstallerDoesNotExist => write!(f, "installer file does not exist."),
            OptifineError::JavacFail(out, err) => {
                write!(f, "java compiler error.\nSTDOUT: {out}\nSTDERR: {err}")
            }
            OptifineError::JavaFail(out, err) => {
                write!(f, "java runtime error.\nSTDOUT: {out}\nSTDERR: {err}")
            }
            OptifineError::Serde(err) => write!(f, "(json) {err}"),
            OptifineError::Request(err) => write!(f, "(request) {err}"),
        }
    }
}

impl From<IoError> for OptifineError {
    fn from(value: IoError) -> Self {
        Self::Io(value)
    }
}

impl From<JavaInstallError> for OptifineError {
    fn from(value: JavaInstallError) -> Self {
        Self::JavaInstall(value)
    }
}

impl From<serde_json::Error> for OptifineError {
    fn from(value: serde_json::Error) -> Self {
        Self::Serde(value)
    }
}

impl From<RequestError> for OptifineError {
    fn from(value: RequestError) -> Self {
        Self::Request(value)
    }
}

impl From<JsonFileError> for OptifineError {
    fn from(value: JsonFileError) -> Self {
        match value {
            JsonFileError::SerdeError(err) => err.into(),
            JsonFileError::Io(err) => err.into(),
        }
    }
}
