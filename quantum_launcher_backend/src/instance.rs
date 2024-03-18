use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
    process::Command,
    sync::Arc,
};

use crate::{
    download::GameDownloader,
    error::{LauncherError, LauncherResult},
    file_utils::{self, create_dir_if_not_exists},
    java_locate::JavaInstall,
    json_structs::json_version::VersionDetails,
};

const CLASSPATH_SEPARATOR: char = if cfg!(unix) { ':' } else { ';' };

pub async fn launch(instance_name: String, username: String, memory: &str) -> Option<String> {
    if let Err(err) = launch_game(&instance_name, &username, memory) {
        Some(format!("{:?}", err))
    } else {
        None
    }
}

fn launch_game(instance_name: &str, username: &str, memory: &str) -> LauncherResult<()> {
    let launcher_dir = file_utils::get_launcher_dir()?;

    let instances_dir = launcher_dir.join("instances");
    file_utils::create_dir_if_not_exists(&instances_dir)?;

    let instance_dir = instances_dir.join(instance_name);
    if !instance_dir.exists() {
        return Err(LauncherError::InstanceNotFound);
    }

    let minecraft_dir = instance_dir.join(".minecraft");
    create_dir_if_not_exists(&minecraft_dir)?;

    let version_json: VersionDetails = launch_read_version_details(&instance_dir)?;

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

    println!("{:?}, \n\n{:?}", java_args, game_arguments);

    let mut command = Command::new(&appropriate_install.path);
    let command = command.args(java_args.iter().chain(game_arguments.iter()));
    let result = command.output()?;
    println!(
        "Output: {}\n\nError: {}",
        String::from_utf8(result.stdout).unwrap(),
        String::from_utf8(result.stderr).unwrap()
    );

    Ok(())
}

fn replace_var(string: &mut String, var: &str, value: &str) {
    *string = string.replace(&format!("${{{}}}", var), value);
}

fn launch_read_version_details(instance_dir: &Path) -> LauncherResult<VersionDetails> {
    let mut file = File::open(instance_dir.join("details.json"))?;
    let mut version_json: String = Default::default();
    file.read_to_string(&mut version_json)?;
    let version_json = serde_json::from_str(&version_json)?;
    Ok(version_json)
}

pub async fn create(instance_name: &str, version: String) -> LauncherResult<()> {
    println!("[info] Started creating instance.");

    let game_downloader = GameDownloader::new(instance_name, &version)?;
    game_downloader.download_jar()?;
    game_downloader.download_libraries()?;
    game_downloader.download_logging_config()?;
    game_downloader.download_assets()?;

    let mut json_file = File::create(game_downloader.instance_dir.join("details.json"))?;
    json_file.write_all(serde_json::to_string(&game_downloader.version_json)?.as_bytes())?;

    Ok(())
}
