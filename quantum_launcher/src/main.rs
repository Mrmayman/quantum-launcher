use std::fs;

use iced::{
    executor,
    widget::{self, column},
    Application, Command, Settings, Theme,
};
use quantum_launcher_backend::error::LauncherResult;

const MINECRAFT_MEMORY: &str = "2G";

#[derive(Debug, Clone)]
enum LauncherMessage {
    InstanceSelected(String),
    UsernameSet(String),
    LaunchGame,
    GameOpened(Option<String>),
}

struct LauncherState {
    instances: Vec<String>,
    selected_instance: String,
    error: Option<String>,
    username: String,
}

impl LauncherState {
    pub fn load() -> LauncherResult<Self> {
        let dir = quantum_launcher_backend::file_utils::get_launcher_dir()?;
        let dir = fs::read_dir(dir.join("instances"))?;

        let subdirectories: Vec<String> = dir
            .filter_map(|entry| {
                if let Ok(entry) = entry {
                    if entry.path().is_dir() {
                        if let Some(file_name) = entry.file_name().to_str() {
                            return Some(file_name.to_owned());
                        }
                    }
                }
                None
            })
            .collect();

        Ok(Self {
            instances: subdirectories,
            selected_instance: Default::default(),
            username: Default::default(),
            error: None,
        })
    }
}

impl Application for LauncherState {
    type Executor = executor::Default;
    type Message = LauncherMessage;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, iced::Command<Self::Message>) {
        let state = match LauncherState::load() {
            Ok(n) => n,
            Err(n) => {
                return (
                    Self {
                        instances: Vec::new(),
                        error: Some(format!("{:?}", n)),
                        selected_instance: Default::default(),
                        username: Default::default(),
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
            LauncherMessage::InstanceSelected(n) => self.selected_instance = n,
            LauncherMessage::UsernameSet(n) => self.username = n,
            LauncherMessage::LaunchGame => {
                let selected_instance = self.selected_instance.clone();
                let username = self.username.clone();
                return Command::perform(
                    quantum_launcher_backend::instance::launch(
                        selected_instance,
                        username,
                        MINECRAFT_MEMORY,
                    ),
                    LauncherMessage::GameOpened,
                );
            }
            LauncherMessage::GameOpened(n) => {
                if let Some(err) = n {
                    self.error = Some(err)
                }
            }
        }
        Command::none()
    }

    fn view(&self) -> iced::Element<'_, Self::Message, Self::Theme, iced::Renderer> {
        let version_list = widget::pick_list(
            self.instances.as_slice(),
            Some(&self.selected_instance),
            LauncherMessage::InstanceSelected,
        );

        let username_input = widget::text_input("Enter username...", &self.username)
            .on_input(LauncherMessage::UsernameSet);
        let column = if let Some(ref err) = self.error {
            column![
                version_list,
                username_input,
                widget::text(format!("Error: {:?}", err.clone())),
            ]
        } else {
            column![
                version_list,
                username_input,
                widget::button("Launch game").on_press(LauncherMessage::LaunchGame)
            ]
        }
        .padding(10)
        .spacing(10)
        .into();
        column
    }
}

fn main() {
    LauncherState::run(Settings::default()).unwrap();
}
