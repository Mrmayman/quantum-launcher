use std::{
    path::{Path, PathBuf},
    process::Stdio,
    sync::{mpsc::Sender, Arc, Mutex},
};

use ql_core::{
    err, file_utils, info,
    json::{InstanceConfigJson, VersionDetails},
    GenericProgress, IntoIoError,
};
use ql_java_handler::{get_java_binary, JavaVersion};
use tokio::process::{Child, Command};

use crate::ServerError;

/// [`run`] `_w` function
///
/// # Errors
/// See [`run`]
pub async fn run_w(
    name: String,
    java_install_progress: Sender<GenericProgress>,
) -> Result<(Arc<Mutex<Child>>, bool), String> {
    run(&name, java_install_progress)
        .await
        .map(|(n, b)| (Arc::new(Mutex::new(n)), b))
        .map_err(|n| n.to_string())
}

/// Runs a server.
///
/// # Arguments
/// - `name` - The name of the server to run.
/// - `java_install_progress` - The channel to send progress updates to
///   if Java needs to be installed.
///
/// # Returns
/// - `Ok((Child, bool))` - The child process and whether the server is a classic server.
/// - `Err(ServerError)` - The error that occurred.
///
/// # Errors
/// - Instance `config.json` couldn't be read or parsed
/// - Instance `details.json` couldn't be read or parsed
/// - Java binary path could not be obtained
/// - Java could not be installed (if not found)
/// - `Command` couldn't be spawned (IO Error)
/// - Forge shim file (`forge-*-shim.jar`) couldn't be found
/// - Other stuff I'm too dumb to see
/// ## Launcher Dir
/// - if config dir (~/.config on linux or AppData/Roaming on windows) is not found
/// - if you're on an unsupported platform (other than Windows, Linux, macOS, Redox, any linux-like unix)
/// - if the launcher directory could not be created (permissions issue)
pub async fn run(
    name: &str,
    java_install_progress: Sender<GenericProgress>,
) -> Result<(Child, bool), ServerError> {
    let launcher_dir = file_utils::get_launcher_dir().await?;
    let server_dir = launcher_dir.join("servers").join(name);

    let config_json = InstanceConfigJson::read(&server_dir).await?;

    let server_jar_path = if config_json.mod_type == "Fabric" || config_json.mod_type == "Quilt" {
        server_dir.join("fabric-server-launch.jar")
    } else if config_json.mod_type == "Forge" {
        find_forge_shim_file(&server_dir)
            .await
            .ok_or(ServerError::NoForgeShimFound)?
    } else if config_json.mod_type == "Paper" {
        server_dir.join("paper_server.jar")
    } else {
        server_dir.join("server.jar")
    };

    let java_path = get_java(&server_dir, &config_json, java_install_progress).await?;

    let mut java_args: Vec<String> = if let Some(java_args) = &config_json.java_args {
        java_args
            .iter()
            .filter(|n| !n.is_empty())
            .cloned()
            .collect()
    } else {
        Vec::new()
    };
    java_args.push(config_json.get_ram_argument());
    if config_json.mod_type == "Forge" {
        java_args.push("-Djava.net.preferIPv6Addresses=system".to_owned());
    }

    let is_classic_server = config_json.is_classic_server.unwrap_or(false);
    java_args.push(if is_classic_server { "-cp" } else { "-jar" }.to_owned());
    java_args.push(
        server_jar_path
            .to_str()
            .ok_or(ServerError::PathBufToStr(server_jar_path.clone()))?
            .to_owned(),
    );

    if is_classic_server {
        java_args.push("com.mojang.minecraft.server.MinecraftServer".to_owned());
    }

    let mut game_args = config_json.game_args.clone().unwrap_or_default();
    game_args.push("nogui".to_owned());

    info!("Java args: {java_args:?}\n");
    info!("Game args: {game_args:?}\n");

    let mut command = Command::new(java_path);
    let mut command = command.args(java_args.iter().chain(game_args.iter()));

    command = if config_json.enable_logger.unwrap_or(true) {
        command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::piped())
    } else {
        command
    }
    .current_dir(&server_dir);

    let child = command.spawn().path(server_jar_path)?;
    info!("Started server");
    Ok((child, is_classic_server))
}

async fn get_java(
    server_dir: &PathBuf,
    config_json: &InstanceConfigJson,
    java_install_progress: Sender<GenericProgress>,
) -> Result<PathBuf, ServerError> {
    let version_json = VersionDetails::load_from_path(server_dir).await?;
    let version = if let Some(version) = version_json.javaVersion.clone() {
        version.into()
    } else {
        JavaVersion::Java8
    };
    let java_path = get_java_path(config_json, version, java_install_progress).await?;
    Ok(java_path)
}

async fn get_java_path(
    config_json: &InstanceConfigJson,
    version: JavaVersion,
    java_install_progress: Sender<GenericProgress>,
) -> Result<PathBuf, ServerError> {
    if let Some(java_path) = &config_json.java_override {
        if !java_path.is_empty() {
            let java_path = PathBuf::from(java_path);
            if java_path.exists() {
                return Ok(java_path);
            }
            err!("Java override at {java_path:?} does not exist!");
        }
    };
    let path = get_java_binary(version, "java", Some(&java_install_progress)).await?;
    Ok(path)
}

async fn find_forge_shim_file(dir: &Path) -> Option<PathBuf> {
    if !dir.is_dir() {
        return None; // Ensure the path is a directory
    }

    let mut dir = tokio::fs::read_dir(dir).await.ok()?;

    while let Ok(Some(entry)) = dir.next_entry().await {
        let path = entry.path();

        if path.is_file() {
            if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
                if file_name.starts_with("forge-") && file_name.ends_with("-shim.jar") {
                    return Some(path);
                }
            }
        }
    }

    None // Return None if no matching file is found
}
