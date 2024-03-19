use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    process::Child,
    sync::{Arc, Mutex},
};

use crate::{
    error::{LauncherError, LauncherResult},
    file_utils,
    java_locate::JavaInstall,
    json_structs::json_version::VersionDetails,
};

const CLASSPATH_SEPARATOR: char = if cfg!(unix) { ':' } else { ';' };

pub async fn launch(
    instance_name: String,
    username: String,
    memory: &str,
) -> Result<Arc<Mutex<Child>>, String> {
    match launch_blocking(&instance_name, &username, memory) {
        Ok(child) => Ok(Arc::new(Mutex::new(child))),
        Err(err) => Err(format!("Error: {:?}", err)),
    }
}

pub fn launch_blocking(instance_name: &str, username: &str, memory: &str) -> LauncherResult<Child> {
    let instance_dir = get_instance_dir(instance_name)?;

    let minecraft_dir = instance_dir.join(".minecraft");
    file_utils::create_dir_if_not_exists(&minecraft_dir)?;

    let version_json: VersionDetails = read_version_json(&instance_dir)?;

    let mut game_arguments: Vec<String> = if let Some(arguments) = version_json.minecraftArguments {
        arguments.split(' ').map(ToOwned::to_owned).collect()
    } else if let Some(arguments) = version_json.arguments {
        arguments
            .game
            .iter()
            .filter_map(|arg| arg.as_str())
            .map(ToOwned::to_owned)
            .collect()
    } else {
        return Err(LauncherError::VersionJsonNoArgumentsField(version_json));
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

    let mut java_args = vec![
        "-Xss1M".to_owned(),
        "-Dminecraft.launcher.brand=minecraft-launcher".to_owned(),
        "-Dminecraft.launcher.version=2.1.1349".to_owned(),
        format!("-Xmx{}", memory),
    ];

    if let Some(logging) = version_json.logging {
        let logging_path = instance_dir.join(format!("logging-{}", logging.client.file.id));
        let logging_path = match logging_path.to_str() {
            Some(n) => n,
            None => return Err(LauncherError::PathBufToString(logging_path)),
        };
        java_args.push(format!("-Dlog4j.configurationFile=\"{}\"", logging_path))
    }

    java_args.push("-cp".to_owned());

    let mut class_path: String = "\"".to_owned();

    for library in version_json.libraries {
        let library_path = instance_dir
            .join("libraries")
            .join(&library.downloads.artifact.path);
        if library_path.exists() {
            let library_path = match library_path.to_str() {
                Some(n) => n,
                None => return Err(LauncherError::PathBufToString(library_path)),
            };
            class_path.push_str(library_path);
            class_path.push(CLASSPATH_SEPARATOR);
        }
    }
    let jar_path = instance_dir.join("version.jar");
    let jar_path = match jar_path.to_str() {
        Some(n) => n,
        None => return Err(LauncherError::PathBufToString(jar_path)),
    };

    class_path.push_str(jar_path);
    class_path.push('"');

    java_args.push(class_path);
    java_args.push(version_json.mainClass.clone());

    let java_installations = JavaInstall::find_java_installs()?;
    let appropriate_install = java_installations
        .iter()
        .find(|n| n.version >= version_json.javaVersion.majorVersion)
        .ok_or(LauncherError::RequiredJavaVersionNotFound)?;

    let mut command = appropriate_install.get_command();
    let command = command.args(java_args.iter().chain(game_arguments.iter()));
    let result = command.spawn()?;

    Ok(result)
}

fn get_instance_dir(instance_name: &str) -> LauncherResult<PathBuf> {
    let launcher_dir = file_utils::get_launcher_dir()?;

    let instances_dir = launcher_dir.join("instances");
    file_utils::create_dir_if_not_exists(&instances_dir)?;

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
    let mut file = File::open(instance_dir.join("details.json"))?;
    let mut version_json: String = Default::default();
    file.read_to_string(&mut version_json)?;
    let version_json = serde_json::from_str(&version_json)?;
    Ok(version_json)
}
