use std::ops::RangeInclusive;

use iced::widget::{self, column};
use quantum_launcher_backend::{
    file_utils, json_structs::json_instance_config::InstanceConfigJson,
};

use crate::{
    l10n,
    launcher_state::{Launcher, Message},
};

type Element<'a> =
    iced::Element<'a, Message, <Launcher as iced::Application>::Theme, iced::Renderer>;

impl Launcher {
    pub fn menu_launch<'element>(
        &'element self,
        selected_instance: &'element Option<String>,
    ) -> Element<'element> {
        let pick_list = if let Some(instances) = self.instances.as_deref() {
            column![
                widget::text("Instances:"),
                widget::pick_list(
                    instances,
                    selected_instance.as_ref(),
                    Message::LaunchInstanceSelected,
                )
                .width(200),
                widget::button("+ New Instance")
                    .on_press(Message::CreateInstanceScreen)
                    .width(200),
                widget::button("× Delete Instance")
                    .on_press_maybe(
                        (selected_instance.is_some()).then_some(Message::DeleteInstanceMenu)
                    )
                    .width(200),
                widget::button("⚙️ Settings")
                    .on_press_maybe((selected_instance.is_some()).then_some(Message::EditInstance))
                    .width(200),
                widget::button("✏️ Manage Mods")
                    .on_press_maybe((selected_instance.is_some()).then_some(Message::ManageMods))
                    .width(200),
                widget::button("> Open Files")
                    .on_press_maybe((selected_instance.is_some()).then(|| {
                        let launcher_dir = file_utils::get_launcher_dir().unwrap();
                        Message::OpenDir(
                            launcher_dir
                                .join("instances")
                                .join(selected_instance.as_ref().unwrap())
                                .join(".minecraft"),
                        )
                    }))
                    .width(200)
            ]
        } else {
            column![widget::text("Loading instances...")]
        };

        column![
            pick_list.spacing(5),
            column![
                widget::text_input("Enter username...", &self.config.as_ref().unwrap().username)
                    .on_input(Message::LaunchUsernameSet)
                    .width(200),
                widget::button("~ Launch game")
                    .on_press_maybe((selected_instance.is_some()).then_some(Message::Launch))
                    .width(200)
            ]
            .spacing(5)
        ]
        .padding(10)
        .spacing(20)
        .into()
    }

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
                widget::button("< Back").on_press(Message::GoToLaunchScreen),
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
                widget::button("+ Create Instance")
                    .on_press_maybe(version.is_some().then(|| Message::CreateInstance)),
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
                "{}: {selected_instance}?",
                l10n!(ENGLISH, AreYouSUREYouWantToDeleteTheInstance),
            )),
            widget::text(l10n!(ENGLISH, AllYourDataIncludingWorldsWillBeLost)),
            widget::button(l10n!(ENGLISH, YesDeleteMyData)).on_press(Message::DeleteInstance),
            widget::button(l10n!(ENGLISH, No)).on_press(Message::GoToLaunchScreen),
        ]
        .padding(10)
        .spacing(10)
        .into()
    }

    pub fn menu_edit<'element>(
        selected_instance: &'element str,
        config: &InstanceConfigJson,
        slider_value: f32,
        slider_text: &str,
    ) -> Element<'element> {
        // 2 ^ 8 = 256 MB
        const MEM_256_MB_IN_TWOS_EXPONENT: f32 = 8.0;
        // 2 ^ 13 = 8192 MB
        const MEM_8192_MB_IN_TWOS_EXPONENT: f32 = 13.0;

        widget::scrollable(
            column![
                widget::button("< Back").on_press(Message::GoToLaunchScreen),
                widget::text(format!("Editing {} instance: {}", config.mod_type, selected_instance)),
                widget::container(
                    column![
                        widget::text("Use a special Java install instead of the default one. (Enter path, leave blank if none)"),
                        widget::text_input(
                            "Enter Java override",
                            config
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
                        widget::slider(MEM_256_MB_IN_TWOS_EXPONENT..=MEM_8192_MB_IN_TWOS_EXPONENT, slider_value, Message::EditInstanceMemoryChanged).step(0.1),
                        widget::text(slider_text),
                    ]
                    .padding(10)
                    .spacing(5),
                ),
                widget::button("Save").on_press(Message::EditInstanceSave),
            ]
            .padding(10)
            .spacing(20)
        ).into()
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
