use std::{ops::RangeInclusive, path::PathBuf};

use config::LauncherConfig;
use iced::{
    executor,
    futures::SinkExt,
    subscription,
    widget::{self, column},
    Application, Command, Settings, Subscription, Theme,
};
use launcher_state::{Launcher, Message, State};
use quantum_launcher_backend::download::Progress;

mod config;
mod launcher_state;
mod menu_renderer;
mod message_handler;

const MINECRAFT_MEMORY: &str = "2G";

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
                            error: format!("Error: {}", n),
                        },
                        instances: None,
                        config: LauncherConfig::load().ok(),
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
            Message::LaunchInstanceSelected(n) => self.m_launch_instance_selected(n),
            Message::LaunchUsernameSet(n) => self.m_launch_username_set(n),
            Message::LaunchStart => return self.m_launch_start(),
            Message::LaunchEnd(n) => self.m_launch_end(n),
            Message::CreateInstance => return self.m_create(),
            Message::CreateInstanceVersionsLoaded(result) => self.m_create_versions_loaded(result),
            Message::CreateInstanceVersionSelected(n) => {
                if let State::Create {
                    ref mut version, ..
                } = self.state
                {
                    *version = n
                }
            }
            Message::CreateInstanceNameInput(n) => {
                if let State::Create {
                    ref mut instance_name,
                    ..
                } = self.state
                {
                    *instance_name = n
                }
            }
            Message::CreateInstanceStart => {
                if let State::Create {
                    ref instance_name,
                    ref version,
                    ref mut progress,
                    ref mut progress_num,
                    ..
                } = self.state
                {
                    let (sender, receiver) = std::sync::mpsc::channel::<Progress>();
                    *progress = Some(receiver);
                    *progress_num = Some(0.0);
                    return Command::perform(
                        quantum_launcher_backend::create_instance(
                            instance_name.to_owned(),
                            version.to_owned(),
                            Some(sender),
                        ),
                        Message::CreateInstanceEnd,
                    );
                }
            }
            Message::CreateInstanceEnd(result) => match result {
                Ok(_) => match Launcher::load() {
                    Ok(launcher) => *self = launcher,
                    Err(err) => self.set_error(err.to_string()),
                },
                Err(n) => self.state = State::Error { error: n },
            },
            Message::LocateJavaStart => {
                return Command::perform(pick_file(), Message::LocateJavaEnd)
            }
            Message::LocateJavaEnd(path) => match path {
                Some(path) => match self.config {
                    Some(ref mut config) => match path.to_str() {
                        Some(path) => {
                            config.java_installs.push(path.to_owned());
                            match config.save() {
                                Ok(_) => {}
                                Err(err) => self.set_error(err.to_string()),
                            }
                            self.state = State::Launch {
                                selected_instance: "".to_owned(),
                                spawned_process: None,
                            }
                        }
                        None => self
                            .set_error("Selected Java path contains invalid characters".to_owned()),
                    },
                    None => self.set_error(
                        "Could not open launcher config at QuantumLauncher/launcher.config"
                            .to_owned(),
                    ),
                },
                None => self.set_error("Selected Java path not found.".to_owned()),
            },
            Message::CreateProgressUpdate => {
                if let State::Create {
                    ref mut progress_num,
                    ref progress,
                    ..
                } = self.state
                {
                    if let Some(progress) = progress {
                        if let Ok(progress) = progress.try_recv() {
                            if let Some(progress_num) = progress_num {
                                *progress_num = match progress {
                                    Progress::Started => 0.0,
                                    Progress::DownloadingJsonManifest => 0.2,
                                    Progress::DownloadingVersionJson => 0.5,
                                    Progress::DownloadingAssets {
                                        progress: progress_num,
                                        out_of,
                                    } => (progress_num as f32 * 2.0 / out_of as f32) + 2.0,
                                    Progress::DownloadingLibraries {
                                        progress: progress_num,
                                        out_of,
                                    } => (progress_num as f32 / out_of as f32) + 1.0,
                                    Progress::DownloadingJar => 1.0,
                                    Progress::DownloadingLoggingConfig => 0.7,
                                }
                            }
                        }
                    }
                }
            }
            Message::LaunchDeleteStart => {
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
            Message::LaunchDeleteEnd => {
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
                                    self.state = State::Launch {
                                        selected_instance: Default::default(),
                                        spawned_process: None,
                                    }
                                }
                            } else {
                                self.set_error("Tried to delete instance folder located outside Launcher. Potential attack avoided.".to_owned())
                            }
                        }
                        Err(err) => self.set_error(err.to_string()),
                    }
                }
            }
            Message::LaunchDeleteCancel => {
                self.state = State::Launch {
                    selected_instance: Default::default(),
                    spawned_process: None,
                }
            }
        }
        Command::none()
    }

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        struct Sub;

        const MESSAGE_BUFFER_SIZE: usize = 100;

        if let State::Create { ref progress, .. } = self.state {
            if progress.is_none() {
                return Subscription::none();
            }
            return subscription::channel(
                std::any::TypeId::of::<Sub>(),
                MESSAGE_BUFFER_SIZE,
                |mut output| async move {
                    loop {
                        output.send(Message::CreateProgressUpdate).await.unwrap();
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
                ref version,
                ref versions,
                ref progress_num,
                ..
            } => {
                let progress_bar = if let Some(progress_num) = progress_num {
                    column![widget::progress_bar(
                        RangeInclusive::new(0.0, 4.0),
                        *progress_num
                    )]
                } else {
                    column![widget::text("Happy Gaming!")]
                };

                column![
                    column![
                        widget::text("Select Version (Fabric/Forge/Optifine coming soon)"),
                        widget::pick_list(
                            versions.as_slice(),
                            Some(version),
                            Message::CreateInstanceVersionSelected
                        ),
                    ]
                    .spacing(10),
                    widget::text_input("Enter instance name...", instance_name)
                        .on_input(Message::CreateInstanceNameInput),
                    widget::button("Create Instance").on_press(Message::CreateInstanceStart),
                    progress_bar,
                ]
                .spacing(20)
                .padding(10)
                .into()
            }
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
            } => column![
                widget::text(format!(
                    "Are you SURE you want to DELETE the Instance {}?",
                    selected_instance
                )),
                widget::text("All your data, including worlds will be lost."),
                widget::button("Yes, delete my data").on_press(Message::LaunchDeleteEnd),
                widget::button("No").on_press(Message::LaunchDeleteCancel),
            ]
            .padding(10)
            .spacing(10)
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
    const WINDOW_HEIGHT: f32 = 600.0;
    const WINDOW_WIDTH: f32 = 600.0;

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
