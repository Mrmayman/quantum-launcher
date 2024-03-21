use iced::widget::{self, column};

use crate::launcher_state::{Launcher, Message};

pub fn launch<'element>(
    instances: &'element [String],
    selected_instance: &'element String,
    username: &str,
) -> iced::Element<'element, Message, <Launcher as iced::Application>::Theme, iced::Renderer> {
    column![
        column![
            widget::text("Instances:"),
            widget::pick_list(
                instances,
                Some(selected_instance),
                Message::InstanceSelected,
            ),
            widget::button("Create Instance").on_press(Message::CreateInstance)
        ]
        .spacing(5),
        column![
            widget::text_input("Enter username...", username).on_input(Message::UsernameSet),
            widget::button("Launch game").on_press(Message::LaunchGame)
        ]
        .spacing(5)
    ]
    .padding(10)
    .spacing(40)
    .into()
}
