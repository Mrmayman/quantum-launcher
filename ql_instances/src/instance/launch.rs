use tokio::process::{Child, Command};

use crate::{
    download::GameDownloader,
    error::{IoError, LauncherError, LauncherResult},
    file_utils, info,
    instance::migrate::migrate_old_instances,
    io_err,
    java_install::{self, JavaInstallProgress},
    json_structs::{
        json_fabric::FabricJSON,
        json_forge::JsonForgeDetails,
        json_instance_config::InstanceConfigJson,
        json_java_list::JavaVersion,
        json_version::{LibraryDownloads, VersionDetails},
        JsonFileError,
    },
};
use std::{
    path::{Path, PathBuf},
    process::Stdio,
    sync::{mpsc::Sender, Arc, Mutex},
};

const CLASSPATH_SEPARATOR: char = if cfg!(unix) { ':' } else { ';' };

pub type GameLaunchResult = Result<Arc<Mutex<Child>>, String>;

/// Wraps the [`launch`] function to give a `Result<Arc<Mutex<Child>>, String`
/// instead of a `Result<Child, LauncherError>` to make it easier to
/// use with the iced GUI toolkit.
///
/// Launches the specified instance with the specified username.
/// Will error if instance isn't created.
///
/// This auto downloads the required version of Java
/// if it's not already installed.
///
/// If you want, you can hook this up to a progress bar
/// (since installing Java takes a while), by using a
/// `std::sync::mpsc::channel::<JavaInstallMessage>()`, giving the
/// sender to this function and polling the receiver frequently.
/// If not needed, simply pass `None` to the function.
pub async fn launch_wrapped(
    instance_name: String,
    username: String,
    java_install_progress_sender: Option<Sender<JavaInstallProgress>>,
    enable_logger: bool,
) -> GameLaunchResult {
    match launch(
        &instance_name,
        &username,
        java_install_progress_sender,
        enable_logger,
    )
    .await
    {
        Ok(child) => GameLaunchResult::Ok(Arc::new(Mutex::new(child))),
        Err(err) => GameLaunchResult::Err(err.to_string()),
    }
}

/// Launches the specified instance with the specified username.
/// Will error if instance isn't created.
///
/// This auto downloads the required version of Java
/// if it's not already installed.
///
/// If you want, you can hook this up to a progress bar
/// (since installing Java takes a while), by using a
/// `std::sync::mpsc::channel::<JavaInstallMessage>()`, giving the
/// sender to this function and polling the receiver frequently.
/// If not needed, simply pass `None` to the function.
pub async fn launch(
    instance_name: &str,
    username: &str,
    java_install_progress_sender: Option<Sender<JavaInstallProgress>>,
    enable_logger: bool,
) -> LauncherResult<Child> {
    if username.contains(' ') || username.is_empty() {
        return Err(LauncherError::UsernameIsInvalid(username.to_owned()));
    }

    let instance_dir = get_instance_dir(instance_name)?;

    migrate_old_instances(&instance_dir).await?;

    let minecraft_dir = instance_dir.join(".minecraft");
    std::fs::create_dir_all(&minecraft_dir).map_err(io_err!(minecraft_dir))?;

    let mods_dir = minecraft_dir.join("mods");
    std::fs::create_dir_all(&mods_dir).map_err(io_err!(mods_dir))?;

    let config_json = get_config(&instance_dir)?;

    let version_json = read_version_json(&instance_dir)?;

    let mut game_arguments =
        get_arguments(&version_json, username, &minecraft_dir, &instance_dir).await?;

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
        config_json.get_ram_argument(),
    ];

    if version_json.r#type == "old_beta" || version_json.r#type == "old_alpha" {
        java_arguments.push("-Dhttp.proxyHost=betacraft.uk".to_owned());
    }

    let fabric_json = setup_fabric(&config_json, &instance_dir, &mut java_arguments)?;
    let forge_json = setup_forge(
        &config_json,
        &instance_dir,
        &mut java_arguments,
        &mut game_arguments,
        username,
        &version_json,
        &minecraft_dir,
    )
    .await?;

    for argument in &mut java_arguments {
        replace_var(
            argument,
            "classpath_separator",
            &CLASSPATH_SEPARATOR.to_string(),
        );
        let library_directory = instance_dir.join("forge/libraries");
        replace_var(
            argument,
            "library_directory",
            library_directory
                .to_str()
                .ok_or(LauncherError::PathBufToString(library_directory.clone()))?,
        );
        replace_var(argument, "version_name", &version_json.id);
    }

    setup_logging(&version_json, &instance_dir, &mut java_arguments)?;
    setup_classpath_and_mainclass(
        &mut java_arguments,
        &version_json,
        &instance_dir,
        fabric_json,
        forge_json,
    )?;

    let mut command = if let Some(java_override) = config_json.java_override {
        Command::new(java_override)
    } else {
        let version = if let Some(version) = version_json.javaVersion {
            version.into()
        } else {
            JavaVersion::Java8
        };
        Command::new(
            java_install::get_java_binary(version, "java", java_install_progress_sender).await?,
        )
    };

    info!("Java args: {java_arguments:?}\n");
    info!("Game args: {game_arguments:?}\n");

    let mut command = command.args(java_arguments.iter().chain(game_arguments.iter()));
    command = if enable_logger {
        command.stdout(Stdio::piped()).stderr(Stdio::piped())
    } else {
        command
    }
    .current_dir(&minecraft_dir);

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

