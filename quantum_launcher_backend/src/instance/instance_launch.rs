use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    process::{Child, Command},
    sync::{Arc, Mutex},
};

use crate::{
    error::{LauncherError, LauncherResult},
    file_utils::{self, create_dir_if_not_exists},
    java_locate::JavaInstall,
    json_structs::{
        json_fabric::FabricJSON,
        json_instance_config::InstanceConfigJson,
        json_version::{Library, VersionDetails},
    },
};

const CLASSPATH_SEPARATOR: char = if cfg!(unix) { ':' } else { ';' };

#[derive(Clone, Debug)]
pub enum GameLaunchResult {
    Ok(Arc<Mutex<Child>>),
    Err(String),
    LocateJavaManually {
        required_java_version: Option<usize>,
    },
}

pub async fn launch(
    instance_name: String,
    username: String,
    manually_added_java_versions: Vec<String>,
) -> GameLaunchResult {
    let manual_result = GameLaunchResult::LocateJavaManually {
        required_java_version: None,
    };

    match launch_blocking(&instance_name, &username, &manually_added_java_versions) {
        Ok(child) => GameLaunchResult::Ok(Arc::new(Mutex::new(child))),
        Err(LauncherError::JavaVersionConvertCmdOutputToStringError(_)) => manual_result,
        Err(LauncherError::JavaVersionImproperVersionPlacement(_)) => manual_result,
        Err(LauncherError::JavaVersionIsEmptyError) => manual_result,
        Err(LauncherError::JavaVersionParseToNumberError(_)) => manual_result,
        Err(LauncherError::RequiredJavaVersionNotFound(ver)) => {
            GameLaunchResult::LocateJavaManually {
                required_java_version: Some(ver),
            }
        }
        Err(err) => GameLaunchResult::Err(err.to_string()),
    }
}

pub fn launch_blocking(
    instance_name: &str,
    username: &str,
    manually_added_java_versions: &[String],
) -> LauncherResult<Child> {
    if username.contains(' ') || username.is_empty() {
        return Err(LauncherError::UsernameIsInvalid(username.to_owned()));
    }

    let instance_dir = get_instance_dir(instance_name)?;
    let minecraft_dir = instance_dir.join(".minecraft");
    file_utils::create_dir_if_not_exists(&minecraft_dir)
        .map_err(|err| LauncherError::IoError(err, minecraft_dir.clone()))?;

    let config_json = get_config(&instance_dir)?;

    let version_json = read_version_json(&instance_dir)?;

    let game_arguments = get_arguments(&version_json, username, minecraft_dir, &instance_dir)?;

    let mut java_arguments = vec![
        "-Xss1M".to_owned(),
        "-Dminecraft.launcher.brand=minecraft-launcher".to_owned(),
        "-Dminecraft.launcher.version=2.1.1349".to_owned(),
        "-Djava.library.path=1.20.4-natives".to_owned(),
        format!("-Xmx{}", config_json.get_ram_in_string()),
    ];

    let fabric_json = if config_json.mod_type == "Fabric" {
        if let Some(fabric_json) = get_fabric_json(&instance_dir, &version_json) {
            let fabric_json = fabric_json?;
            // Add all the special fabric arguments.
            fabric_json.arguments.jvm.iter().for_each(|n| {
                java_arguments.push(n.clone());
            });

            Some(fabric_json)
        } else {
            None
        }
    } else {
        None
    };

    if let Some(ref logging) = version_json.logging {
        let logging_path = instance_dir.join(format!("logging-{}", logging.client.file.id));
        let logging_path = logging_path
            .to_str()
            .ok_or(LauncherError::PathBufToString(logging_path.clone()))?;
        java_arguments.push(format!("-Dlog4j.configurationFile=\"{}\"", logging_path))
    }

    java_arguments.push("-cp".to_owned());
    java_arguments.push(get_class_path(&version_json, instance_dir)?);
    if let Some(ref fabric_json) = fabric_json {
        java_arguments.push(fabric_json.mainClass.clone());
    } else {
        java_arguments.push(version_json.mainClass.clone());
    }

    let mut command = if let Some(java_override) = config_json.java_override {
        Command::new(java_override)
    } else {
        let java_installations =
            JavaInstall::find_java_installs(Some(manually_added_java_versions))?;
        let appropriate_install = java_installations
            .iter()
            .find(|n| n.version == version_json.javaVersion.majorVersion)
            .or_else(|| {
                java_installations
                    .iter()
                    .find(|n| n.version >= version_json.javaVersion.majorVersion)
            })
            .ok_or(LauncherError::RequiredJavaVersionNotFound(
                version_json.javaVersion.majorVersion,
            ))?;

        appropriate_install.get_command()
    };

    let command = command.args(java_arguments.iter().chain(game_arguments.iter()));
    let result = command.spawn().map_err(LauncherError::CommandError)?;

    Ok(result)
}

