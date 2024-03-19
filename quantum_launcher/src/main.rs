use iced::{executor, widget, Application, Command, Settings, Theme};
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
        }
        Command::none()
    }

    fn view(&self) -> iced::Element<'_, Self::Message, Self::Theme, iced::Renderer> {
        const USERNAME_INPUT_MESSAGE: &str = "Enter username...";

        match self.state {
            State::Launch {
                ref instances,
                ref selected_instance,
                ref username,
            } => menu_renderer::launch(instances.as_slice(), selected_instance, &username),
            State::Create {
                ref instance_name,
                ref version,
            } => todo!(),
            State::Error { ref error } => {
                widget::container(widget::text(format!("Error: {}", error))).into()
            }
        }
    }
}

fn main() {
    Launcher::run(Settings::default()).unwrap();
}
