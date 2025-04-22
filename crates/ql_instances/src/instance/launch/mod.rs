use crate::mc_auth::AccountData;
use error::GameLaunchError;
use ql_core::{info, no_window, GenericProgress};
use std::{
    process::Stdio,
    sync::{mpsc::Sender, Arc, Mutex},
};
use tokio::process::Child;

pub(super) mod error;
mod launcher;
pub use launcher::GameLauncher;

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
) -> Result<Arc<Mutex<Child>>, GameLaunchError> {
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

    let mut game_arguments = game_launcher.init_game_arguments()?;
    let mut java_arguments = game_launcher.init_java_arguments(auth.is_some())?;

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
                censor_string(
                    args,
                    auth.as_ref()
                        .map(|n| n.access_token.as_deref().unwrap_or_default())
                        .unwrap_or_default(),
                    |args| {
                        info!("Game args: {args:?}\n");
                    },
                );
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

    if game_launcher.config_json.enable_logger.unwrap_or(true) {
        no_window!(command);
    }

    #[cfg(all(target_arch = "aarch64", target_os = "linux"))]
    {
        use chrono::DateTime;
        use ql_core::err;

        match (
            DateTime::parse_from_rfc3339(&game_launcher.version_json.releaseTime),
            // Minecraft 21w19a release date (1.17 snapshot)
            // Not sure if this is the right place to start,
            // but the env var started being required sometime between 1.16.5 and 1.17
            DateTime::parse_from_rfc3339("2021-05-12T11:19:15+00:00"),
        ) {
            // On Raspberry Pi (aarch64 linux), the game crashes with some GL
            // error. But adding this environment variable fixes it.
            // I don't know if this is the perfect solution though,
            // contact me if this solution sucks.
            (Ok(dt), Ok(v1_20)) => {
                if dt >= v1_20 {
                    command = command.env("MESA_GL_VERSION_OVERRIDE", "3.3")
                }
            }
            (Err(e), Err(_) | Ok(_)) | (Ok(_), Err(e)) => {
                err!("Could not parse instance date/time: {e}")
            }
        }
    }

    let child = command.spawn().map_err(GameLaunchError::CommandError)?;

    if game_launcher.config_json.close_on_start.unwrap_or(false) {
        ql_core::logger_finish();
        std::process::exit(0);
    }

    Ok(Arc::new(Mutex::new(child)))
}

fn censor<F: FnOnce(&mut Vec<String>)>(vec: &mut Vec<String>, argument: &str, code: F) {
    if let Some(index) = vec
        .iter_mut()
        .enumerate()
        .find_map(|(i, n)| (n == argument).then_some(i))
    {
        let old_id = vec.get(index + 1).cloned();
        if let Some(n) = vec.get_mut(index + 1) {
            "[REDACTED]".clone_into(n);
        }

        code(vec);

        if let (Some(n), Some(old_id)) = (vec.get_mut(index + 1), old_id) {
            *n = old_id;
        }
    } else {
        code(vec);
    }
}

fn censor_string<F: FnOnce(&mut Vec<String>)>(vec: &[String], argument: &str, code: F) {
    let mut new = vec.to_owned();
    for s in &mut new {
        if s == argument {
            "[REDACTED]".clone_into(s);
        }
    }

    code(&mut new);
}

fn replace_var(string: &mut String, var: &str, value: &str) {
    *string = string.replace(&format!("${{{var}}}"), value);
}
