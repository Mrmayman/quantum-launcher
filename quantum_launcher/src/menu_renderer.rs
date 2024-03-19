use iced::widget::{self, column};

use crate::launcher_state::{Launcher, Message};

pub fn launch<'element>(
    instances: &'element [String],
    selected_instance: &'element String,
    username: &str,
) -> iced::Element<'element, Message, <Launcher as iced::Application>::Theme, iced::Renderer> {
    const USERNAME_INPUT_MESSAGE: &str = "Enter username...";

    let version_list = widget::pick_list(
        instances,
        Some(selected_instance),
        Message::InstanceSelected,
    );

    let username_input =
        widget::text_input(USERNAME_INPUT_MESSAGE, username).on_input(Message::UsernameSet);

    column![
        version_list,
        username_input,
        widget::button("Launch game").on_press(Message::LaunchGame)
    ]
    .padding(10)
    .spacing(10)
    .into()
}
