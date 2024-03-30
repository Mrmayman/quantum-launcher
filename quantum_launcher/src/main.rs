use std::{ops::RangeInclusive, path::PathBuf};

use config::LauncherConfig;
use iced::{
    executor, subscription,
    widget::{self, column},
    Application, Command, Settings, Theme,
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
        if let State::Create {
            ref progress,
            ref mut progress_num,
            ..
        } = self.state
        {
            if let Some(ref mut progress_num) = progress_num {
                if let Some(ref progress) = progress {
                    match progress.try_recv() {
                        Ok(progress_message) => {
                            *progress_num = match progress_message {
                                Progress::Started => 0.0,
                                Progress::DownloadingJsonManifest => 0.2,
                                Progress::DownloadingVersionJson => 0.5,
                                Progress::DownloadingAssets { progress, out_of } => {
                                    (progress as f32 / out_of as f32) + 2.0
                                }
                                Progress::DownloadingLibraries { progress, out_of } => {
                                    (progress as f32 / out_of as f32) + 1.0
                                }
                                Progress::DownloadingJar => 1.0,
                                Progress::DownloadingLoggingConfig => 0.7,
                            }
                        }
                        Err(err) => {
                            println!("Err: {err:?}")
                        }
                    }
                }
            }
        }
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
            Message::CreateProgressUpdate(n) => {
                if let State::Create {
                    ref mut progress_num,
                    ..
                } = self.state
                {
                    if let Some(progress) = progress_num {
                        *progress = n
                    }
                }
            }
        }
        Command::none()
    }

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        struct Sub;

        const MESSAGE_BUFFER_SIZE: usize = 100;

        subscription::channel(
            std::any::TypeId::of::<Sub>(),
            MESSAGE_BUFFER_SIZE,
            |mut output| async move {
                loop {
                    let (sender, receiver) =
                        iced::futures::channel::mpsc::channel(MESSAGE_BUFFER_SIZE);
                    println!("Test")
                }
            },
        )
    }

    fn view(&self) -> iced::Element<'_, Self::Message, Self::Theme, iced::Renderer> {
        match self.state {
            State::Launch {
                ref selected_instance,
                ..
            } => menu_renderer::launch(
                self.instances.as_ref().map(|n| n.as_slice()),
                selected_instance,
                &self.config.as_ref().unwrap().username,
            ),
            State::Create {
                ref instance_name,
                ref version,
                ref versions,
                ref progress_num,
                ..
            } => {
                let progress_bar = if let Some(progress_num) = progress_num {
                    column![widget::progress_bar(
                        RangeInclusive::new(0.0, 3.0),
                        *progress_num
                    )]
                } else {
                    column![widget::text("Get ready to enjoy your instance lol.")]
                };

                column![
                    column![
                        widget::text(
                            "Select Instance (only vanilla unmodded Minecraft is supported currently)"
                        ),
                        widget::pick_list(
                            versions.as_slice(),
                            Some(version),
                            Message::CreateInstanceVersionSelected
                        ),
                        ]
                        .spacing(10),
                        widget::text_input("Enter instance name...", instance_name)
                        .on_input(Message::CreateInstanceNameInput),
                        widget::button("Create Instance")
                        .on_press(Message::CreateInstanceStart),
                        progress_bar,
                        widget::progress_bar(RangeInclusive::new(0.0, 1.0), 0.5)
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
            .into(),
        }
    }

    // fn subscription(&self) -> iced::Subscription<Self::Message> {
    //     iced::time::Duration::from_secs_f32(1.0).map
    // }
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