fn get_fabric_json(
    instance_dir: &PathBuf,
    version_json: &VersionDetails,
) -> Option<Result<FabricJSON, LauncherError>> {
    find_fabric_directory(
        &instance_dir.join(".minecraft").join("versions"),
        &version_json.id,
    )
    .map(|dir| find_first_json(&dir))
    .flatten()
    .map(|json_path| {
        let fabric_json = std::fs::read_to_string(&json_path)
            .map_err(|err| LauncherError::IoError(err, json_path))?;
        let fabric_json: FabricJSON = serde_json::from_str(&fabric_json)?;
        Ok(fabric_json)
    })
}

fn get_config(instance_dir: &PathBuf) -> Result<InstanceConfigJson, LauncherError> {
    let config_file_path = instance_dir.join("config.json");
    let config_json = std::fs::read_to_string(&config_file_path)
        .map_err(|err| LauncherError::IoError(err, config_file_path))?;
    Ok(serde_json::from_str(&config_json)?)
}

fn find_first_json(dir: &Path) -> Option<PathBuf> {
    for entry in std::fs::read_dir(dir).ok()?.flatten() {
        let path = entry.path();
        if path.is_file() {
            if let Some(extension) = path.extension() {
                if extension == "json" {
                    return Some(path);
                }
            }
        }
    }
    None
}

fn find_fabric_directory(dir: &Path, exclude_dir: &str) -> Option<PathBuf> {
    for entry in std::fs::read_dir(dir).ok()?.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(name) = path.file_name() {
                if name == exclude_dir {
                    continue;
                }
                if !name.to_str()?.contains("fabric") {
                    continue;
                }
            }
            return Some(path);
        }
    }
    None
}

fn get_class_path(version_json: &VersionDetails, instance_dir: PathBuf) -> LauncherResult<String> {
    let mut class_path: String = "".to_owned();
    if cfg!(windows) {
        class_path.push('"');
    }

    version_json
        .libraries
        .iter()
        .filter_map(|n| {
            if let Library::Normal { downloads, .. } = n {
                Some(downloads)
            } else {
                None
            }
        })
        .map(|downloads| {
            let library_path = instance_dir
                .join("libraries")
                .join(&downloads.artifact.path);
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

    let mod_lib_path = instance_dir.join(".minecraft").join("libraries");
    if mod_lib_path.exists() {
        find_jar_files(&mod_lib_path)?
            .iter()
            .for_each(|library_path| {
                library_path.to_str().map(|jar_file| {
                    class_path.push_str(jar_file);
                    class_path.push(CLASSPATH_SEPARATOR);
                });
            })
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

fn find_jar_files(dir: &Path) -> LauncherResult<Vec<PathBuf>> {
    let mut jar_files = Vec::new();

    // Recursively iterate over the directory entries
    let entries =
        std::fs::read_dir(dir).map_err(|err| LauncherError::IoError(err, dir.to_owned()))?;

    for entry in entries {
        let entry = entry.map_err(|err| LauncherError::IoError(err, dir.to_owned()))?;
        let path = entry.path();

        if path.is_file() {
            // Check if the file has a .jar extension
            if let Some(extension) = path.extension() {
                if extension == "jar" {
                    jar_files.push(path);
                }
            }
        } else if path.is_dir() {
            // Recursively search directories
            jar_files.extend(find_jar_files(&path)?);
        }
    }

    Ok(jar_files)
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
    create_dir_if_not_exists(&launcher_dir)
        .map_err(|err| LauncherError::IoError(err, launcher_dir.clone()))?;

    let instances_dir = launcher_dir.join("instances");
    create_dir_if_not_exists(&instances_dir)
        .map_err(|err| LauncherError::IoError(err, instances_dir.clone()))?;

    let instance_dir = instances_dir.join(instance_name);
    if !instance_dir.exists() {
        return Err(LauncherError::InstanceNotFound);
    }
    Ok(instance_dir)
}

fn replace_var(string: &mut String, var: &str, value: &str) {
    *string = string.replace(&format!("${{{}}}", var), value);
}

fn read_version_json(instance_dir: &Path) -> LauncherResult<VersionDetails> {
    let file_path = instance_dir.join("details.json");
    let mut file =
        File::open(&file_path).map_err(|err| LauncherError::IoError(err, file_path.clone()))?;

    let mut version_json: String = Default::default();
    file.read_to_string(&mut version_json)
        .map_err(|err| LauncherError::IoError(err, file_path))?;

    let version_json = serde_json::from_str(&version_json)?;
    Ok(version_json)
}
