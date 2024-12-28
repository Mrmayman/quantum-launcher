use std::{
    path::{Path, PathBuf},
    process::Stdio,
    sync::{mpsc::Sender, Arc, Mutex},
};

use ql_core::{
    err, file_utils, get_java_binary, info, io_err,
    json::{instance_config::InstanceConfigJson, java_list::JavaVersion, version::VersionDetails},
    JavaInstallProgress,
};
use tokio::process::{Child, Command};

use crate::ServerError;

pub async fn run_w(
    name: String,
    java_install_progress: Sender<JavaInstallProgress>,
) -> Result<(Arc<Mutex<Child>>, bool), String> {
    run(&name, java_install_progress)
        .await
        .map(|(n, b)| (Arc::new(Mutex::new(n)), b))
        .map_err(|n| n.to_string())
}

async fn run(
    name: &str,
    java_install_progress: Sender<JavaInstallProgress>,
) -> Result<(Child, bool), ServerError> {
    let launcher_dir = file_utils::get_launcher_dir()?;
    let server_dir = launcher_dir.join("servers").join(name);

    let config_json = get_config_json(&server_dir).await?;

    let server_jar_path = if config_json.mod_type == "Fabric" {
        server_dir.join("fabric-server-launch.jar")
    } else if config_json.mod_type == "Forge" {
        find_forge_shim_file(&server_dir).ok_or(ServerError::NoForgeShimFound)?
    } else {
        server_dir.join("server.jar")
    };

    let version_json_path = server_dir.join("details.json");
    let version_json = tokio::fs::read_to_string(&version_json_path)
        .await
        .map_err(io_err!(version_json_path))?;
    let version_json: VersionDetails = serde_json::from_str(&version_json)?;

    let version = if let Some(version) = version_json.javaVersion.clone() {
        version.into()
    } else {
        JavaVersion::Java8
    };

    let java_path = get_java_path(&config_json, version, java_install_progress).await?;

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
    java_args.push(server_jar_path.to_str().unwrap().to_owned());

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

    let child = command.spawn().map_err(io_err!(server_jar_path))?;
    info!("Started server");
    Ok((child, is_classic_server))
}

async fn get_config_json(server_dir: &Path) -> Result<InstanceConfigJson, ServerError> {
    let config_json_path = server_dir.join("config.json");
    let config_json = tokio::fs::read_to_string(&config_json_path)
        .await
        .map_err(io_err!(config_json_path))?;
    let config_json: InstanceConfigJson = serde_json::from_str(&config_json)?;
    Ok(config_json)
}

async fn get_java_path(
    config_json: &InstanceConfigJson,
    version: JavaVersion,
    java_install_progress: Sender<JavaInstallProgress>,
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
    let path = get_java_binary(version, "java", Some(java_install_progress)).await?;
    Ok(path)
}

fn find_forge_shim_file(dir: &Path) -> Option<PathBuf> {
    if !dir.is_dir() {
        return None; // Ensure the path is a directory
    }

    for entry in (std::fs::read_dir(dir).ok()?).flatten() {
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
