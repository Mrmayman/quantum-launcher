use std::path::Path;

use iced::{widget, Length};
use ql_core::{InstanceSelection, SelectedMod};

use crate::{
    icon_manager,
    launcher_state::{
        EditPresetsMessage, InstallFabricMessage, InstallModsMessage, InstallOptifineMessage,
        ManageModsMessage, MenuEditMods, Message, ModListEntry, SelectedState,
    },
    stylesheet::{color::Color, styles::LauncherTheme},
};

use super::{back_to_launch_screen, button_with_icon, Element};

impl MenuEditMods {
    pub fn view<'a>(
        &'a self,
        selected_instance: &'a InstanceSelection,
        launcher_dir: &'a Path,
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
                        widget::button(
                            widget::row![icon_manager::back(), "Back"]
                                .spacing(10)
                                .padding(5)
                        )
                        .on_press(back_to_launch_screen(selected_instance, None)),
                        self.get_mod_installer_buttons(selected_instance),
                        Self::open_mod_folder_button(selected_instance, launcher_dir),
                        widget::container(self.get_mod_update_pane()),
                    )
                    .padding(10)
                    .spacing(20)
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

    fn get_mod_update_pane(&self) -> widget::Column<'_, Message, LauncherTheme> {
        if self.available_updates.is_empty() {
            widget::column!()
        } else {
            widget::column!(
                "Mod Updates Available!",
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
            .width(200)
        }
    }

    fn get_mod_installer_buttons(
        &self,
        selected_instance: &InstanceSelection,
    ) -> widget::Column<'_, Message, LauncherTheme> {
        match self.config.mod_type.as_str() {
            "Vanilla" => match selected_instance {
                InstanceSelection::Instance(_) => widget::column![
                    "Install:",
                    widget::row!(
                        widget::button("Fabric")
                            .on_press(Message::InstallFabric(InstallFabricMessage::ScreenOpen {
                                is_quilt: false
                            }))
                            .width(97),
                        widget::button("Quilt")
                            .width(98)
                            .on_press(Message::InstallFabric(InstallFabricMessage::ScreenOpen {
                                is_quilt: true
                            })),
                    )
                    .spacing(5),
                    widget::row!(
                        widget::button("Forge")
                            .on_press(Message::InstallForgeStart { is_neoforge: false })
                            .width(97),
                        widget::button("NeoForge")
                            .on_press(Message::InstallForgeStart { is_neoforge: true })
                            .width(98),
                    )
                    .spacing(5),
                    widget::row!(widget::button("OptiFine")
                        .on_press(Message::InstallOptifine(InstallOptifineMessage::ScreenOpen))
                        .width(97))
                    .spacing(5),
                ],
                InstanceSelection::Server(_) => widget::column!(
                    "Install:",
                    widget::row!(
                        widget::button("Fabric")
                            .width(97)
                            .on_press(Message::InstallFabric(InstallFabricMessage::ScreenOpen {
                                is_quilt: false
                            })),
                        widget::button("Quilt")
                            .width(98)
                            .on_press(Message::InstallFabric(InstallFabricMessage::ScreenOpen {
                                is_quilt: true
                            })),
                    )
                    .spacing(5),
                    widget::row!(
                        widget::button("Forge")
                            .on_press(Message::InstallForgeStart { is_neoforge: false })
                            .width(97),
                        widget::button("NeoForge")
                            .on_press(Message::InstallForgeStart { is_neoforge: true })
                            .width(98)
                    )
                    .spacing(5),
                    widget::row!(
                        widget::button("Bukkit").width(97),
                        widget::button("Spigot").width(98)
                    )
                    .spacing(5),
                    widget::button("Paper")
                        .width(97)
                        .on_press(Message::InstallPaperStart),
                ),
            },
            "Forge" => {
                widget::column!(
                    widget::button("Install OptiFine"),
                    Self::get_uninstall_panel(
                        &self.config.mod_type,
                        Message::UninstallLoaderForgeStart,
                        true
                    )
                )
            }
            "OptiFine" => {
                widget::column!(
                    widget::button("Install Forge"),
                    Self::get_uninstall_panel(
                        &self.config.mod_type,
                        Message::UninstallLoaderOptiFineStart,
                        false
                    ),
                )
            }
            "NeoForge" => Self::get_uninstall_panel(
                &self.config.mod_type,
                Message::UninstallLoaderForgeStart,
                true,
            ),
            "Fabric" | "Quilt" => Self::get_uninstall_panel(
                &self.config.mod_type,
                Message::UninstallLoaderFabricStart,
                true,
            ),
            "Paper" => Self::get_uninstall_panel(
                &self.config.mod_type,
                Message::UninstallLoaderPaperStart,
                false,
            ),
            _ => {
                widget::column!(widget::text!("Unknown mod type: {}", self.config.mod_type))
            }
        }
        .spacing(5)
    }

    fn get_uninstall_panel(
        mod_type: &str,
        uninstall_loader_message: Message,
        download_mods: bool,
    ) -> iced::widget::Column<'_, Message, LauncherTheme> {
        widget::column!(
            widget::button(
                widget::row!(
                    icon_manager::delete(),
                    widget::text!("Uninstall {mod_type}")
                )
                .spacing(10)
                .padding(5)
            )
            .on_press(Message::UninstallLoaderConfirm(
                Box::new(uninstall_loader_message),
                mod_type.to_owned()
            )),
            if download_mods {
                widget::column!(
                    button_with_icon(icon_manager::download(), "Download Mods", 16)
                        .on_press(Message::InstallMods(InstallModsMessage::Open)),
                    button_with_icon(icon_manager::save(), "Mod Presets...", 16)
                        .on_press(Message::EditPresets(EditPresetsMessage::Open))
                )
                .spacing(5)
            } else {
                widget::column!()
            },
        )
        .spacing(5)
    }

    fn open_mod_folder_button<'a>(
        selected_instance: &'a InstanceSelection,
        parent: &'a Path,
    ) -> Element<'a> {
        let path = {
            let path = selected_instance
                .get_dot_minecraft_path(parent)
                .join("mods");
            path.exists().then_some(path.to_str().unwrap().to_owned())
        };

        button_with_icon(icon_manager::folder(), "Open Mods Folder", 16)
            .on_press_maybe(path.map(Message::CoreOpenDir))
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
                widget::column![
                    widget::text("Select some mods to perform actions on them").size(14),
                    widget::row![
                        button_with_icon(icon_manager::delete(), "Delete", 14)
                            .on_press(Message::ManageMods(ManageModsMessage::DeleteSelected)),
                        button_with_icon(icon_manager::toggle(), "Toggle", 14)
                            .on_press(Message::ManageMods(ManageModsMessage::ToggleSelected)),
                        button_with_icon(
                            icon_manager::tick(),
                            if matches!(self.selected_state, SelectedState::All) {
                                "Unselect All"
                            } else {
                                "Select All"
                            },
                            14
                        )
                        .on_press(Message::ManageMods(ManageModsMessage::SelectAll))
                    ]
                    .spacing(5)
                    .wrap()
                ]
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
