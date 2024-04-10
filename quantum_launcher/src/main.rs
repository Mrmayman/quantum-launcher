use std::path::PathBuf;

use config::LauncherConfig;
use iced::{
    executor,
    futures::SinkExt,
    subscription,
    widget::{self, column},
    Application, Command, Settings, Subscription, Theme,
};
use launcher_state::{Launcher, Message, State};
use message_handler::{format_memory, open_file_explorer};
use quantum_launcher_backend::{error::LauncherError, instance::instance_mod_installer};

mod config;
mod l10n;
mod launcher_state;
mod menu_renderer;
mod message_handler;

impl Application for Launcher {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, iced::Command<Self::Message>) {
        let state = match Launcher::load() {
            Ok(n) => n,
            Err(n) => {
                return (
                    Self {
                        state: State::Error {
                            error: format!("{}: {n}", l10n!(ENGLISH, Error)),
                        },
                        instances: None,
                        config: LauncherConfig::load().ok(),
                        spawned_process: None,
                    },
                    Command::none(),
                )
            }
        };

        (state, Command::none())
    }

    fn title(&self) -> String {
        "Quantum Launcher".to_owned()
    }

    fn update(&mut self, message: Self::Message) -> iced::Command<Self::Message> {
        match message {
            Message::LaunchInstanceSelected(selected_instance) => {
                self.select_launch_instance(selected_instance)
            }
            Message::LaunchUsernameSet(username) => self.set_username(username),
            Message::Launch => return self.launch_game(),
            Message::LaunchEnd(result) => self.finish_launching(result),
            Message::CreateInstanceScreen => return self.go_to_create_screen(),
            Message::CreateInstanceVersionsLoaded(result) => {
                self.create_instance_finish_loading_versions_list(result)
            }
            Message::CreateInstanceVersionSelected(selected_version) => {
                self.select_created_instance_version(selected_version)
            }
            Message::CreateInstanceNameInput(name) => self.update_created_instance_name(name),
            Message::CreateInstance => return self.create_instance(),
            Message::CreateInstanceEnd(result) => match result {
                Ok(_) => match Launcher::load() {
                    Ok(launcher) => *self = launcher,
                    Err(err) => self.set_error(err.to_string()),
                },
                Err(n) => self.state = State::Error { error: n },
            },
            Message::CreateInstanceProgressUpdate => self.update_instance_creation_progress_bar(),
            Message::LocateJavaStart => {
                return Command::perform(pick_file(), Message::LocateJavaEnd)
            }
            Message::LocateJavaEnd(path) => self.add_java_to_config(path),
            Message::DeleteInstanceMenu => self.confirm_instance_deletion(),
            Message::DeleteInstance => self.delete_selected_instance(),
            Message::GoToLaunchScreen => self.go_to_launch_screen(),
            Message::EditInstance => {
                if let State::Launch {
                    ref selected_instance,
                    ..
                } = self.state
                {
                    match self.edit_instance(selected_instance.clone().unwrap()) {
                        Ok(_) => {}
                        Err(err) => self.set_error(err.to_string()),
                    }
                }
            }
            Message::EditInstanceJavaOverride(n) => {
                if let State::EditInstance { ref mut config, .. } = self.state {
                    config.java_override = Some(n);
                }
            }
            Message::EditInstanceMemoryChanged(new_slider_value) => {
                if let State::EditInstance {
                    ref mut config,
                    ref mut slider_value,
                    ref mut slider_text,
                    ..
                } = self.state
                {
                    *slider_value = new_slider_value;
                    config.ram_in_mb = 2f32.powf(new_slider_value) as usize;
                    *slider_text = format_memory(config.ram_in_mb);
                }
            }
            Message::EditInstanceSave => {
                if let State::EditInstance {
                    ref selected_instance,
                    ref config,
                    ..
                } = self.state
                {
                    match Launcher::save_config(selected_instance, config) {
                        Ok(_) => self.go_to_launch_screen(),
                        Err(err) => self.set_error(err.to_string()),
                    }
                }
            }
            Message::ManageMods => {
                if let State::Launch {
                    ref selected_instance,
                } = self.state
                {
                    match self.edit_mods(selected_instance.clone().unwrap()) {
                        Ok(_) => {}
                        Err(err) => self.set_error(err.to_string()),
                    }
                }
            }
            Message::InstallFabric => {
                if let State::EditMods {
                    ref selected_instance,
                    ..
                } = self.state
                {
                    self.state = State::InstallFabric {
                        selected_instance: selected_instance.clone(),
                        fabric_version: None,
                        fabric_versions: Vec::new(),
                    };

                    return Command::perform(
                        instance_mod_installer::fabric::get_list_of_versions(),
                        Message::InstallFabricVersionsLoaded,
                    );
                }
            }
            Message::InstallFabricVersionsLoaded(result) => match result {
                Ok(list_of_versions) => {
                    if let State::InstallFabric {
                        ref mut fabric_versions,
                        ..
                    } = self.state
                    {
                        *fabric_versions = list_of_versions
                            .iter()
                            .map(|ver| ver.version.clone())
                            .collect();
                    }
                }
                Err(err) => self.set_error(err),
            },
            Message::InstallFabricVersionSelected(selection) => {
                if let State::InstallFabric {
                    ref mut fabric_version,
                    ..
                } = self.state
                {
                    *fabric_version = Some(selection);
                }
            }
            Message::InstallFabricClicked => {
                if let State::InstallFabric {
                    ref selected_instance,
                    ref fabric_version,
                    ..
                } = self.state
                {
                    return Command::perform(
                        instance_mod_installer::fabric::install_wrapped(
                            fabric_version.clone().unwrap(),
                            selected_instance.to_owned(),
                        ),
                        Message::InstallFabricEnd,
                    );
                }
            }
            Message::InstallFabricEnd(result) => match result {
                Ok(_) => self.go_to_launch_screen(),
                Err(err) => self.set_error(err),
            },
            Message::OpenDir(dir) => match dir.to_str() {
                Some(dir) => open_file_explorer(dir),
                None => self.set_error(LauncherError::PathBufToString(dir).to_string()),
            },
        }
        Command::none()
    }

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        struct Sub;

        const MESSAGE_BUFFER_SIZE: usize = 100;

        if let State::Create {
            progress_reciever: ref progress,
            ..
        } = self.state
        {
            if progress.is_none() {
                return Subscription::none();
            }
            return subscription::channel(
                std::any::TypeId::of::<Sub>(),
                MESSAGE_BUFFER_SIZE,
                |mut output| async move {
                    loop {
                        output
                            .send(Message::CreateInstanceProgressUpdate)
                            .await
                            .unwrap();
                    }
                },
            );
        }
        Subscription::none()
    }

    fn view(&self) -> iced::Element<'_, Self::Message, Self::Theme, iced::Renderer> {
        match self.state {
            State::Launch {
                ref selected_instance,
                ..
            } => self.menu_launch(selected_instance),
            State::Create {
                ref instance_name,
                ref selected_version,
                ref versions,
                ref progress_number,
                ref progress_text,
                ..
            } => self.menu_create(
                progress_number,
                progress_text,
                versions,
                selected_version.as_ref(),
                instance_name,
            ),
            State::Error { ref error } => {
                widget::container(widget::text(format!("Error: {}", error))).into()
            }
            State::FindJavaVersion {
                ref required_version,
                ..
            } => column![
                widget::text(if let Some(ver) = required_version {
                    format!("An installation of Java ({ver}) could not be found",)
                } else {
                    "Required Java Install not found".to_owned()
                }),
                widget::button("Select Java Executable").on_press(Message::LocateJavaStart),
            ]
            .padding(10)
            .spacing(20)
            .into(),
            State::DeleteInstance {
                ref selected_instance,
            } => Launcher::menu_delete(selected_instance),
            State::EditInstance {
                ref selected_instance,
                ref config,
                slider_value,
                ref slider_text,
            } => Launcher::menu_edit(selected_instance, config, slider_value, slider_text),
            State::EditMods { ref config, .. } => {
                let mod_installer = if config.mod_type == "Vanilla" {
                    column![
                        widget::button("Install Fabric").on_press(Message::InstallFabric),
                        widget::button("Install Quilt"),
                        widget::button("Install Forge"),
                        widget::button("Install OptiFine")
                    ]
                    .spacing(5)
                } else {
                    column![widget::button(widget::text(format!(
                        "Uninstall {}",
                        config.mod_type
                    )))]
                };

                column![
                    widget::button("< Back").on_press(Message::GoToLaunchScreen),
                    mod_installer,
                    widget::button("Go to mods folder"),
                    widget::text("Mod management and store coming soon...")
                ]
                .padding(10)
                .spacing(20)
                .into()
            }
            State::InstallFabric {
                ref selected_instance,
                ref fabric_version,
                ref fabric_versions,
            } => column![
                widget::button("< Back").on_press(Message::GoToLaunchScreen),
                widget::text(format!(
                    "Select Fabric Version for instance {}",
                    selected_instance
                )),
                widget::pick_list(
                    fabric_versions.as_slice(),
                    fabric_version.as_ref(),
                    Message::InstallFabricVersionSelected
                ),
                widget::button("Install Fabric").on_press_maybe(
                    fabric_version
                        .is_some()
                        .then(|| Message::InstallFabricClicked)
                ),
            ]
            .padding(10)
            .spacing(20)
            .into(),
        }
    }
}

async fn pick_file() -> Option<PathBuf> {
    const MESSAGE: &str = if cfg!(windows) {
        "Select the java.exe executable"
    } else {
        "Select the java executable"
    };

    rfd::AsyncFileDialog::new()
        .set_title(MESSAGE)
        .pick_file()
        .await
        .map(|n| n.path().to_owned())
}

fn main() {
    const WINDOW_HEIGHT: f32 = 350.0;
    const WINDOW_WIDTH: f32 = 220.0;

    Launcher::run(Settings {
        window: iced::window::Settings {
            size: iced::Size {
                width: WINDOW_WIDTH,
                height: WINDOW_HEIGHT,
            },
            resizable: false,
            ..Default::default()
        },
        ..Default::default()
    })
    .unwrap();
}
