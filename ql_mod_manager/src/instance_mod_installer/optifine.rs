use std::{
    fmt::Display,
    path::{Path, PathBuf},
    process::Command,
};

use ql_instances::{
    error::IoError,
    file_utils::{self, RequestError},
    info, io_err,
    java_install::{self, JavaInstallError},
    json_structs::{
        json_instance_config::InstanceConfigJson, json_java_list::JavaVersion,
        json_optifine::JsonOptifine, json_version::VersionDetails, JsonFileError,
    },
};

const CLASSPATH_SEPARATOR: char = if cfg!(unix) { ':' } else { ';' };

// javac -cp OptiFine_1.21.1_HD_U_J1.jar Hook.java -d .
// java -cp OptiFine_1.21.1_HD_U_J1.jar:. Hook

pub async fn install_optifine_wrapped(
    instance_name: String,
    path_to_installer: PathBuf,
) -> Result<(), String> {
    install_optifine(&instance_name, &path_to_installer)
        .await
        .map_err(|err| err.to_string())
}

pub async fn install_optifine(
    instance_name: &str,
    path_to_installer: &Path,
) -> Result<(), OptifineError> {
    if !path_to_installer.exists() || !path_to_installer.is_file() {
        return Err(OptifineError::InstallerDoesNotExist);
    }

    info!("Started installing OptiFine");

    let instance_path = file_utils::get_launcher_dir()?
        .join("instances")
        .join(&instance_name);

    create_details_json(&instance_path)?;

    let dot_minecraft_path = instance_path.join(".minecraft");

    let hook = include_str!("../../../assets/Hook.java")
        .replace("REPLACE_WITH_MC_PATH", dot_minecraft_path.to_str().unwrap());

    let optifine_path = instance_path.join("optifine");
    tokio::fs::create_dir_all(&optifine_path)
        .await
        .map_err(io_err!(optifine_path))?;

    let hook_path = optifine_path.join("Hook.java");
    tokio::fs::write(&hook_path, hook)
        .await
        .map_err(io_err!(hook_path))?;

    let new_installer_path = optifine_path.join("OptiFine.jar");
    tokio::fs::copy(&path_to_installer, &new_installer_path)
        .await
        .map_err(io_err!(path_to_installer))?;

    info!("Compiling Hook.java");
    // TODO: Add java install progress.
    let javac_path = java_install::get_java_binary(JavaVersion::Java21, "javac", None).await?;

    let output = Command::new(&javac_path)
        .args([
            "-cp",
            new_installer_path.to_str().unwrap(),
            "Hook.java",
            "-d",
            ".",
        ])
        .current_dir(&optifine_path)
        .output()
        .map_err(io_err!(javac_path))?;

    if !output.status.success() {
        return Err(OptifineError::JavacFail(
            String::from_utf8(output.stdout).unwrap(),
            String::from_utf8(output.stderr).unwrap(),
        ));
    }

    info!("Running Hook.java");
    let java_path = java_install::get_java_binary(JavaVersion::Java21, "java", None).await?;

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
        .current_dir(&optifine_path)
        .output()
        .map_err(io_err!(java_path))?;

    if !output.status.success() {
        return Err(OptifineError::JavaFail(
            String::from_utf8(output.stdout).unwrap(),
            String::from_utf8(output.stderr).unwrap(),
        ));
    }

    let (optifine_json, _) = JsonOptifine::read(instance_name)?;

    let client = reqwest::Client::new();
    let libraries_path = dot_minecraft_path.join("libraries");
    for library in optifine_json
        .libraries
        .iter()
        .filter_map(|l| (!l.name.starts_with("optifine:")).then_some(&l.name))
    {
        // l = com.mojang:netty:1.8.8
        // path = com/mojang/netty/1.8.8/netty-1.8.8.jar
        // url = https://libraries.minecraft.net/com/mojang/netty/1.8.8/netty-1.8.8.jar

        // Split in colon
        let parts: Vec<&str> = library.split(':').collect();

        let url_parent_path = format!("{}/{}/{}", parts[0].replace('.', "/"), parts[1], parts[2],);
        let url_final_part = format!("{url_parent_path}/{}-{}.jar", parts[1], parts[2],);

        let parent_path = libraries_path.join(&url_parent_path);
        tokio::fs::create_dir_all(&parent_path)
            .await
            .map_err(io_err!(parent_path))?;

        let url = format!("https://libraries.minecraft.net/{url_final_part}");
        info!("Downloading library: {url}");

        let jar_path = libraries_path.join(&url_final_part);
        if jar_path.exists() {
            continue;
        }
        let jar_bytes = file_utils::download_file_to_bytes(&client, &url, false).await?;
        tokio::fs::write(&jar_path, jar_bytes)
            .await
            .map_err(io_err!(jar_path))?;
    }

    update_instance_config_json(&instance_path)?;
    info!("Finished installing OptiFine");

    Ok(())
}

fn update_instance_config_json(instance_path: &Path) -> Result<(), OptifineError> {
    let config_path = instance_path.join("config.json");
    let config = std::fs::read_to_string(&config_path).map_err(io_err!(config_path))?;
    let mut config: InstanceConfigJson = serde_json::from_str(&config)?;

    config.mod_type = "OptiFine".to_string();
    let config = serde_json::to_string(&config)?;
    std::fs::write(&config_path, config).map_err(io_err!(config_path))?;
    Ok(())
}

fn create_details_json(instance_path: &Path) -> Result<(), OptifineError> {
    let details_path = instance_path.join("details.json");
    let details = std::fs::read_to_string(&details_path).map_err(io_err!(details_path))?;
    let details: VersionDetails = serde_json::from_str(&details)?;

    let new_details_path = instance_path
        .join(".minecraft/versions")
        .join(&details.id)
        .join(format!("{}.json", details.id));

    std::fs::copy(&details_path, &new_details_path).map_err(io_err!(details_path))?;

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
