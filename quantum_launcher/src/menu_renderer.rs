use iced::widget::{self, column};

use crate::launcher_state::{Launcher, Message};

pub fn launch<'element>(
    instances: Option<&'element [String]>,
    selected_instance: &'element String,
    username: &str,
) -> iced::Element<'element, Message, <Launcher as iced::Application>::Theme, iced::Renderer> {
    let pick_list = if let Some(instances) = instances {
        column![
            widget::text("Instances:"),
            widget::pick_list(
                instances,
                Some(selected_instance),
                Message::LaunchInstanceSelected,
            ),
            widget::button("Create Instance").on_press(Message::CreateInstance)
        ]
    } else {
        column![widget::text("Loading instances...")]
    };

    column![
        pick_list.spacing(5),
        column![
            widget::text_input("Enter username...", username).on_input(Message::LaunchUsernameSet),
            widget::button("Launch game").on_press(Message::LaunchStart)
        ]
        .spacing(5)
    ]
    .padding(10)
    .spacing(20)
    .into()
}
