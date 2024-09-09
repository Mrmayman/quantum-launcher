use std::{
    collections::HashMap,
    ops::RangeInclusive,
    process::Child,
    sync::{Arc, Mutex},
};

use iced::widget::{self, column, row};
use ql_instances::file_utils;

use crate::{
    config::LauncherConfig,
    icon_manager,
    launcher_state::{
        Launcher, MenuCreateInstance, MenuDeleteInstance, MenuEditInstance, MenuEditMods,
        MenuInstallFabric, MenuLaunch, Message,
    },
    stylesheet::styles::LauncherTheme,
};

pub type Element<'a> =
    iced::Element<'a, Message, <Launcher as iced::Application>::Theme, iced::Renderer>;

fn button_with_icon<'element>(
    icon: Element<'element>,
    text: &'element str,
) -> iced::widget::Button<'element, Message, LauncherTheme> {
    widget::button(row![icon, text].spacing(10).padding(5))
}

impl MenuLaunch {
    pub fn view<'element>(
        &'element self,
        config: Option<&'element LauncherConfig>,
        instances: Option<&'element [String]>,
        processes: &'element HashMap<String, Arc<Mutex<Child>>>,
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
                widget::row![
                    button_with_icon(icon_manager::create(), "New")
                        .on_press(Message::CreateInstanceScreenOpen)
                        .width(97),
                    button_with_icon(icon_manager::delete(), "Delete")
                        .on_press_maybe(
                            (self.selected_instance.is_some())
                                .then_some(Message::DeleteInstanceMenu)
                        )
                        .width(98),
                ]
                .spacing(5),
                widget::row![
                    button_with_icon(icon_manager::settings(), "Edit")
                        .on_press_maybe(
                            (self.selected_instance.is_some()).then_some(Message::EditInstance)
                        )
                        .width(97),
                    button_with_icon(icon_manager::download(), "Mods")
                        .on_press_maybe(
                            (self.selected_instance.is_some())
                                .then_some(Message::ManageModsScreenOpen)
                        )
                        .width(98),
                ]
                .spacing(5),
            ]
        } else {
            column![widget::text("Loading instances...")]
        };

        let java_progress_bar = if let Some(progress) = &self.java_install_progress {
            widget::column!(
                widget::progress_bar(0.0..=1.0, progress.num),
                widget::text(&progress.message)
            )
        } else {
            let version_message =
                widget::text("QuantumLauncher v0.1\nA Minecraft Launcher\nby Mrmayman");
            if self.message.is_empty() {
                widget::column!(version_message)
            } else {
                widget::column!(
                    widget::container(widget::text(&self.message)).padding(10),
                    version_message
                )
            }
            .spacing(10)
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
            widget::row![
                button_with_icon(icon_manager::folder(), "Files")
                    .on_press_maybe((self.selected_instance.is_some()).then(|| {
                        let launcher_dir = file_utils::get_launcher_dir().unwrap();
                        Message::OpenDir(
                            launcher_dir
                                .join("instances")
                                .join(self.selected_instance.as_ref().unwrap())
                                .join(".minecraft"),
                        )
                    }))
                    .width(97),
                button_with_icon(icon_manager::play(), "Play")
                    .on_press_maybe(
                        {
                            if let Some(selected_instance) = &self.selected_instance {
                                !processes.contains_key(selected_instance)
                            } else {
                                false
                            }
                        }
                        .then_some(Message::LaunchStart)
                    )
                    .width(98),
            ]
            .spacing(5),
            java_progress_bar
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
                ).on_press(Message::LaunchScreenOpen(None)),
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
            ]
            .padding(10)
            .spacing(20)
        ).into()
    }
}

