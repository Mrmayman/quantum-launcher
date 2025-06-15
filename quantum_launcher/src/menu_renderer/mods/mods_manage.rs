use iced::{widget, Length};
use ql_core::{InstanceSelection, SelectedMod};

use crate::{
    icon_manager,
    menu_renderer::{back_button, back_to_launch_screen, button_with_icon, Element},
    state::{
        EditPresetsMessage, InstallFabricMessage, InstallModsMessage, InstallOptifineMessage,
        ManageJarModsMessage, ManageModsMessage, MenuEditMods, Message, ModListEntry,
        SelectedState,
    },
    stylesheet::{color::Color, styles::LauncherTheme},
};

impl MenuEditMods {
    pub fn view<'a>(
        &'a self,
        selected_instance: &'a InstanceSelection,
        tick_timer: usize,
    ) -> Element<'a> {
        if let Some(progress) = &self.mod_update_progress {
            return widget::column!(widget::text("Updating mods").size(20), progress.view())
                .padding(10)
                .spacing(10)
                .into();
        }

        let menu_main = widget::row!(
            widget::container(
                widget::scrollable(
                    widget::column!(
                        widget::row![
                            back_button().on_press(back_to_launch_screen(selected_instance, None)),
                            button_with_icon(icon_manager::create_with_size(14), "Add File", 14)
                                .on_press(Message::ManageMods(ManageModsMessage::AddFile))
                        ]
                        .spacing(7),
                        self.get_mod_installer_buttons(selected_instance),
                        widget::column!(
                            button_with_icon(
                                icon_manager::download_with_size(14),
                                "Download Content",
                                15
                            )
                            .on_press(Message::InstallMods(InstallModsMessage::Open)),
                            button_with_icon(icon_manager::save(), "Mod Presets", 15)
                                .on_press(Message::EditPresets(EditPresetsMessage::Open)),
                            button_with_icon(icon_manager::page(), "Jarmod Patches", 15)
                                .on_press(Message::ManageJarMods(ManageJarModsMessage::Open))
                        )
                        .spacing(5),
                        Self::open_mod_folder_button(selected_instance),
                        self.get_mod_update_pane(tick_timer),
                    )
                    .padding(10)
                    .spacing(10)
                )
                .style(LauncherTheme::style_scrollable_flat_dark)
                .height(Length::Fill)
            )
            .style(|n| n.style_container_sharp_box(0.0, Color::Dark)),
            self.get_mod_list()
        );

        if self.drag_and_drop_hovered {
            widget::stack!(
                menu_main,
                widget::center(widget::button(
                    widget::text("Drag and drop mod files to add them").size(20)
                ))
            )
            .into()
        } else {
            menu_main.into()
        }
    }

    fn get_mod_update_pane(&self, tick_timer: usize) -> Element {
        if self.update_check_handle.is_some() {
            let dots = ".".repeat((tick_timer % 3) + 1);
            widget::text!("Checking for mod updates{dots}")
                .size(13)
                .into()
        } else if self.available_updates.is_empty() {
            widget::column!().into()
        } else {
            widget::container(
                widget::column!(
                    widget::text("Mod Updates Available!").size(15),
                    widget::column(self.available_updates.iter().enumerate().map(
                        |(i, (id, name, is_enabled))| {
                            widget::checkbox(
                                format!(
                                    "{} - {name}",
                                    self.mods
                                        .mods
                                        .get(&id.get_index_str())
                                        .map(|n| n.name.clone())
                                        .unwrap_or_default()
                                ),
                                *is_enabled,
                            )
                            .on_toggle(move |b| {
                                Message::ManageMods(ManageModsMessage::UpdateCheckToggle(i, b))
                            })
                            .text_size(12)
                            .into()
                        }
                    ))
                    .spacing(10),
                    button_with_icon(icon_manager::update(), "Update", 16)
                        .on_press(Message::ManageMods(ManageModsMessage::UpdateMods)),
                )
                .padding(10)
                .spacing(10)
                .width(190),
            )
            .into()
        }
    }

    fn get_mod_installer_buttons(&self, selected_instance: &InstanceSelection) -> Element {
        match self.config.mod_type.as_str() {
            "Vanilla" => match selected_instance {
                InstanceSelection::Instance(_) => widget::column![
                    "Install:",
                    widget::row!(
                        install_ldr("Fabric").on_press(Message::InstallFabric(
                            InstallFabricMessage::ScreenOpen { is_quilt: false }
                        )),
                        install_ldr("Quilt").on_press(Message::InstallFabric(
                            InstallFabricMessage::ScreenOpen { is_quilt: true }
                        )),
                    )
                    .spacing(5),
                    widget::row!(
                        install_ldr("Forge")
                            .on_press(Message::InstallForgeStart { is_neoforge: false }),
                        install_ldr("NeoForge")
                            .on_press(Message::InstallForgeStart { is_neoforge: true })
                    )
                    .spacing(5),
                    install_ldr("OptiFine")
                        .on_press(Message::InstallOptifine(InstallOptifineMessage::ScreenOpen))
                ]
                .spacing(5)
                .into(),
                InstanceSelection::Server(_) => widget::column!(
                    "Install:",
                    widget::row!(
                        install_ldr("Fabric").on_press(Message::InstallFabric(
                            InstallFabricMessage::ScreenOpen { is_quilt: false }
                        )),
                        install_ldr("Quilt").on_press(Message::InstallFabric(
                            InstallFabricMessage::ScreenOpen { is_quilt: true }
                        )),
                    )
                    .spacing(5),
                    widget::row!(
                        install_ldr("Forge")
                            .on_press(Message::InstallForgeStart { is_neoforge: false }),
                        install_ldr("NeoForge")
                            .on_press(Message::InstallForgeStart { is_neoforge: true })
                    )
                    .spacing(5),
                    widget::row!(
                        widget::button("Bukkit").width(97),
                        widget::button("Spigot").width(97)
                    )
                    .spacing(5),
                    install_ldr("Paper").on_press(Message::InstallPaperStart),
                )
                .spacing(5)
                .into(),
            },

            "Forge" => widget::column!(
                widget::button("Install OptiFine"),
                Self::get_uninstall_panel(
                    &self.config.mod_type,
                    Message::UninstallLoaderForgeStart,
                )
            )
            .spacing(5)
            .into(),
            "OptiFine" => widget::column!(
                widget::button("Install Forge"),
                Self::get_uninstall_panel(
                    &self.config.mod_type,
                    Message::UninstallLoaderOptiFineStart,
                ),
            )
            .spacing(5)
            .into(),

            "NeoForge" => {
                Self::get_uninstall_panel(&self.config.mod_type, Message::UninstallLoaderForgeStart)
            }
            "Fabric" | "Quilt" => Self::get_uninstall_panel(
                &self.config.mod_type,
                Message::UninstallLoaderFabricStart,
            ),
            "Paper" => {
                Self::get_uninstall_panel(&self.config.mod_type, Message::UninstallLoaderPaperStart)
            }

            _ => {
                widget::column!(widget::text!("Unknown mod type: {}", self.config.mod_type)).into()
            }
        }
    }

    fn get_uninstall_panel(mod_type: &str, uninstall_loader_message: Message) -> Element {
        widget::button(
            widget::row![
                icon_manager::delete_with_size(14),
                widget::text!("Uninstall {mod_type}").size(15)
            ]
            .align_y(iced::alignment::Vertical::Center)
            .spacing(11)
            .padding(3),
        )
        .on_press(Message::UninstallLoaderConfirm(
            Box::new(uninstall_loader_message),
            mod_type.to_owned(),
        ))
        .into()
    }

    fn open_mod_folder_button(selected_instance: &InstanceSelection) -> Element {
        let path = {
            let path = selected_instance.get_dot_minecraft_path().join("mods");
            path.exists().then_some(path)
        };

        button_with_icon(icon_manager::folder_with_size(14), "Open Mods Folder", 15)
            .on_press_maybe(path.map(Message::CoreOpenPath))
            .into()
    }

    fn get_mod_list(&self) -> Element {
        if self.sorted_mods_list.is_empty() {
            return widget::column!("Download some mods to get started")
                .spacing(10)
                .padding(10)
                .width(Length::Fill)
                .into();
        }

        widget::container(
            widget::column!(
                widget::column![]
                    .push_maybe(
                        (self.config.mod_type == "Vanilla" && !self.sorted_mods_list.is_empty())
                        .then_some(
                            widget::container(
                                widget::text(
                                    // WARN: No loader installed
                                    "You haven't installed any mod loader! Install Fabric/Forge/Quilt/NeoForge as per your mods"
                                ).size(12)
                            ).padding(10).width(Length::Fill).style(|n: &LauncherTheme| n.style_container_sharp_box(0.0, Color::ExtraDark)),
                        )
                    )
                    .push(widget::text("Select some mods to perform actions on them").size(14))
                    .push(
                        widget::row![
                            button_with_icon(icon_manager::delete_with_size(13), "Delete", 13)
                                .on_press(Message::ManageMods(ManageModsMessage::DeleteSelected)),
                            button_with_icon(icon_manager::toggle_with_size(13), "Toggle", 13)
                                .on_press(Message::ManageMods(ManageModsMessage::ToggleSelected)),
                            button_with_icon(
                                icon_manager::tick_with_size(13),
                                if matches!(self.selected_state, SelectedState::All) {
                                    "Unselect All"
                                } else {
                                    "Select All"
                                },
                                13
                            )
                            .on_press(Message::ManageMods(ManageModsMessage::SelectAll)),
                        ]
                        .spacing(5)
                        .wrap()
                    )
                    .padding(10)
                    .spacing(5),
                self.get_mod_list_contents(),
            )
            .spacing(10),
        )
        .style(|n| n.style_container_sharp_box(0.0, Color::ExtraDark))
        .into()
    }

    fn get_mod_list_contents(&self) -> Element {
        widget::scrollable(
            widget::row![
                widget::column({
                    self.sorted_mods_list
                        .iter()
                        .map(|mod_list_entry| match mod_list_entry {
                            ModListEntry::Downloaded { id, config } => {
                                widget::row!(if config.manually_installed {
                                    widget::row!(widget::checkbox(
                                        format!(
                                            "{}{}",
                                            if config.enabled { "" } else { "(DISABLED) " },
                                            config.name
                                        ),
                                        self.selected_mods.contains(&SelectedMod::Downloaded {
                                            name: config.name.clone(),
                                            id: (*id).clone()
                                        })
                                    )
                                    .on_toggle(move |t| {
                                        Message::ManageMods(ManageModsMessage::ToggleCheckbox(
                                            (config.name.clone(), id.clone()),
                                            t,
                                        ))
                                    }))
                                } else {
                                    widget::row!(widget::text!("- (DEPENDENCY) {}", config.name))
                                },)
                                .into()
                            }
                            ModListEntry::Local { file_name } => widget::checkbox(
                                file_name.clone(),
                                self.selected_mods.contains(&SelectedMod::Local {
                                    file_name: file_name.clone(),
                                }),
                            )
                            .on_toggle(move |t| {
                                Message::ManageMods(ManageModsMessage::ToggleCheckboxLocal(
                                    file_name.clone(),
                                    t,
                                ))
                            })
                            .into(),
                        })
                })
                .padding(10)
                .spacing(10),
                widget::column({
                    self.sorted_mods_list.iter().map(|entry| match entry {
                        ModListEntry::Downloaded { config, .. } => {
                            widget::text(&config.installed_version).into()
                        }
                        ModListEntry::Local { .. } => widget::text(" ").into(),
                    })
                })
                .padding(10)
                .spacing(10)
            ]
            .spacing(10),
        )
        .direction(widget::scrollable::Direction::Both {
            vertical: widget::scrollable::Scrollbar::new(),
            horizontal: widget::scrollable::Scrollbar::new(),
        })
        .style(LauncherTheme::style_scrollable_flat_extra_dark)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}

fn install_ldr(fabric: &str) -> widget::Button<'_, Message, LauncherTheme, iced::Renderer> {
    widget::button(fabric).width(97)
}
