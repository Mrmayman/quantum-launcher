use iced::{
    executor,
    widget::{self, column},
    Application, Command, Settings, Theme,
};
use launcher_state::{Launcher, Message, State};

mod launcher_state;
mod menu_renderer;

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
                            error: format!("Error: {:?}", n),
                        },
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
            Message::InstanceSelected(n) => {
                if let State::Launch {
                    ref mut selected_instance,
                    ..
                } = self.state
                {
                    *selected_instance = n
                }
            }
            Message::UsernameSet(n) => {
                if let State::Launch {
                    ref mut username, ..
                } = self.state
                {
                    *username = n
                }
            }
            Message::LaunchGame => {
                if let State::Launch {
                    ref mut selected_instance,
                    ref mut username,
                    ..
                } = self.state
                {
                    let selected_instance = selected_instance.clone();
                    let username = username.clone();
                    return Command::perform(
                        quantum_launcher_backend::launch(
                            selected_instance,
                            username,
                            MINECRAFT_MEMORY,
                        ),
                        Message::GameOpened,
                    );
                }
            }
            Message::GameOpened(n) => {
                if let Err(err) = n {
                    self.state = State::Error { error: err }
                }
            }
            Message::CreateInstance => {
                self.state = State::Create {
                    instance_name: Default::default(),
                    version: Default::default(),
                    versions: Vec::new(),
                };
                return Command::perform(
                    quantum_launcher_backend::list_versions(),
                    Message::CreateInstanceLoaded,
                );
            }
            Message::CreateInstanceLoaded(result) => match result {
                Ok(version_list) => {
                    if let State::Create {
                        ref mut versions, ..
                    } = self.state
                    {
                        versions.extend_from_slice(&version_list)
                    }
                }
                Err(n) => self.state = State::Error { error: n },
            },
            Message::CreateSelectedVersion(n) => {
                if let State::Create {
                    ref mut version, ..
                } = self.state
                {
                    *version = n
                }
            }
            Message::CreateInputName(n) => {
                if let State::Create {
                    ref mut instance_name,
                    ..
                } = self.state
                {
                    *instance_name = n
                }
            }
        }
        Command::none()
    }

    fn view(&self) -> iced::Element<'_, Self::Message, Self::Theme, iced::Renderer> {
        match self.state {
            State::Launch {
                ref instances,
                ref selected_instance,
                ref username,
            } => menu_renderer::launch(instances.as_slice(), selected_instance, &username),
            State::Create {
                ref instance_name,
                ref version,
                ref versions,
            } => column![
                column![
                    widget::text("Select Instance"),
                    widget::pick_list(
                        versions.as_slice(),
                        Some(version),
                        Message::CreateSelectedVersion
                    ),
                ],
                widget::text_input("Enter input", instance_name).on_input(Message::CreateInputName)
            ]
            .into(),
            State::Error { ref error } => {
                widget::container(widget::text(format!("Error: {}", error))).into()
            }
        }
    }
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
