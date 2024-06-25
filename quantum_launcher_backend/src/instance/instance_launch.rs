use crate::{
    error::{LauncherError, LauncherResult},
    file_utils, io_err,
    json_structs::{
        json_fabric::FabricJSON,
        json_instance_config::InstanceConfigJson,
        json_version::{LibraryDownloads, VersionDetails},
        JsonFileError,
    },
};
use std::{
    path::{Path, PathBuf},
    process::{Child, Command},
    sync::{Arc, Mutex},
};

const CLASSPATH_SEPARATOR: char = if cfg!(unix) { ':' } else { ';' };

pub type GameLaunchResult = Result<Arc<Mutex<Child>>, String>;

pub async fn launch_async(instance_name: String, username: String) -> GameLaunchResult {
    match launch(&instance_name, &username) {
        Ok(child) => GameLaunchResult::Ok(Arc::new(Mutex::new(child))),
        Err(err) => GameLaunchResult::Err(err.to_string()),
    }
}

pub fn launch(instance_name: &str, username: &str) -> LauncherResult<Child> {
    if username.contains(' ') || username.is_empty() {
        return Err(LauncherError::UsernameIsInvalid(username.to_owned()));
    }

    let instance_dir = get_instance_dir(instance_name)?;
    let minecraft_dir = instance_dir.join(".minecraft");
    std::fs::create_dir_all(&minecraft_dir).map_err(io_err!(minecraft_dir))?;

    let config_json = get_config(&instance_dir)?;

    let version_json = read_version_json(&instance_dir)?;

    let game_arguments = get_arguments(&version_json, username, minecraft_dir, &instance_dir)?;

    let natives_path = instance_dir.join("libraries").join("natives");

    let mut java_arguments = vec![
        "-Xss1M".to_owned(),
        "-Dminecraft.launcher.brand=minecraft-launcher".to_owned(),
        "-Dminecraft.launcher.version=2.1.1349".to_owned(),
        format!(
            "-Djava.library.path={}",
            natives_path
                .to_str()
                .ok_or(LauncherError::PathBufToString(natives_path.clone()))?
        ),
        format!("-Xmx{}", config_json.get_ram_in_string()),
    ];

    let fabric_json = setup_fabric(&config_json, &instance_dir, &mut java_arguments)?;

    setup_logging(&version_json, &instance_dir, &mut java_arguments)?;
    setup_classpath_and_mainclass(
        &mut java_arguments,
        &version_json,
        instance_dir,
        fabric_json,
    )?;

    let mut command = if let Some(java_override) = config_json.java_override {
        Command::new(java_override)
    } else {
        todo!()
    };

    println!("[info] Java args: {java_arguments:?}\n\n[info] Game args: {game_arguments:?}\n");

    let command = command.args(java_arguments.iter().chain(game_arguments.iter()));
    let result = command.spawn().map_err(LauncherError::CommandError)?;

    Ok(result)
}

fn setup_fabric(
    config_json: &InstanceConfigJson,
    instance_dir: &Path,
    java_arguments: &mut Vec<String>,
) -> Result<Option<FabricJSON>, LauncherError> {
    let fabric_json = if config_json.mod_type == "Fabric" {
        Some(get_fabric_json(instance_dir)?)
    } else {
        None
    };
    if let Some(ref fabric_json) = fabric_json {
        fabric_json.arguments.jvm.iter().for_each(|n| {
            java_arguments.push(n.clone());
        });
    }
    Ok(fabric_json)
}

fn setup_classpath_and_mainclass(
    java_arguments: &mut Vec<String>,
    version_json: &VersionDetails,
    instance_dir: PathBuf,
    fabric_json: Option<FabricJSON>,
) -> Result<(), LauncherError> {
    java_arguments.push("-cp".to_owned());
    java_arguments.push(get_class_path(version_json, instance_dir, &fabric_json)?);
    java_arguments.push(if let Some(ref fabric_json) = fabric_json {
        fabric_json.mainClass.clone()
    } else {
        version_json.mainClass.clone()
    });
    Ok(())
}

fn setup_logging(
    version_json: &VersionDetails,
    instance_dir: &Path,
    java_arguments: &mut Vec<String>,
) -> Result<(), LauncherError> {
    if let Some(ref logging) = version_json.logging {
        let logging_path = instance_dir.join(format!("logging-{}", logging.client.file.id));
        let logging_path = logging_path
            .to_str()
            .ok_or(LauncherError::PathBufToString(logging_path.clone()))?;
        java_arguments.push(format!("-Dlog4j.configurationFile=\"{}\"", logging_path))
    }
    Ok(())
}

fn get_fabric_json(instance_dir: &Path) -> Result<FabricJSON, JsonFileError> {
    let json_path = instance_dir.join("fabric.json");
    let fabric_json = std::fs::read_to_string(&json_path).map_err(io_err!(json_path))?;
    Ok(serde_json::from_str(&fabric_json)?)
}