async fn setup_forge(
    config_json: &InstanceConfigJson,
    instance_dir: &Path,
    java_arguments: &mut Vec<String>,
    game_arguments: &mut Vec<String>,
    username: &str,
    version_json: &VersionDetails,
    minecraft_dir: &Path,
) -> Result<Option<JsonForgeDetails>, LauncherError> {
    let json = if config_json.mod_type == "Forge" {
        Some(get_forge_json(instance_dir)?)
    } else {
        None
    };
    if let Some(json) = &json {
        if let Some(arguments) = &json.arguments {
            if let Some(jvm) = &arguments.jvm {
                for arg in jvm {
                    java_arguments.push(arg.clone());
                }
            }
            arguments.game.iter().for_each(|n| {
                game_arguments.push(n.clone());
            });
        } else if let Some(arguments) = &json.minecraftArguments {
            *game_arguments = arguments.split(' ').map(str::to_owned).collect();
            fill_game_arguments(
                game_arguments,
                username,
                version_json,
                minecraft_dir,
                instance_dir,
            )
            .await?;
        }
    }
    Ok(json)
}

fn setup_classpath_and_mainclass(
    java_arguments: &mut Vec<String>,
    version_json: &VersionDetails,
    instance_dir: &Path,
    fabric_json: Option<FabricJSON>,
    forge_json: Option<JsonForgeDetails>,
) -> Result<(), LauncherError> {
    java_arguments.push("-cp".to_owned());
    java_arguments.push(get_class_path(
        version_json,
        instance_dir,
        fabric_json.as_ref(),
        forge_json.as_ref(),
    )?);
    java_arguments.push(if let Some(fabric_json) = fabric_json {
        fabric_json.mainClass
    } else if let Some(forge_json) = forge_json {
        forge_json.mainClass.clone()
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
        java_arguments.push(format!("-Dlog4j.configurationFile={logging_path}"));
    }
    Ok(())
}

fn get_fabric_json(instance_dir: &Path) -> Result<FabricJSON, JsonFileError> {
    let json_path = instance_dir.join("fabric.json");
    let fabric_json = std::fs::read_to_string(&json_path).map_err(io_err!(json_path))?;
    Ok(serde_json::from_str(&fabric_json)?)
}

fn get_forge_json(instance_dir: &Path) -> Result<JsonForgeDetails, JsonFileError> {
    let json_path = instance_dir.join("forge/details.json");
    let json = std::fs::read_to_string(&json_path).map_err(io_err!(json_path))?;
    Ok(serde_json::from_str(&json)?)
}

fn get_config(instance_dir: &Path) -> Result<InstanceConfigJson, JsonFileError> {
    let config_file_path = instance_dir.join("config.json");
    let config_json =
        std::fs::read_to_string(&config_file_path).map_err(io_err!(config_file_path))?;
    Ok(serde_json::from_str(&config_json)?)
}

fn get_class_path(
    version_json: &VersionDetails,
    instance_dir: &Path,
    fabric_json: Option<&FabricJSON>,
    forge_json: Option<&JsonForgeDetails>,
) -> LauncherResult<String> {
    let mut class_path = String::new();

    if forge_json.is_some() {
        let classpath_path = instance_dir.join("forge/classpath.txt");
        let forge_classpath =
            std::fs::read_to_string(&classpath_path).map_err(io_err!(classpath_path))?;
        class_path.push_str(&forge_classpath);
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
                let Some(library_path) = library_path.to_str() else {
                    return Err(LauncherError::PathBufToString(library_path));
                };
                class_path.push_str(library_path);
                class_path.push(CLASSPATH_SEPARATOR);
            }
            Ok(())
        })
        .find(std::result::Result::is_err)
        .unwrap_or(Ok(()))?;

    if let Some(fabric_json) = fabric_json {
        for library in &fabric_json.libraries {
            let library_path = instance_dir.join("libraries").join(library.get_path());
            class_path.push_str(
                library_path
                    .to_str()
                    .ok_or(LauncherError::PathBufToString(library_path.clone()))?,
            );
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

    Ok(class_path)
}

async fn get_arguments(
    version_json: &VersionDetails,
    username: &str,
    minecraft_dir: &Path,
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
            return Err(LauncherError::VersionJsonNoArgumentsField(Box::new(
                version_json.clone(),
            )));
        };
    fill_game_arguments(
        &mut game_arguments,
        username,
        version_json,
        minecraft_dir,
        instance_dir,
    )
    .await?;
    Ok(game_arguments)
}

