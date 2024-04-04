use std::{
    path::PathBuf,
    sync::{mpsc, Arc},
};

use iced::Command;
use quantum_launcher_backend::{
    download::DownloadProgress, instance::instance_launch::GameLaunchResult,
};

use crate::{
    l10n,
    launcher_state::{Launcher, Message, State},
    MINECRAFT_MEMORY,
};

impl Launcher {
    pub fn select_launch_instance(&mut self, instance_name: String) {
        if let State::Launch {
            ref mut selected_instance,
            ..
        } = self.state
        {
            *selected_instance = instance_name
        }
    }

    pub fn set_username(&mut self, username: String) {
        self.config.as_mut().unwrap().username = username;
    }

    pub fn launch_game(&mut self) -> Command<Message> {
        if let State::Launch {
            ref mut selected_instance,
            ..
        } = self.state
        {
            match self.config.as_ref().unwrap().save() {
                Ok(_) => {
                    let selected_instance = selected_instance.clone();
                    let username = self.config.as_ref().unwrap().username.clone();
                    let manually_added_versions =
                        self.config.as_ref().unwrap().java_installs.clone();

                    return Command::perform(
                        quantum_launcher_backend::launch(
                            selected_instance,
                            username,
                            MINECRAFT_MEMORY,
                            manually_added_versions,
                        ),
                        Message::LaunchEnd,
                    );
                }
                Err(err) => self.set_error(err.to_string()),
            };
        }
        Command::none()
    }

    pub fn finish_launching(&mut self, result: GameLaunchResult) {
        match result {
            GameLaunchResult::Ok(child) => {
                if let State::Launch {
                    ref mut spawned_process,
                    ..
                } = self.state
                {
                    *spawned_process = Some(child)
                }
            }
            GameLaunchResult::Err(err) => self.state = State::Error { error: err },
            GameLaunchResult::LocateJavaManually {
                required_java_version,
            } => {
                self.state = State::FindJavaVersion {
                    version: None,
                    required_version: required_java_version,
                }
            }
        }
    }

    pub fn go_to_create_screen(&mut self) -> Command<Message> {
        self.state = State::Create {
            instance_name: Default::default(),
            version: Default::default(),
            versions: Vec::new(),
            progress_reciever: None,
            progress_number: None,
            progress_text: None,
        };
        Command::perform(
            quantum_launcher_backend::list_versions(),
            Message::CreateInstanceVersionsLoaded,
        )
    }

    pub fn create_instance_finish_loading_versions_list(
        &mut self,
        result: Result<Arc<Vec<String>>, String>,
    ) {
        match result {
            Ok(version_list) => {
                if let State::Create {
                    ref mut versions, ..
                } = self.state
                {
                    versions.extend_from_slice(&version_list)
                }
            }
            Err(n) => self.state = State::Error { error: n },
        }
    }

    pub fn select_created_instance_version(&mut self, selected_version: String) {
        if let State::Create {
            ref mut version, ..
        } = self.state
        {
            *version = selected_version
        }
    }

    pub fn update_created_instance_name(&mut self, name: String) {
        if let State::Create {
            ref mut instance_name,
            ..
        } = self.state
        {
            *instance_name = name
        }
    }

    pub fn create_instance(&mut self) -> Command<Message> {
        if let State::Create {
            ref instance_name,
            ref version,
            ref mut progress_reciever,
            ref mut progress_number,
            ..
        } = self.state
        {
            let (sender, receiver) = mpsc::channel::<DownloadProgress>();
            *progress_reciever = Some(receiver);
            *progress_number = Some(0.0);

            // Create Instance asynchronously using iced Command.
            return Command::perform(
                quantum_launcher_backend::create_instance(
                    instance_name.to_owned(),
                    version.to_owned(),
                    Some(sender),
                ),
                Message::CreateInstanceEnd,
            );
        }
        Command::none()
    }

    pub fn delete_selected_instance(&mut self) {
        if let State::DeleteInstance {
            ref selected_instance,
        } = self.state
        {
            match quantum_launcher_backend::file_utils::get_launcher_dir() {
                Ok(launcher_dir) => {
                    let instances_dir = launcher_dir.join("instances");
                    let deleted_instance_dir = instances_dir.join(selected_instance);
                    if deleted_instance_dir.starts_with(&instances_dir) {
                        if let Err(err) = std::fs::remove_dir_all(&deleted_instance_dir) {
                            self.set_error(err.to_string())
                        } else {
                            self.go_to_launch_screen()
                        }
                    } else {
                        self.set_error("Tried to delete instance folder located outside Launcher. Potential attack avoided.".to_owned())
                    }
                }
                Err(err) => self.set_error(err.to_string()),
            }
        }
    }

    pub fn confirm_instance_deletion(&mut self) {
        if let State::Launch {
            ref selected_instance,
            ..
        } = self.state
        {
            self.state = State::DeleteInstance {
                selected_instance: selected_instance.clone(),
            }
        }
    }

    pub fn update_instance_creation_progress_bar(&mut self) {
        if let State::Create {
            ref mut progress_number,
            ref progress_reciever,
            ref mut progress_text,
            ..
        } = self.state
        {
            if let Some(Ok(progress)) = progress_reciever.as_ref().map(|n| n.try_recv()) {
                if let Some(progress_text) = progress_text {
                    *progress_text = progress.to_string()
                }
                if let Some(progress_num) = progress_number {
                    *progress_num = progress.into();
                }
            }
        }
    }

    pub fn add_java_to_config(&mut self, path: Option<PathBuf>) {
        match (path.as_ref().map(|n| n.to_str()), &mut self.config) {
            // Config not loaded.
            (_, None) => self.set_error(format!(
                "{} (QuantumLauncher/launcher.config)",
                l10n!(ENGLISH, CouldNotOpenLauncherConfig)
            )),
            // Couldn't find path.
            (None, _) => self.set_error(l10n!(ENGLISH, SelectedJavaPathNotFound).to_owned()),
            // Couldn't convert path to string.
            (Some(None), _) => self.set_error(l10n!(ENGLISH, InvalidCharsInJavaPath).to_owned()),
            (Some(Some(path)), Some(config)) => {
                // Add java path to list of java installs.
                config.java_installs.push(path.to_owned());
                // Save config.
                match config.save() {
                    Ok(_) => {}
                    Err(err) => self.set_error(err.to_string()),
                }
                self.go_to_launch_screen()
            }
        }
    }
}
