use crate::{
    download::GameDownloader,
    mc_auth::{AccountData, CLIENT_ID},
};
use chrono::DateTime;
use error::GameLaunchError;
use ql_core::{
    err, file_utils, get_java_binary, info,
    json::{
        forge,
        version::{LibraryDownloadArtifact, LibraryDownloads},
        FabricJSON, InstanceConfigJson, JavaVersion, JsonOptifine, OmniarchiveEntry,
        VersionDetails,
    },
    GenericProgress, IntoIoError, IoError, JsonFileError, CLASSPATH_SEPARATOR,
};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    process::Stdio,
    sync::{mpsc::Sender, Arc, Mutex},
};
use tokio::process::{Child, Command};

pub(super) mod error;

pub type GameLaunchResult = Result<Arc<Mutex<Child>>, String>;

/// [`launch`] `_w` function
pub async fn launch_w(
    instance_name: String,
    username: String,
    java_install_progress_sender: Option<Sender<GenericProgress>>,
    asset_redownload_progress: Option<Sender<GenericProgress>>,
    auth: Option<AccountData>,
) -> GameLaunchResult {
    match launch(
        instance_name,
        username,
        java_install_progress_sender,
        asset_redownload_progress,
        auth,
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
    instance_name: String,
    username: String,
    java_install_progress_sender: Option<Sender<GenericProgress>>,
    asset_redownload_progress: Option<Sender<GenericProgress>>,
    auth: Option<AccountData>,
) -> Result<Child, GameLaunchError> {
    if username.is_empty() {
        return Err(GameLaunchError::UsernameIsEmpty);
    }
    if username.contains(' ') {
        return Err(GameLaunchError::UsernameHasSpaces);
    }

    let mut game_launcher = GameLauncher::new(
        instance_name,
        username,
        java_install_progress_sender,
        asset_redownload_progress,
    )
    .await?;

    game_launcher.migrate_old_instances().await?;
    game_launcher.create_mods_dir().await?;

    let mut game_arguments = game_launcher.init_game_arguments().await?;
    let mut java_arguments = game_launcher.init_java_arguments()?;

    let fabric_json = game_launcher
        .setup_fabric(&mut java_arguments, &mut game_arguments)
        .await?;
    let forge_json = game_launcher
        .setup_forge(&mut java_arguments, &mut game_arguments)
        .await?;
    let optifine_json = game_launcher.setup_optifine(&mut game_arguments).await?;

    game_launcher.fill_java_arguments(&mut java_arguments)?;

    game_launcher
        .fill_game_arguments(&mut game_arguments, auth.as_ref())
        .await?;

    game_launcher.setup_logging(&mut java_arguments)?;
    game_launcher
        .setup_classpath_and_mainclass(
            &mut java_arguments,
            fabric_json,
            forge_json,
            optifine_json.as_ref(),
        )
        .await?;

    let mut command = game_launcher.get_java_command().await?;

    info!("Java args: {java_arguments:?}\n");

    censor(&mut game_arguments, "--clientId", |args| {
        censor(args, "--accessToken", |args| {
            censor(args, "--uuid", |args| {
                info!("Game args: {args:?}\n");
            });
        });
    });

    let n = game_launcher
        .config_json
        .java_args
        .clone()
        .unwrap_or_default();

    let mut command = command.args(
        n.iter()
            .chain(java_arguments.iter())
            .chain(game_arguments.iter())
            .chain(
                game_launcher
                    .config_json
                    .game_args
                    .clone()
                    .unwrap_or_default()
                    .iter(),
            )
            .filter(|n| !n.is_empty()),
    );
    command = if game_launcher.config_json.enable_logger.unwrap_or(true) {
        command.stdout(Stdio::piped()).stderr(Stdio::piped())
    } else {
        command
    }
    .current_dir(&game_launcher.minecraft_dir);

    #[cfg(all(target_arch = "aarch64", target_os = "linux"))]
    match (
        DateTime::parse_from_rfc3339(&game_launcher.version_json.releaseTime),
        DateTime::parse_from_rfc3339("2023-06-02T08:36:17+00:00"), // Minecraft 1.20 release date
    ) {
        (Ok(dt), Ok(v1_20)) => {
            if dt >= v1_20 {
                command = command.env("MESA_GL_VERSION_OVERRIDE", "3.3")
            }
        }
        (Err(e), Err(_) | Ok(_)) | (Ok(_), Err(e)) => {
            err!("Could not parse instance date/time: {e}")
        }
    }

    let child = command.spawn().map_err(GameLaunchError::CommandError)?;

    Ok(child)
}

fn censor<F: FnOnce(&mut Vec<String>)>(vec: &mut Vec<String>, argument: &str, code: F) {
    if let Some(index) = vec
        .iter_mut()
        .enumerate()
        .find_map(|(i, n)| (n == argument).then_some(i))
    {
        let old_id = vec.get(index + 1).cloned();
        if let Some(n) = vec.get_mut(index + 1) {
            *n = "[REDACTED]".to_owned();
        }

        code(vec);

        if let (Some(n), Some(old_id)) = (vec.get_mut(index + 1), old_id) {
            *n = old_id;
        }
    } else {
        code(vec);
    }
}

pub struct GameLauncher {
    pub username: String,
    pub instance_name: String,
    pub java_install_progress_sender: Option<Sender<GenericProgress>>,
    pub asset_redownload_progress: Option<Sender<GenericProgress>>,
    pub instance_dir: PathBuf,
    pub minecraft_dir: PathBuf,
    pub config_json: InstanceConfigJson,
    pub version_json: VersionDetails,
}

impl GameLauncher {
    pub async fn new(
        instance_name: String,
        username: String,
        java_install_progress_sender: Option<Sender<GenericProgress>>,
        asset_redownload_progress: Option<Sender<GenericProgress>>,
    ) -> Result<Self, GameLaunchError> {
        let instance_dir = get_instance_dir(&instance_name).await?;

        let minecraft_dir = instance_dir.join(".minecraft");
        tokio::fs::create_dir_all(&minecraft_dir)
            .await
            .path(&minecraft_dir)?;

        let config_json = get_config(&instance_dir).await?;

        let version_json = read_version_json(&instance_dir).await?;

        Ok(Self {
            username,
            instance_name,
            java_install_progress_sender,
            asset_redownload_progress,
            instance_dir,
            minecraft_dir,
            config_json,
            version_json,
        })
    }

    async fn init_game_arguments(&self) -> Result<Vec<String>, GameLaunchError> {
        let game_arguments: Vec<String> =
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
                return Err(GameLaunchError::VersionJsonNoArgumentsField(Box::new(
                    self.version_json.clone(),
                )));
            };

        Ok(game_arguments)
    }

    async fn fill_game_arguments(
        &self,
        game_arguments: &mut [String],
        account_details: Option<&AccountData>,
    ) -> Result<(), GameLaunchError> {
        for argument in game_arguments.iter_mut() {
            replace_var(argument, "auth_player_name", &self.username);
            replace_var(argument, "version_name", &self.version_json.id);
            let Some(minecraft_dir_path) = self.minecraft_dir.to_str() else {
                return Err(GameLaunchError::PathBufToString(self.minecraft_dir.clone()));
            };
            replace_var(argument, "game_directory", minecraft_dir_path);

            self.set_assets_argument(argument).await?;
            replace_var(argument, "auth_xuid", "0");

            let uuid = if let Some(account_details) = account_details {
                &account_details.uuid
            } else {
                "00000000-0000-0000-0000-000000000000"
            };
            replace_var(argument, "auth_uuid", uuid);
            replace_var(argument, "uuid", uuid);

            let access_token = if let Some(account_details) = account_details {
                &account_details.access_token
            } else {
                "0"
            };
            replace_var(argument, "auth_access_token", access_token);
            replace_var(argument, "auth_session", access_token);
            replace_var(argument, "accessToken", access_token);

            replace_var(argument, "clientid", CLIENT_ID);
            replace_var(
                argument,
                "user_type",
                if account_details.is_some() {
                    "msa"
                } else {
                    "legacy"
                },
            );
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

    async fn set_assets_argument(&self, argument: &mut String) -> Result<(), GameLaunchError> {
        let old_assets_path_v2 = file_utils::get_launcher_dir()
            .await?
            .join("assets")
            .join(&self.version_json.assetIndex.id);

        let old_assets_path_v1 = self.instance_dir.join("assets");

        if self.version_json.assetIndex.id == "legacy" {
            let assets_path = file_utils::get_launcher_dir()
                .await?
                .join("assets/legacy_assets");

            if old_assets_path_v2.exists() {
                self.redownload_legacy_assets().await?;
                tokio::fs::remove_dir_all(&old_assets_path_v2)
                    .await
                    .path(old_assets_path_v2)?;
            }

            if old_assets_path_v1.exists() {
                self.redownload_legacy_assets().await?;
                tokio::fs::remove_dir_all(&old_assets_path_v1)
                    .await
                    .path(old_assets_path_v1)?;
            }

            let assets_path_fixed = if assets_path.exists() {
                assets_path
            } else {
                file_utils::get_launcher_dir().await?.join("assets/null")
            };

            let Some(assets_path) = assets_path_fixed.to_str() else {
                return Err(GameLaunchError::PathBufToString(assets_path_fixed));
            };
            replace_var(argument, "assets_root", assets_path);
            replace_var(argument, "game_assets", assets_path);
        } else {
            let assets_path = file_utils::get_launcher_dir().await?.join("assets/dir");

            if old_assets_path_v2.exists() {
                info!("Migrating old assets to new path...");
                copy_dir_recursive(&old_assets_path_v2, &assets_path).await?;
                tokio::fs::remove_dir_all(&old_assets_path_v2)
                    .await
                    .path(old_assets_path_v2)?;
            }

            if old_assets_path_v1.exists() {
                migrate_to_new_assets_path(&old_assets_path_v1, &assets_path).await?;
            }

            let assets_path_fixed = if assets_path.exists() {
                assets_path
            } else {
                file_utils::get_launcher_dir().await?.join("assets/null")
            };
            let Some(assets_path) = assets_path_fixed.to_str() else {
                return Err(GameLaunchError::PathBufToString(assets_path_fixed));
            };
            replace_var(argument, "assets_root", assets_path);
            replace_var(argument, "game_assets", assets_path);
        }
        Ok(())
    }

    async fn create_mods_dir(&self) -> Result<(), IoError> {
        let mods_dir = self.minecraft_dir.join("mods");
        tokio::fs::create_dir_all(&mods_dir).await.path(mods_dir)?;
        Ok(())
    }

    async fn redownload_legacy_assets(&self) -> Result<(), GameLaunchError> {
        info!("Redownloading legacy assets");
        let game_downloader = GameDownloader::with_existing_instance(
            self.version_json.clone(),
            self.instance_dir.clone(),
            None,
        );
        game_downloader
            .download_assets(self.asset_redownload_progress.as_ref())
            .await?;
        Ok(())
    }

    fn init_java_arguments(&self) -> Result<Vec<String>, GameLaunchError> {
        let natives_path = self.instance_dir.join("libraries").join("natives");
        let natives_path = natives_path
            .to_str()
            .ok_or(GameLaunchError::PathBufToString(natives_path.clone()))?;

        // TODO: deal with self.version_json.arguments.jvm (currently ignored)
        let mut args = vec![
            "-Xss1M".to_owned(),
            "-Dminecraft.launcher.brand=minecraft-launcher".to_owned(),
            "-Dminecraft.launcher.version=2.1.1349".to_owned(),
            format!("-Djava.library.path={natives_path}"),
            format!("-Djna.tmpdir={natives_path}"),
            format!("-Dorg.lwjgl.system.SharedLibraryExtractPath={natives_path}"),
            format!("-Dio.netty.native.workdir={natives_path}"),
            self.config_json.get_ram_argument(),
        ];

        if cfg!(target_os = "macos") {
            args.push("-XstartOnFirstThread".to_owned());
        }

        if let Some(OmniarchiveEntry { name, .. }) = &self.config_json.omniarchive {
            if name.starts_with("beta/b1.9/pre/") {
                args.push("-Dhttp.proxyHost=betacraft.uk".to_owned());
                args.push("-Dhttp.proxyPort=11706".to_owned());
                return Ok(args);
            }
        }
        if self.version_json.r#type == "old_beta" || self.version_json.r#type == "old_alpha" {
            args.push("-Dhttp.proxyHost=betacraft.uk".to_owned());
            if self.version_json.id.starts_with("c0.") {
                args.push("-Dhttp.proxyPort=11701".to_owned());
            } else if self.version_json.r#type == "old_alpha" {
                args.push("-Dhttp.proxyPort=11702".to_owned());
            } else {
                args.push("-Dhttp.proxyPort=11705".to_owned());
            }
            args.push("-Djava.util.Arrays.useLegacyMergeSort=true".to_owned());
        } else {
            match (
                DateTime::parse_from_rfc3339(&self.version_json.releaseTime),
                DateTime::parse_from_rfc3339("2013-04-25T15:45:00+00:00"),
            ) {
                (Ok(dt), Ok(v1_5_2)) => {
                    if dt <= v1_5_2 {
                        args.push("-Dhttp.proxyHost=betacraft.uk".to_owned());
                        args.push("-Dhttp.proxyPort=11707".to_owned());
                    }
                }
                (Err(e), Err(_) | Ok(_)) | (Ok(_), Err(e)) => {
                    err!("Could not parse instance date/time: {e}");
                }
            }
        }

        Ok(args)
    }

    async fn setup_fabric(
        &self,
        java_arguments: &mut Vec<String>,
        game_arguments: &mut Vec<String>,
    ) -> Result<Option<FabricJSON>, GameLaunchError> {
        if !(self.config_json.mod_type == "Fabric" || self.config_json.mod_type == "Quilt") {
            return Ok(None);
        }

        let fabric_json = self.get_fabric_json().await?;
        if let Some(jvm) = fabric_json.arguments.as_ref().and_then(|n| n.jvm.as_ref()) {
            java_arguments.extend(jvm.clone());
        }

        if let Some(jvm) = fabric_json.arguments.as_ref().and_then(|n| n.game.as_ref()) {
            game_arguments.extend(jvm.clone());
        }

        Ok(Some(fabric_json))
    }

    async fn setup_forge(
        &self,
        java_arguments: &mut Vec<String>,
        game_arguments: &mut Vec<String>,
    ) -> Result<Option<forge::JsonDetails>, GameLaunchError> {
        if self.config_json.mod_type != "Forge" {
            return Ok(None);
        }

        let json = self.get_forge_json().await?;

        if let Some(arguments) = &json.arguments {
            if let Some(jvm) = &arguments.jvm {
                java_arguments.extend(jvm.clone());
            }
            game_arguments.extend(arguments.game.clone());
        } else if let Some(arguments) = &json.minecraftArguments {
            *game_arguments = arguments.split(' ').map(str::to_owned).collect();
        }
        Ok(Some(json))
    }

    async fn get_fabric_json(&self) -> Result<FabricJSON, JsonFileError> {
        let json_path = self.instance_dir.join("fabric.json");
        let fabric_json = tokio::fs::read_to_string(&json_path)
            .await
            .path(json_path)?;
        Ok(serde_json::from_str(&fabric_json)?)
    }

    async fn get_forge_json(&self) -> Result<forge::JsonDetails, JsonFileError> {
        let json_path = self.instance_dir.join("forge/details.json");
        let json = tokio::fs::read_to_string(&json_path)
            .await
            .path(json_path)?;
        Ok(serde_json::from_str(&json)?)
    }

    async fn setup_optifine(
        &self,
        game_arguments: &mut Vec<String>,
    ) -> Result<Option<(JsonOptifine, PathBuf)>, GameLaunchError> {
        if self.config_json.mod_type != "OptiFine" {
            return Ok(None);
        }

        let (optifine_json, jar) = JsonOptifine::read(&self.instance_name).await?;
        if let Some(arguments) = &optifine_json.arguments {
            game_arguments.extend(arguments.game.clone());
        } else if let Some(arguments) = &optifine_json.minecraftArguments {
            *game_arguments = arguments.split(' ').map(str::to_owned).collect();
        }

        Ok(Some((optifine_json, jar)))
    }

    fn fill_java_arguments(&self, java_arguments: &mut Vec<String>) -> Result<(), GameLaunchError> {
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
                    .ok_or(GameLaunchError::PathBufToString(library_directory.clone()))?,
            );
            replace_var(argument, "version_name", &self.version_json.id);
        }
        Ok(())
    }

    fn setup_logging(&self, java_arguments: &mut Vec<String>) -> Result<(), GameLaunchError> {
        if let Some(logging) = &self.version_json.logging {
            let logging_path = self
                .instance_dir
                .join(format!("logging-{}", logging.client.file.id));
            let logging_path = logging_path
                .to_str()
                .ok_or(GameLaunchError::PathBufToString(logging_path.clone()))?;
            java_arguments.push(format!("-Dlog4j.configurationFile={logging_path}"));
        }
        Ok(())
    }

    async fn setup_classpath_and_mainclass(
        &self,
        java_arguments: &mut Vec<String>,
        fabric_json: Option<FabricJSON>,
        forge_json: Option<forge::JsonDetails>,
        optifine_json: Option<&(JsonOptifine, PathBuf)>,
    ) -> Result<(), GameLaunchError> {
        java_arguments.push("-cp".to_owned());
        java_arguments.push(
            self.get_class_path(fabric_json.as_ref(), forge_json.as_ref(), optifine_json)
                .await?,
        );
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

    async fn get_class_path(
        &self,
        fabric_json: Option<&FabricJSON>,
        forge_json: Option<&forge::JsonDetails>,
        optifine_json: Option<&(JsonOptifine, PathBuf)>,
    ) -> Result<String, GameLaunchError> {
        let mut class_path = String::new();
        let mut classpath_entries = HashSet::new();

        if forge_json.is_some() {
            let classpath_path = self.instance_dir.join("forge/classpath.txt");
            let forge_classpath = tokio::fs::read_to_string(&classpath_path)
                .await
                .path(classpath_path)?;

            class_path.push_str(&forge_classpath);

            let classpath_entries_path = self.instance_dir.join("forge/clean_classpath.txt");
            if let Ok(forge_classpath_entries) =
                tokio::fs::read_to_string(&classpath_entries_path).await
            {
                for entry in forge_classpath_entries.lines() {
                    classpath_entries.insert(entry.to_owned());
                }
            } else {
                self.migrate_create_forge_clean_classpath(
                    forge_classpath,
                    &mut classpath_entries,
                    classpath_entries_path,
                )
                .await?;
            }
        }

        if optifine_json.is_some() {
            let jar_file_location = self.instance_dir.join(".minecraft/libraries");
            let jar_files = find_jar_files(&jar_file_location).await?;
            for jar_file in jar_files {
                class_path.push_str(
                    jar_file
                        .to_str()
                        .ok_or(GameLaunchError::PathBufToString(jar_file.clone()))?,
                );
                class_path.push(CLASSPATH_SEPARATOR);
            }
        }

        self.add_libs_to_classpath(&mut class_path, &mut classpath_entries)?;

        if let Some(fabric_json) = fabric_json {
            for library in &fabric_json.libraries {
                let mut skip = false;
                if let Some(name) = remove_version_from_library(&library.name) {
                    skip = !classpath_entries.insert(name);
                }
                if skip {
                    continue;
                }

                let library_path = self.instance_dir.join("libraries").join(library.get_path());
                class_path.push_str(
                    library_path
                        .to_str()
                        .ok_or(GameLaunchError::PathBufToString(library_path.clone()))?,
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
            .ok_or(GameLaunchError::PathBufToString(jar_path.clone()))?;
        class_path.push_str(jar_path);

        Ok(class_path)
    }

    fn add_libs_to_classpath(
        &self,
        class_path: &mut String,
        classpath_entries: &mut HashSet<String>,
    ) -> Result<(), GameLaunchError> {
        self.version_json
            .libraries
            .iter()
            .filter(|n| GameDownloader::download_libraries_library_is_allowed(n))
            .filter_map(|n| match (&n.name, n.downloads.as_ref()) {
                (
                    Some(name),
                    Some(LibraryDownloads {
                        artifact: Some(artifact),
                        ..
                    }),
                ) => Some((name, artifact)),
                _ => None,
            })
            .map(|(name, artifact)| {
                self.add_entry_to_classpath(name, classpath_entries, artifact, class_path)
            })
            .find(std::result::Result::is_err)
            .unwrap_or(Ok(()))?;
        Ok(())
    }

    fn add_entry_to_classpath(
        &self,
        name: &str,
        classpath_entries: &mut HashSet<String>,
        artifact: &LibraryDownloadArtifact,
        class_path: &mut String,
    ) -> Result<(), GameLaunchError> {
        if let Some(name) = remove_version_from_library(name) {
            if classpath_entries.contains(&name) {
                return Ok(());
            }
            classpath_entries.insert(name);
        }
        let library_path = self.instance_dir.join("libraries").join(&artifact.path);

        if library_path.exists() {
            let Some(library_path) = library_path.to_str() else {
                return Err(GameLaunchError::PathBufToString(library_path));
            };
            class_path.push_str(library_path);
            class_path.push(CLASSPATH_SEPARATOR);
        }
        Ok(())
    }

    async fn get_java_command(&mut self) -> Result<Command, GameLaunchError> {
        if let Some(java_override) = &self.config_json.java_override {
            if !java_override.is_empty() {
                return Ok(Command::new(java_override));
            }
        }
        let version = if let Some(version) = self.version_json.javaVersion.clone() {
            version.into()
        } else {
            JavaVersion::Java8
        };
        Ok(Command::new(
            get_java_binary(
                version,
                "java",
                self.java_install_progress_sender.take().as_ref(),
            )
            .await?,
        ))
    }

    pub async fn cleanup_junk_files(&self) -> Result<(), GameLaunchError> {
        let forge_dir = self.instance_dir.join("forge");

        if forge_dir.exists() {
            delete_junk_file(&forge_dir, "ClientInstaller.class").await?;
            delete_junk_file(&forge_dir, "ClientInstaller.java").await?;
            delete_junk_file(&forge_dir, "ForgeInstaller.class").await?;
            delete_junk_file(&forge_dir, "ForgeInstaller.java").await?;
            delete_junk_file(&forge_dir, "launcher_profiles.json").await?;
            delete_junk_file(&forge_dir, "launcher_profiles_microsoft_store.json").await?;

            let versions_dir = forge_dir.join("versions").join(&self.version_json.id);
            if versions_dir.is_dir() {
                tokio::fs::remove_dir_all(&versions_dir)
                    .await
                    .path(versions_dir)?;
            }
        }

        Ok(())
    }
}

async fn delete_junk_file(forge_dir: &Path, path: &str) -> Result<(), GameLaunchError> {
    let path = forge_dir.join(path);
    if path.exists() {
        tokio::fs::remove_file(&path).await.path(path)?;
    }
    Ok(())
}

fn remove_version_from_library(library: &str) -> Option<String> {
    // Split the input string by colons
    let parts: Vec<&str> = library.split(':').collect();

    // Ensure the input has exactly three parts (group, name, version)
    if parts.len() == 3 {
        // Return the first two parts joined by a colon
        Some(format!("{}:{}", parts[0], parts[1]))
    } else {
        // Return None if the input format is incorrect
        None
    }
}

async fn get_config(instance_dir: &Path) -> Result<InstanceConfigJson, JsonFileError> {
    let config_file_path = instance_dir.join("config.json");
    let config_json = tokio::fs::read_to_string(&config_file_path)
        .await
        .path(config_file_path)?;
    Ok(serde_json::from_str(&config_json)?)
}

async fn find_jar_files(dir_path: &Path) -> Result<Vec<PathBuf>, IoError> {
    let mut jar_files = Vec::new();

    let mut dir = tokio::fs::read_dir(dir_path).await.path(dir_path)?;
    // Recursively traverse the directory
    while let Ok(Some(entry)) = dir.next_entry().await {
        let path = entry.path();

        if path.is_dir() {
            // If the entry is a directory, recursively search it
            jar_files.extend(Box::pin(find_jar_files(&path)).await?);
        } else if let Some(extension) = path.extension() {
            // If the entry is a file, check if it has a .jar extension
            if extension == "jar" {
                jar_files.push(path);
            }
        }
    }

    Ok(jar_files)
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
async fn migrate_to_new_assets_path(
    old_assets_path: &Path,
    assets_path: &Path,
) -> Result<(), IoError> {
    info!("Migrating old assets to new path...");
    copy_dir_recursive(old_assets_path, assets_path).await?;
    tokio::fs::remove_dir_all(old_assets_path)
        .await
        .path(old_assets_path)?;
    info!("Finished");
    Ok(())
}

async fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), IoError> {
    // Create the destination directory if it doesn't exist
    if !dst.exists() {
        tokio::fs::create_dir_all(dst).await.path(dst)?;
    }

    let mut dir = tokio::fs::read_dir(src).await.path(src)?;

    // Iterate over the directory entries
    while let Ok(Some(entry)) = dir.next_entry().await {
        let path = entry.path();
        let dest_path = dst.join(entry.file_name());

        if path.is_dir() {
            // Recursively copy the subdirectory
            Box::pin(copy_dir_recursive(&path, &dest_path)).await?;
        } else {
            // Copy the file to the destination directory
            tokio::fs::copy(&path, &dest_path).await.path(path)?;
        }
    }

    Ok(())
}

async fn get_instance_dir(instance_name: &str) -> Result<PathBuf, GameLaunchError> {
    if instance_name.is_empty() {
        return Err(GameLaunchError::InstanceNotFound);
    }

    let launcher_dir = file_utils::get_launcher_dir().await?;
    tokio::fs::create_dir_all(&launcher_dir)
        .await
        .path(&launcher_dir)?;

    let instances_folder_dir = launcher_dir.join("instances");
    tokio::fs::create_dir_all(&instances_folder_dir)
        .await
        .path(&instances_folder_dir)?;

    let instance_dir = instances_folder_dir.join(instance_name);
    if !instance_dir.exists() {
        return Err(GameLaunchError::InstanceNotFound);
    }
    Ok(instance_dir)
}

fn replace_var(string: &mut String, var: &str, value: &str) {
    *string = string.replace(&format!("${{{var}}}"), value);
}

async fn read_version_json(instance_dir: &Path) -> Result<VersionDetails, JsonFileError> {
    let file_path = instance_dir.join("details.json");

    let version_json: String = tokio::fs::read_to_string(&file_path)
        .await
        .path(file_path)?;
    let version_json = serde_json::from_str(&version_json)?;
    Ok(version_json)
}