async fn fill_game_arguments(
    game_arguments: &mut [String],
    username: &str,
    version_json: &VersionDetails,
    minecraft_dir: &Path,
    instance_dir: &Path,
) -> Result<(), LauncherError> {
    for argument in game_arguments.iter_mut() {
        replace_var(argument, "auth_player_name", username);
        replace_var(argument, "version_name", &version_json.id);
        let Some(minecraft_dir_path) = minecraft_dir.to_str() else {
            return Err(LauncherError::PathBufToString(minecraft_dir.to_owned()));
        };
        replace_var(argument, "game_directory", minecraft_dir_path);

        set_assets_argument(version_json, instance_dir, argument).await?;
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
        replace_var(argument, "user_properties", "{}");
    }
    Ok(())
}

async fn set_assets_argument(
    version_json: &VersionDetails,
    instance_dir: &Path,
    argument: &mut String,
) -> Result<(), LauncherError> {
    let old_assets_path_v2 = file_utils::get_launcher_dir()?
        .join("assets")
        .join(&version_json.assetIndex.id);

    let old_assets_path_v1 = instance_dir.join("assets");

    if version_json.assetIndex.id == "legacy" {
        let assets_path = file_utils::get_launcher_dir()?.join("assets/legacy_assets");

        if old_assets_path_v2.exists() {
            redownload_legacy_assets(version_json, instance_dir).await?;
            std::fs::remove_dir_all(&old_assets_path_v2).map_err(io_err!(old_assets_path_v2))?;
        }

        if old_assets_path_v1.exists() {
            redownload_legacy_assets(version_json, instance_dir).await?;
            std::fs::remove_dir_all(&old_assets_path_v1).map_err(io_err!(old_assets_path_v1))?;
        }

        let assets_path_fixed = if assets_path.exists() {
            assets_path
        } else {
            file_utils::get_launcher_dir()?.join("assets/null")
        };

        let Some(assets_path) = assets_path_fixed.to_str() else {
            return Err(LauncherError::PathBufToString(assets_path_fixed));
        };
        replace_var(argument, "assets_root", assets_path);
        replace_var(argument, "game_assets", assets_path);
    } else {
        let assets_path = file_utils::get_launcher_dir()?.join("assets/dir");

        if old_assets_path_v2.exists() {
            info!("Migrating old assets to new path...");
            copy_dir_recursive(&old_assets_path_v2, &assets_path)?;
            std::fs::remove_dir_all(&old_assets_path_v2).map_err(io_err!(old_assets_path_v2))?;
        }

        if old_assets_path_v1.exists() {
            migrate_to_new_assets_path(&old_assets_path_v1, &assets_path)?;
        }

        let assets_path_fixed = if assets_path.exists() {
            assets_path
        } else {
            file_utils::get_launcher_dir()?.join("assets/null")
        };
        let Some(assets_path) = assets_path_fixed.to_str() else {
            return Err(LauncherError::PathBufToString(assets_path_fixed));
        };
        replace_var(argument, "assets_root", assets_path);
        replace_var(argument, "game_assets", assets_path);
    }
    Ok(())
}

