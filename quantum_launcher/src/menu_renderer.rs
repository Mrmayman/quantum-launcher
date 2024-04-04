use std::ops::RangeInclusive;

use iced::widget::{self, column};

use crate::launcher_state::{Launcher, Message};

impl Launcher {
    pub fn menu_launch<'element>(
        &'element self,
        selected_instance: &'element String,
    ) -> iced::Element<'element, Message, <Launcher as iced::Application>::Theme, iced::Renderer>
    {
        let pick_list = if let Some(instances) = self.instances.as_deref() {
            column![
                widget::text("Instances:"),
                widget::pick_list(
                    instances,
                    Some(selected_instance),
                    Message::LaunchInstanceSelected,
                ),
                widget::button("Create Instance").on_press(Message::CreateInstanceScreen),
                widget::button("Delete Selected Instance").on_press(Message::DeleteInstanceMenu),
            ]
        } else {
            column![widget::text("Loading instances...")]
        };

        column![
            pick_list.spacing(5),
            column![
                widget::text_input("Enter username...", &self.config.as_ref().unwrap().username)
                    .on_input(Message::LaunchUsernameSet),
                widget::button("Launch game").on_press(Message::Launch)
            ]
            .spacing(5)
        ]
        .padding(10)
        .spacing(20)
        .into()
    }

    pub fn menu_create<'element>(
        &'element self,
        progress_number: &Option<f32>,
        progress_text: &Option<String>,
        versions: &'element Vec<String>,
        version: &'element String,
        instance_name: &String,
    ) -> iced::Element<Message, <Launcher as iced::Application>::Theme, iced::Renderer> {
        let progress_bar = if let Some(progress_number) = progress_number {
            if let Some(progress_text) = progress_text {
                column![
                    widget::progress_bar(RangeInclusive::new(0.0, 10.0), *progress_number),
                    widget::text(progress_text),
                ]
            } else {
                column![widget::text("Happy Gaming!")]
            }
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
            widget::button("Create Instance").on_press(Message::CreateInstance),
            progress_bar,
        ]
        .spacing(20)
        .padding(10)
        .into()
    }
}
