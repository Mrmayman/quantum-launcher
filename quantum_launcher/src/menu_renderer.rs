use iced::widget::{self, column};

use crate::launcher_state::{Launcher, Message};

impl Launcher {
    pub fn menu_launch<'element>(
        &'element self,
        selected_instance: &'element String,
    ) -> iced::Element<'element, Message, <Launcher as iced::Application>::Theme, iced::Renderer>
    {
        let pick_list = if let Some(instances) = self.instances.as_ref().map(|n| n.as_slice()) {
            column![
                widget::text("Instances:"),
                widget::pick_list(
                    instances,
                    Some(selected_instance),
                    Message::LaunchInstanceSelected,
                ),
                widget::button("Create Instance").on_press(Message::CreateInstance),
                widget::button("Delete Selected Instance").on_press(Message::LaunchDeleteStart),
            ]
        } else {
            column![widget::text("Loading instances...")]
        };

        column![
            pick_list.spacing(5),
            column![
                widget::text_input("Enter username...", &self.config.as_ref().unwrap().username)
                    .on_input(Message::LaunchUsernameSet),
                widget::button("Launch game").on_press(Message::LaunchStart)
            ]
            .spacing(5)
        ]
        .padding(10)
        .spacing(20)
        .into()
    }
}