fn get_config(instance_dir: &Path) -> Result<InstanceConfigJson, JsonFileError> {
    let config_file_path = instance_dir.join("config.json");
    let config_json =
        std::fs::read_to_string(&config_file_path).map_err(io_err!(config_file_path))?;
    Ok(serde_json::from_str(&config_json)?)
}

fn get_class_path(
    version_json: &VersionDetails,
    instance_dir: PathBuf,
    fabric_json: &Option<FabricJSON>,
) -> LauncherResult<String> {
    let mut class_path: String = "".to_owned();
    if cfg!(windows) {
        class_path.push('"');
    }

    version_json
        .libraries
        .iter()
        .filter_map(|n| match n.downloads.as_ref() {
            Some(LibraryDownloads::Normal { artifact, .. }) => Some(artifact),
            _ => None,
        })
        .map(|artifact| {
            let library_path = instance_dir.join("libraries").join(&artifact.path);
            if library_path.exists() {
                let library_path = match library_path.to_str() {
                    Some(n) => n,
                    None => return Err(LauncherError::PathBufToString(library_path)),
                };
                class_path.push_str(library_path);
                class_path.push(CLASSPATH_SEPARATOR);
            }
            Ok(())
        })
        .find(|n| n.is_err())
        .unwrap_or(Ok(()))?;

    if let Some(ref fabric_json) = fabric_json {
        for library in fabric_json.libraries.iter() {
            let library_path = instance_dir.join("libraries").join(library.get_path());
            class_path.push_str(library_path.to_str().unwrap());
            class_path.push(CLASSPATH_SEPARATOR);
        }
    }

    let jar_path = instance_dir
        .join(".minecraft")
        .join("versions")
        .join(&version_json.id)
        .join(format!("{}.jar", version_json.id));
    let jar_path = jar_path
        .to_str()
        .ok_or(LauncherError::PathBufToString(jar_path.clone()))?;
    class_path.push_str(jar_path);

    if cfg!(windows) {
        class_path.push('"');
    }
    Ok(class_path)
}

fn get_arguments(
    version_json: &VersionDetails,
    username: &str,
    minecraft_dir: PathBuf,
    instance_dir: &Path,
) -> LauncherResult<Vec<String>> {
    let mut game_arguments: Vec<String> =
        if let Some(ref arguments) = version_json.minecraftArguments {
            arguments.split(' ').map(ToOwned::to_owned).collect()
        } else if let Some(ref arguments) = version_json.arguments {
            arguments
                .game
                .iter()
                .filter_map(|arg| arg.as_str())
                .map(ToOwned::to_owned)
                .collect()
        } else {
            return Err(LauncherError::VersionJsonNoArgumentsField(
                version_json.clone(),
            ));
        };
    for argument in game_arguments.iter_mut() {
        replace_var(argument, "auth_player_name", username);
        replace_var(argument, "version_name", &version_json.id);
        let minecraft_dir_path = match minecraft_dir.to_str() {
            Some(n) => n,
            None => return Err(LauncherError::PathBufToString(minecraft_dir)),
        };
        replace_var(argument, "game_directory", minecraft_dir_path);

        let assets_path = instance_dir.join("assets");
        let assets_path = match assets_path.to_str() {
            Some(n) => n,
            None => return Err(LauncherError::PathBufToString(assets_path)),
        };
        replace_var(argument, "assets_root", assets_path);
        replace_var(argument, "game_assets", assets_path);
        replace_var(argument, "auth_xuid", "0");
        replace_var(
            argument,
            "auth_uuid",
            "00000000-0000-0000-0000-000000000000",
        );
        replace_var(argument, "auth_access_token", "0");
        replace_var(argument, "clientid", "0");
        replace_var(argument, "user_type", "legacy");
        replace_var(argument, "version_type", "release");
        replace_var(argument, "assets_index_name", &version_json.assetIndex.id);
    }
    Ok(game_arguments)
}

fn get_instance_dir(instance_name: &str) -> LauncherResult<PathBuf> {
    if instance_name.is_empty() {
        return Err(LauncherError::InstanceNotFound);
    }

    let launcher_dir = file_utils::get_launcher_dir()?;
    std::fs::create_dir_all(&launcher_dir).map_err(io_err!(launcher_dir))?;

    let instances_dir = launcher_dir.join("instances");
    std::fs::create_dir_all(&instances_dir).map_err(io_err!(instances_dir))?;

    let instance_dir = instances_dir.join(instance_name);
    if !instance_dir.exists() {
        return Err(LauncherError::InstanceNotFound);
    }
    Ok(instance_dir)
}

fn replace_var(string: &mut String, var: &str, value: &str) {
    *string = string.replace(&format!("${{{}}}", var), value);
}

fn read_version_json(instance_dir: &Path) -> Result<VersionDetails, JsonFileError> {
    let file_path = instance_dir.join("details.json");

    let version_json: String = std::fs::read_to_string(&file_path).map_err(io_err!(file_path))?;
    let version_json = serde_json::from_str(&version_json)?;
    Ok(version_json)
}
