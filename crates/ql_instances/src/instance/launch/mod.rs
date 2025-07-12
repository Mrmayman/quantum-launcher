use crate::auth::AccountData;
use error::GameLaunchError;
use ql_core::{info, GenericProgress};
use std::sync::{mpsc::Sender, Arc, Mutex};
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
    auth: Option<AccountData>,
) -> Result<Arc<Mutex<Child>>, GameLaunchError> {
    if username.is_empty() {
        return Err(GameLaunchError::UsernameIsEmpty);
    }
    if username.contains(' ') {
        return Err(GameLaunchError::UsernameHasSpaces);
    }

    let mut game_launcher =
        GameLauncher::new(instance_name, username, java_install_progress_sender).await?;

    game_launcher.migrate_old_instances().await?;
    game_launcher.create_mods_dir().await?;

    let mut game_arguments = game_launcher.init_game_arguments(auth.as_ref())?;
    let mut java_arguments = game_launcher.init_java_arguments(auth.as_ref()).await?;

    let fabric_json = game_launcher
        .setup_fabric(&mut java_arguments, &mut game_arguments)
        .await?;
    let forge_json = game_launcher
        .setup_forge(&mut java_arguments, &mut game_arguments)
        .await?;
    let optifine_json = game_launcher.setup_optifine(&mut game_arguments).await?;

    game_launcher.fill_java_arguments(&mut java_arguments);

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

    info!("Java args: {java_arguments:?}\n");

    print_censored_args(auth.as_ref(), &mut game_arguments);

    let mut command = game_launcher
        .get_command(game_arguments, java_arguments)
        .await?;
    let child = command.spawn().map_err(GameLaunchError::CommandError)?;

    if game_launcher.config_json.close_on_start.unwrap_or(false) {
        ql_core::logger_finish();
        std::process::exit(0);
    }

    Ok(Arc::new(Mutex::new(child)))
}

fn print_censored_args(auth: Option<&AccountData>, game_arguments: &mut Vec<String>) {
    censor(game_arguments, "--clientId", |args| {
        censor(args, "--session", |args| {
            censor(args, "--accessToken", |args| {
                censor(args, "--uuid", |args| {
                    censor_string(
                        args,
                        &auth
                            .as_ref()
                            .and_then(|n| n.access_token.clone())
                            .unwrap_or_default(),
                        |args| {
                            info!("Game args: {args:?}\n");
                        },
                    );
                });
            });
        });
    });
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
