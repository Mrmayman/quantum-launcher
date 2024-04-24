use std::ops::RangeInclusive;

use iced::widget::{self, column, row};
use quantum_launcher_backend::file_utils;

use crate::{
    config::LauncherConfig,
    icon_manager,
    launcher_state::{Launcher, MenuEditInstance, MenuLaunch, Message},
    stylesheet::styles::LauncherTheme,
};

pub type Element<'a> =
    iced::Element<'a, Message, <Launcher as iced::Application>::Theme, iced::Renderer>;

fn button_with_icon<'element>(
    icon: Element<'element>,
    text: &'element str,
) -> iced::widget::Button<'element, Message, LauncherTheme> {
    widget::button(row![icon, text].spacing(10).padding(5)).width(200)
}

impl MenuLaunch {
    pub fn view<'element>(
        &'element self,
        config: Option<&'element LauncherConfig>,
        instances: Option<&'element [String]>,
    ) -> Element<'element> {
        let pick_list = if let Some(instances) = instances {
            column![
                widget::text("Instances:"),
                widget::pick_list(
                    instances,
                    self.selected_instance.as_ref(),
                    Message::LaunchInstanceSelected,
                )
                .width(200),
                button_with_icon(icon_manager::create(), "New Instance")
                    .on_press(Message::CreateInstanceScreen),
                button_with_icon(icon_manager::delete(), "Delete Instance").on_press_maybe(
                    (self.selected_instance.is_some()).then_some(Message::DeleteInstanceMenu)
                ),
                button_with_icon(icon_manager::settings(), "Settings").on_press_maybe(
                    (self.selected_instance.is_some()).then_some(Message::EditInstance)
                ),
                button_with_icon(icon_manager::download(), "Manage Mods").on_press_maybe(
                    (self.selected_instance.is_some()).then_some(Message::ManageMods)
                ),
                button_with_icon(icon_manager::folder(), "Open Files").on_press_maybe(
                    (self.selected_instance.is_some()).then(|| {
                        let launcher_dir = file_utils::get_launcher_dir().unwrap();
                        Message::OpenDir(
                            launcher_dir
                                .join("instances")
                                .join(self.selected_instance.as_ref().unwrap())
                                .join(".minecraft"),
                        )
                    })
                )
            ]
        } else {
            column![widget::text("Loading instances...")]
        };

        column![
            column![
                widget::text("Username:"),
                widget::text_input("Enter username...", &config.as_ref().unwrap().username)
                    .on_input(Message::LaunchUsernameSet)
                    .width(200),
            ]
            .spacing(5),
            pick_list.spacing(5),
            button_with_icon(icon_manager::play(), "Launch Game")
                .on_press_maybe((self.selected_instance.is_some()).then_some(Message::Launch))
        ]
        .padding(10)
        .spacing(20)
        .into()
    }
}

impl MenuEditInstance {
    pub fn view<'element>(&self) -> Element<'element> {
        // 2 ^ 8 = 256 MB
        const MEM_256_MB_IN_TWOS_EXPONENT: f32 = 8.0;
        // 2 ^ 13 = 8192 MB
        const MEM_8192_MB_IN_TWOS_EXPONENT: f32 = 13.0;

        widget::scrollable(
            column![
                widget::button(row![icon_manager::back(), widget::text("Back")]
                    .spacing(10)
                    .padding(5)
                ).on_press(Message::GoToLaunchScreen),
                widget::text(format!("Editing {} instance: {}", self.config.mod_type, self.selected_instance)),
                widget::container(
                    column![
                        widget::text("Use a special Java install instead of the default one. (Enter path, leave blank if none)"),
                        widget::text_input(
                            "Enter Java override",
                            self.config
                                .java_override
                                .as_deref()
                                .unwrap_or_default()
                        )
                        .on_input(Message::EditInstanceJavaOverride)
                    ]
                    .padding(10)
                    .spacing(10)
                ),
                widget::container(
                    column![
                        widget::text("Allocated memory"),
                        widget::text("For normal Minecraft, allocate 2 - 3 GB"),
                        widget::text("For old versions, allocate 512 MB - 1 GB"),
                        widget::text("For heavy modpacks or very high render distances, allocate 4 - 8 GB"),
                        widget::slider(MEM_256_MB_IN_TWOS_EXPONENT..=MEM_8192_MB_IN_TWOS_EXPONENT, self.slider_value, Message::EditInstanceMemoryChanged).step(0.1),
                        widget::text(&self.slider_text),
                    ]
                    .padding(10)
                    .spacing(5),
                ),
                widget::button(row![icon_manager::save(), widget::text("Save")]
                    .spacing(10)
                    .padding(5)
                ).on_press(Message::EditInstanceSave),
            ]
            .padding(10)
            .spacing(20)
        ).into()
    }
}

impl Launcher {
    pub fn menu_create<'element>(
        progress_number: &Option<f32>,
        progress_text: &Option<String>,
        versions: &'element Vec<String>,
        version: Option<&'element String>,
        instance_name: &str,
    ) -> Element<'element> {
        let progress_bar = if let Some(progress_number) = progress_number {
            if let Some(progress_text) = progress_text {
                column![
                    widget::progress_bar(RangeInclusive::new(0.0, 10.0), *progress_number),
                    widget::text(progress_text),
                ]
            } else {
                column![]
            }
        } else {
            column![]
        };

        widget::scrollable(
            column![
                widget::button(
                    row![icon_manager::back(), widget::text("Back")]
                        .spacing(10)
                        .padding(5)
                ).on_press(Message::GoToLaunchScreen),
                column![
                    widget::text("Select Version"),
                    widget::text("To install Fabric/Forge/OptiFine/Quilt, click on Manage Mods after installing the instance"),
                    widget::pick_list(
                        versions.as_slice(),
                        version,
                        Message::CreateInstanceVersionSelected
                    ),
                ]
                .spacing(10),
                widget::text_input("Enter instance name...", instance_name)
                    .on_input(Message::CreateInstanceNameInput),
                widget::button(row![icon_manager::create(), widget::text("Create Instance")]
                        .spacing(10)
                        .padding(5)
                ).on_press_maybe((version.is_some() && !instance_name.is_empty()).then(|| Message::CreateInstance)),
                progress_bar,
            ]
            .spacing(10)
            .padding(10),
        )
        .into()
    }

    pub fn menu_delete(selected_instance: &str) -> Element {
        column![
            widget::text(format!(
                "Are you SURE you want to DELETE the Instance: {selected_instance}?",
            )),
            widget::text("All your data, including worlds will be lost."),
            widget::button("Yes, delete my data").on_press(Message::DeleteInstance),
            widget::button("No").on_press(Message::GoToLaunchScreen),
        ]
        .padding(10)
        .spacing(10)
        .into()
    }

    pub fn menu_find_java(required_version: &Option<usize>) -> Element {
        column![
            widget::text(if let Some(ver) = required_version {
                format!("An installation of Java ({ver}) could not be found",)
            } else {
                "Required Java Install not found".to_owned()
            }),
            widget::button("Select Java Executable").on_press(Message::LocateJavaStart),
        ]
        .padding(10)
        .spacing(20)
        .into()
    }
}
