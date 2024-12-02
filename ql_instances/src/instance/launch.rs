use tokio::process::{Child, Command};

use crate::{
    download::GameDownloader,
    error::{IoError, LauncherError, LauncherResult},
    file_utils, info, io_err,
    java_install::{self, JavaInstallProgress},
    json_structs::{
        json_fabric::FabricJSON,
        json_forge::JsonForgeDetails,
        json_instance_config::InstanceConfigJson,
        json_java_list::JavaVersion,
        json_optifine::JsonOptifine,
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
    asset_redownload_progress: Option<Sender<AssetRedownloadProgress>>,
) -> GameLaunchResult {
    match launch(
        instance_name,
        username,
        java_install_progress_sender,
        enable_logger,
        asset_redownload_progress,
    )
    .await
    {
        Ok(child) => GameLaunchResult::Ok(Arc::new(Mutex::new(child))),
        Err(err) => GameLaunchResult::Err(err.to_string()),
    }
}

pub struct GameLauncher {
    pub username: String,
    pub instance_name: String,
    pub java_install_progress_sender: Option<Sender<JavaInstallProgress>>,
    pub asset_redownload_progress: Option<Sender<AssetRedownloadProgress>>,
    pub instance_dir: PathBuf,
    pub minecraft_dir: PathBuf,
    pub config_json: InstanceConfigJson,
    pub version_json: VersionDetails,
}

impl GameLauncher {
    pub fn new(
        instance_name: String,
        username: String,
        java_install_progress_sender: Option<Sender<JavaInstallProgress>>,
        asset_redownload_progress: Option<Sender<AssetRedownloadProgress>>,
    ) -> Result<Self, LauncherError> {
        let instance_dir = get_instance_dir(&instance_name)?;

        let minecraft_dir = instance_dir.join(".minecraft");
        std::fs::create_dir_all(&minecraft_dir).map_err(io_err!(minecraft_dir))?;

        let config_json = get_config(&instance_dir)?;

        let version_json = read_version_json(&instance_dir)?;

        Ok(Self {
            instance_name,
            username,
            java_install_progress_sender,
            asset_redownload_progress,
            instance_dir,
            minecraft_dir,
            config_json,
            version_json,
        })
    }

    async fn init_game_arguments(&self) -> LauncherResult<Vec<String>> {
        let mut game_arguments: Vec<String> =
            if let Some(arguments) = &self.version_json.minecraftArguments {
                arguments.split(' ').map(ToOwned::to_owned).collect()
            } else if let Some(arguments) = &self.version_json.arguments {
                arguments
                    .game
                    .iter()
                    .filter_map(|arg| arg.as_str())
                    .map(ToOwned::to_owned)
                    .collect()
            } else {
                return Err(LauncherError::VersionJsonNoArgumentsField(Box::new(
                    self.version_json.clone(),
                )));
            };
        self.fill_game_arguments(&mut game_arguments).await?;
        Ok(game_arguments)
    }

    async fn fill_game_arguments(
        &self,
        game_arguments: &mut [String],
    ) -> Result<(), LauncherError> {
        for argument in game_arguments.iter_mut() {
            replace_var(argument, "auth_player_name", &self.username);
            replace_var(argument, "version_name", &self.version_json.id);
            let Some(minecraft_dir_path) = self.minecraft_dir.to_str() else {
                return Err(LauncherError::PathBufToString(
                    self.minecraft_dir.to_owned(),
                ));
            };
            replace_var(argument, "game_directory", minecraft_dir_path);

            self.set_assets_argument(argument).await?;
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
            replace_var(
                argument,
                "assets_index_name",
                &self.version_json.assetIndex.id,
            );
            replace_var(argument, "user_properties", "{}");
        }
        Ok(())
    }

    async fn set_assets_argument(&self, argument: &mut String) -> Result<(), LauncherError> {
        let old_assets_path_v2 = file_utils::get_launcher_dir()?
            .join("assets")
            .join(&self.version_json.assetIndex.id);

        let old_assets_path_v1 = self.instance_dir.join("assets");

        if self.version_json.assetIndex.id == "legacy" {
            let assets_path = file_utils::get_launcher_dir()?.join("assets/legacy_assets");

            if old_assets_path_v2.exists() {
                self.redownload_legacy_assets().await?;
                std::fs::remove_dir_all(&old_assets_path_v2)
                    .map_err(io_err!(old_assets_path_v2))?;
            }

            if old_assets_path_v1.exists() {
                self.redownload_legacy_assets().await?;
                std::fs::remove_dir_all(&old_assets_path_v1)
                    .map_err(io_err!(old_assets_path_v1))?;
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
                std::fs::remove_dir_all(&old_assets_path_v2)
                    .map_err(io_err!(old_assets_path_v2))?;
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

    fn create_mods_dir(&self) -> LauncherResult<()> {
        let mods_dir = self.minecraft_dir.join("mods");
        std::fs::create_dir_all(&mods_dir).map_err(io_err!(mods_dir))?;
        Ok(())
    }

    async fn redownload_legacy_assets(&self) -> Result<(), LauncherError> {
        info!("Redownloading legacy assets");
        let game_downloader = GameDownloader::with_existing_instance(
            self.version_json.clone(),
            self.instance_dir.to_owned(),
            None,
        );
        game_downloader
            .download_assets(self.asset_redownload_progress.as_ref())
            .await?;
        Ok(())
    }

    async fn init_java_arguments(&self) -> LauncherResult<Vec<String>> {
        let natives_path = self.instance_dir.join("libraries").join("natives");

        let mut args = vec![
            "-Xss1M".to_owned(),
            "-Dminecraft.launcher.brand=minecraft-launcher".to_owned(),
            "-Dminecraft.launcher.version=2.1.1349".to_owned(),
            format!(
                "-Djava.library.path={}",
                natives_path
                    .to_str()
                    .ok_or(LauncherError::PathBufToString(natives_path.clone()))?
            ),
            self.config_json.get_ram_argument(),
        ];

        if self.version_json.r#type == "old_beta" || self.version_json.r#type == "old_alpha" {
            args.push("-Dhttp.proxyHost=betacraft.uk".to_owned());
        }

        Ok(args)
    }

    fn setup_fabric(
        &self,
        java_arguments: &mut Vec<String>,
    ) -> Result<Option<FabricJSON>, LauncherError> {
        if self.config_json.mod_type != "Fabric" {
            return Ok(None);
        }

        let fabric_json = self.get_fabric_json()?;
        java_arguments.extend(fabric_json.arguments.jvm.clone());

        Ok(Some(fabric_json))
    }

    async fn setup_forge(
        &self,
        java_arguments: &mut Vec<String>,
        game_arguments: &mut Vec<String>,
    ) -> Result<Option<JsonForgeDetails>, LauncherError> {
        if self.config_json.mod_type != "Forge" {
            return Ok(None);
        }

        let json = self.get_forge_json()?;

        if let Some(arguments) = &json.arguments {
            if let Some(jvm) = &arguments.jvm {
                java_arguments.extend(jvm.clone());
            }
            game_arguments.extend(arguments.game.clone());
        } else if let Some(arguments) = &json.minecraftArguments {
            *game_arguments = arguments.split(' ').map(str::to_owned).collect();
            self.fill_game_arguments(game_arguments).await?;
        }
        Ok(Some(json))
    }

    fn get_fabric_json(&self) -> Result<FabricJSON, JsonFileError> {
        let json_path = self.instance_dir.join("fabric.json");
        let fabric_json = std::fs::read_to_string(&json_path).map_err(io_err!(json_path))?;
        Ok(serde_json::from_str(&fabric_json)?)
    }

    fn get_forge_json(&self) -> Result<JsonForgeDetails, JsonFileError> {
        let json_path = self.instance_dir.join("forge/details.json");
        let json = std::fs::read_to_string(&json_path).map_err(io_err!(json_path))?;
        Ok(serde_json::from_str(&json)?)
    }

    async fn setup_optifine(
        &self,
        game_arguments: &mut Vec<String>,
    ) -> LauncherResult<Option<(JsonOptifine, PathBuf)>> {
        if self.config_json.mod_type != "OptiFine" {
            return Ok(None);
        }

        let (optifine_json, jar) = JsonOptifine::read(&self.instance_name)?;
        if let Some(arguments) = &optifine_json.arguments {
            game_arguments.extend(arguments.game.clone());
        } else if let Some(arguments) = &optifine_json.minecraftArguments {
            *game_arguments = arguments.split(' ').map(str::to_owned).collect();
            self.fill_game_arguments(game_arguments).await?;
        }

        Ok(Some((optifine_json, jar)))
    }

    fn fill_java_arguments(&self, java_arguments: &mut Vec<String>) -> LauncherResult<()> {
        for argument in java_arguments {
            replace_var(
                argument,
                "classpath_separator",
                &CLASSPATH_SEPARATOR.to_string(),
            );
            let library_directory = self.instance_dir.join("forge/libraries");
            replace_var(
                argument,
                "library_directory",
                library_directory
                    .to_str()
                    .ok_or(LauncherError::PathBufToString(library_directory.clone()))?,
            );
            replace_var(argument, "version_name", &self.version_json.id);
        }
        Ok(())
    }

    fn setup_logging(&self, java_arguments: &mut Vec<String>) -> Result<(), LauncherError> {
        if let Some(logging) = &self.version_json.logging {
            let logging_path = self
                .instance_dir
                .join(format!("logging-{}", logging.client.file.id));
            let logging_path = logging_path
                .to_str()
                .ok_or(LauncherError::PathBufToString(logging_path.clone()))?;
            java_arguments.push(format!("-Dlog4j.configurationFile={logging_path}"));
        }
        Ok(())
    }

    fn setup_classpath_and_mainclass(
        &self,
        java_arguments: &mut Vec<String>,
        fabric_json: Option<FabricJSON>,
        forge_json: Option<JsonForgeDetails>,
        optifine_json: Option<(JsonOptifine, PathBuf)>,
    ) -> Result<(), LauncherError> {
        java_arguments.push("-cp".to_owned());
        java_arguments.push(self.get_class_path(
            fabric_json.as_ref(),
            forge_json.as_ref(),
            optifine_json.as_ref(),
        )?);
        java_arguments.push(if let Some(fabric_json) = fabric_json {
            fabric_json.mainClass
        } else if let Some(forge_json) = forge_json {
            forge_json.mainClass.clone()
        } else if let Some((optifine_json, _)) = &optifine_json {
            optifine_json.mainClass.clone()
        } else {
            self.version_json.mainClass.clone()
        });
        Ok(())
    }

    fn get_class_path(
        &self,
        fabric_json: Option<&FabricJSON>,
        forge_json: Option<&JsonForgeDetails>,
        optifine_json: Option<&(JsonOptifine, PathBuf)>,
    ) -> LauncherResult<String> {
        let mut class_path = String::new();

        if forge_json.is_some() {
            let classpath_path = self.instance_dir.join("forge/classpath.txt");
            let forge_classpath =
                std::fs::read_to_string(&classpath_path).map_err(io_err!(classpath_path))?;
            class_path.push_str(&forge_classpath);
        }

        if optifine_json.is_some() {
            let jar_file_location = self.instance_dir.join(".minecraft/libraries");
            let jar_files = find_jar_files(&jar_file_location)?;
            for jar_file in jar_files {
                class_path.push_str(
                    jar_file
                        .to_str()
                        .ok_or(LauncherError::PathBufToString(jar_file.clone()))?,
                );
                class_path.push(CLASSPATH_SEPARATOR);
            }
        }

        self.add_libs_to_classpath(&mut class_path)?;

        if let Some(fabric_json) = fabric_json {
            for library in &fabric_json.libraries {
                let library_path = self.instance_dir.join("libraries").join(library.get_path());
                class_path.push_str(
                    library_path
                        .to_str()
                        .ok_or(LauncherError::PathBufToString(library_path.clone()))?,
                );
                class_path.push(CLASSPATH_SEPARATOR);
            }
        }

        let jar_path = if let Some((_, jar)) = optifine_json {
            jar.to_owned()
        } else {
            self.instance_dir
                .join(".minecraft/versions")
                .join(&self.version_json.id)
                .join(format!("{}.jar", self.version_json.id))
        };
        let jar_path = jar_path
            .to_str()
            .ok_or(LauncherError::PathBufToString(jar_path.clone()))?;
        class_path.push_str(jar_path);

        Ok(class_path)
    }

    fn add_libs_to_classpath(&self, class_path: &mut String) -> Result<(), LauncherError> {
        self.version_json
            .libraries
            .iter()
            .filter_map(|n| match n.downloads.as_ref() {
                Some(LibraryDownloads::Normal { artifact, .. }) => Some(artifact),
                _ => None,
            })
            .map(|artifact| {
                let library_path = self.instance_dir.join("libraries").join(&artifact.path);
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
        Ok(())
    }

    async fn get_java_command(&mut self) -> LauncherResult<Command> {
        if let Some(java_override) = &self.config_json.java_override {
            Ok(Command::new(java_override))
        } else {
            let version = if let Some(version) = self.version_json.javaVersion.clone() {
                version.into()
            } else {
                JavaVersion::Java8
            };
            Ok(Command::new(
                java_install::get_java_binary(
                    version,
                    "java",
                    self.java_install_progress_sender.take(),
                )
                .await?,
            ))
        }
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
    instance_name: String,
    username: String,
    java_install_progress_sender: Option<Sender<JavaInstallProgress>>,
    enable_logger: bool,
    asset_redownload_progress: Option<Sender<AssetRedownloadProgress>>,
) -> LauncherResult<Child> {
    if username.contains(' ') || username.is_empty() {
        return Err(LauncherError::UsernameIsInvalid(username.to_owned()));
    }

    let mut game_launcher = GameLauncher::new(
        instance_name,
        username,
        java_install_progress_sender,
        asset_redownload_progress,
    )?;

    game_launcher.migrate_old_instances().await?;
    game_launcher.create_mods_dir()?;

    let mut game_arguments = game_launcher.init_game_arguments().await?;
    let mut java_arguments = game_launcher.init_java_arguments().await?;

    let fabric_json = game_launcher.setup_fabric(&mut java_arguments)?;
    let forge_json = game_launcher
        .setup_forge(&mut java_arguments, &mut game_arguments)
        .await?;
    let optifine_json = game_launcher.setup_optifine(&mut game_arguments).await?;

    game_launcher.fill_java_arguments(&mut java_arguments)?;
    game_launcher.setup_logging(&mut java_arguments)?;
    game_launcher.setup_classpath_and_mainclass(
        &mut java_arguments,
        fabric_json,
        forge_json,
        optifine_json,
    )?;

    let mut command = game_launcher.get_java_command().await?;

    info!("Java args: {java_arguments:?}\n");
    info!("Game args: {game_arguments:?}\n");

    let mut command = command.args(java_arguments.iter().chain(game_arguments.iter()));
    command = if enable_logger {
        command.stdout(Stdio::piped()).stderr(Stdio::piped())
    } else {
        command
    }
    .current_dir(&game_launcher.minecraft_dir);

    let result = command.spawn().map_err(LauncherError::CommandError)?;

    Ok(result)
}

fn get_config(instance_dir: &Path) -> Result<InstanceConfigJson, JsonFileError> {
    let config_file_path = instance_dir.join("config.json");
    let config_json =
        std::fs::read_to_string(&config_file_path).map_err(io_err!(config_file_path))?;
    Ok(serde_json::from_str(&config_json)?)
}

fn find_jar_files(dir_path: &Path) -> Result<Vec<PathBuf>, IoError> {
    let mut jar_files = Vec::new();

    // Recursively traverse the directory
    for entry in std::fs::read_dir(dir_path).map_err(io_err!(dir_path))? {
        let entry = entry.map_err(io_err!(dir_path))?;
        let path = entry.path();

        if path.is_dir() {
            // If the entry is a directory, recursively search it
            jar_files.extend(find_jar_files(&path)?);
        } else if let Some(extension) = path.extension() {
            // If the entry is a file, check if it has a .jar extension
            if extension == "jar" {
                jar_files.push(path);
            }
        }
    }

    Ok(jar_files)
}

pub enum AssetRedownloadProgress {
    P1Start,
    P2Progress { done: usize, out_of: usize },
    P3Done,
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
