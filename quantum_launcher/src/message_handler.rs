use std::sync::Arc;

use iced::Command;
use quantum_launcher_backend::instance::instance_launch::GameLaunchResult;

use crate::{
    launcher_state::{Launcher, Message, State},
    MINECRAFT_MEMORY,
};

impl Launcher {
    pub fn m_launch_instance_selected(&mut self, instance_name: String) {
        if let State::Launch {
            ref mut selected_instance,
            ..
        } = self.state
        {
            *selected_instance = instance_name
        }
    }

    pub fn m_launch_username_set(&mut self, username: String) {
        self.config.as_mut().unwrap().username = username;
    }

    pub fn m_launch_start(&mut self) -> Command<Message> {
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

    pub fn m_launch_end(&mut self, result: GameLaunchResult) {
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

    pub fn m_create(&mut self) -> Command<Message> {
        self.state = State::Create {
            instance_name: Default::default(),
            version: Default::default(),
            versions: Vec::new(),
            progress: None,
            progress_num: None,
        };
        Command::perform(
            quantum_launcher_backend::list_versions(),
            Message::CreateInstanceVersionsLoaded,
        )
    }

    pub fn m_create_versions_loaded(&mut self, result: Result<Arc<Vec<String>>, String>) {
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
}