impl MenuEditMods {
    pub fn view(&self) -> Element {
        let mod_installer = if self.config.mod_type == "Vanilla" {
            widget::column![
                widget::button("Install Fabric").on_press(Message::InstallFabricScreenOpen),
                widget::button("Install Quilt"),
                widget::button("Install Forge"),
                widget::button("Install OptiFine")
            ]
            .spacing(5)
        } else {
            widget::column![widget::button(
                widget::row![
                    icon_manager::delete(),
                    widget::text(format!("Uninstall {}", self.config.mod_type))
                ]
                .spacing(10)
                .padding(5)
            )
            .on_press_maybe(
                (self.config.mod_type == "Fabric").then_some(Message::UninstallLoaderStart)
            )]
        };

        widget::column![
            widget::button(
                widget::row![icon_manager::back(), widget::text("Back")]
                    .spacing(10)
                    .padding(5)
            )
            .on_press(Message::LaunchScreenOpen(None)),
            mod_installer,
            widget::button("Go to mods folder"),
            widget::text("Mod management and store coming soon...")
        ]
        .padding(10)
        .spacing(20)
        .into()
    }
}

impl MenuCreateInstance {
    pub fn view(&self) -> Element {
        let progress_bar = if let Some(progress_number) = self.progress_number {
            if let Some(progress_text) = &self.progress_text {
                column![
                    widget::progress_bar(RangeInclusive::new(0.0, 10.0), progress_number),
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
                ).on_press(Message::LaunchScreenOpen(None)),
                column![
                    widget::text("To install Fabric/Forge/OptiFine/Quilt, click on Manage Mods after installing the instance"),
                    widget::text("Select Version"),
                    widget::pick_list(
                        self.versions.as_slice(),
                        self.selected_version.as_ref(),
                        Message::CreateInstanceVersionSelected
                    ),
                ]
                .spacing(10),
                widget::text_input("Enter instance name...", &self.instance_name)
                    .on_input(Message::CreateInstanceNameInput),
                widget::text("Download assets? If disabled, creating instance will be MUCH faster, but no sound or music will play in-game"),
                widget::checkbox("Download assets?", self.download_assets).on_toggle(Message::CreateInstanceChangeAssetToggle),
                widget::button(row![icon_manager::create(), widget::text("Create Instance")]
                        .spacing(10)
                        .padding(5)
                ).on_press_maybe((self.selected_version.is_some() && !self.instance_name.is_empty()).then(|| Message::CreateInstanceStart)),
                progress_bar,
            ]
            .spacing(10)
            .padding(10),
        )
        .into()
    }
}

impl MenuDeleteInstance {
    pub fn view(&self) -> Element {
        column![
            widget::text(format!(
                "Are you SURE you want to DELETE the Instance: {}?",
                &self.selected_instance
            )),
            widget::text("All your data, including worlds will be lost."),
            widget::button("Yes, delete my data").on_press(Message::DeleteInstance),
            widget::button("No").on_press(Message::LaunchScreenOpen(None)),
        ]
        .padding(10)
        .spacing(10)
        .into()
    }
}

impl MenuInstallFabric {
    pub fn view(&self) -> Element {
        if self.progress_receiver.is_some() {
            column!(
                widget::text("Installing Fabric..."),
                widget::progress_bar(0.0..=1.0, self.progress_num)
            )
            .padding(10)
            .spacing(20)
            .into()
        } else {
            column![
                widget::button(
                    row![icon_manager::back(), widget::text("Back")]
                        .spacing(10)
                        .padding(5)
                )
                .on_press(Message::LaunchScreenOpen(None)),
                widget::text(format!(
                    "Select Fabric Version for instance {}",
                    &self.selected_instance
                )),
                widget::pick_list(
                    self.fabric_versions.as_slice(),
                    self.fabric_version.as_ref(),
                    Message::InstallFabricVersionSelected
                ),
                widget::button("Install Fabric").on_press_maybe(
                    self.fabric_version
                        .is_some()
                        .then(|| Message::InstallFabricClicked)
                ),
            ]
            .padding(10)
            .spacing(20)
            .into()
        }
    }
}