async fn redownload_legacy_assets(
    version_json: &VersionDetails,
    instance_dir: &Path,
) -> Result<(), LauncherError> {
    info!("Redownloading legacy assets");
    let game_downloader =
        GameDownloader::with_existing_instance(version_json.clone(), instance_dir.to_owned());
    game_downloader.download_assets().await?;
    Ok(())
}

/// Moves the game assets from the old path:
///
/// `QuantumLauncher/instances/INSTANCE_NAME/assets/`
///
/// to the usual one:
///
/// `QuantumLauncher/assets/ASSETS_NAME/`
///
/// Old versions of the launcher put the assets at the
/// old path. This migrates it to the new path.
///
/// This applies to early development builds of the
/// launcher (before v0.1), most people won't ever
/// need to run this aside from the early beta testers.
fn migrate_to_new_assets_path(
    old_assets_path: &Path,
    assets_path: &Path,
) -> Result<(), LauncherError> {
    info!("Migrating old assets to new path...");
    copy_dir_recursive(old_assets_path, assets_path)?;
    std::fs::remove_dir_all(old_assets_path).map_err(io_err!(old_assets_path))?;
    info!("Finished");
    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), IoError> {
    // Create the destination directory if it doesn't exist
    if !dst.exists() {
        std::fs::create_dir_all(dst).map_err(io_err!(dst))?;
    }

    // Iterate over the directory entries
    for entry in std::fs::read_dir(src).map_err(io_err!(src))? {
        let entry = entry.map_err(io_err!(src))?;
        let path = entry.path();
        let dest_path = dst.join(entry.file_name());

        if path.is_dir() {
            // Recursively copy the subdirectory
            copy_dir_recursive(&path, &dest_path)?;
        } else {
            // Copy the file to the destination directory
            std::fs::copy(&path, &dest_path).map_err(io_err!(path))?;
        }
    }

    Ok(())
}

fn get_instance_dir(instance_name: &str) -> LauncherResult<PathBuf> {
    if instance_name.is_empty() {
        return Err(LauncherError::InstanceNotFound);
    }

    let launcher_dir = file_utils::get_launcher_dir()?;
    std::fs::create_dir_all(&launcher_dir).map_err(io_err!(launcher_dir))?;

    let instances_folder_dir = launcher_dir.join("instances");
    std::fs::create_dir_all(&instances_folder_dir).map_err(io_err!(instances_folder_dir))?;

    let instance_dir = instances_folder_dir.join(instance_name);
    if !instance_dir.exists() {
        return Err(LauncherError::InstanceNotFound);
    }
    Ok(instance_dir)
}

fn replace_var(string: &mut String, var: &str, value: &str) {
    *string = string.replace(&format!("${{{var}}}"), value);
}

fn read_version_json(instance_dir: &Path) -> Result<VersionDetails, JsonFileError> {
    let file_path = instance_dir.join("details.json");

    let version_json: String = std::fs::read_to_string(&file_path).map_err(io_err!(file_path))?;
    let version_json = serde_json::from_str(&version_json)?;
    Ok(version_json)
}
