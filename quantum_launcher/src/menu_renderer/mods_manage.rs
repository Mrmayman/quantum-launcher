use iced::widget;
use ql_core::{file_utils, InstanceSelection, SelectedMod};

use crate::{
    icon_manager,
    launcher_state::{
        InstallFabricMessage, InstallModsMessage, InstallOptifineMessage, ManageModsMessage,
        MenuEditMods, Message, ModListEntry, SelectedState,
    },
    stylesheet::styles::LauncherTheme,
};

use super::{back_to_launch_screen, button_with_icon, Element};

impl MenuEditMods {
    pub fn view<'a>(&'a self, selected_instance: &'a InstanceSelection) -> Element<'a> {
        if let Some(progress) = &self.mod_update_progress {
            return widget::column!(
                widget::text("Updating mods").size(20),
                widget::progress_bar(0.0..=1.0, progress.num),
                widget::text(&progress.message),
            )
            .padding(10)
            .spacing(10)
            .into();
        }

        widget::row!(
            widget::scrollable(
                widget::column!(
                    widget::button(
                        widget::row![icon_manager::back(), "Back"]
                            .spacing(10)
                            .padding(5)
                    )
                    .on_press(back_to_launch_screen(selected_instance, None)),
                    self.get_mod_installer_buttons(selected_instance),
                    Self::open_mod_folder_button(selected_instance),
                    widget::container(self.get_mod_update_pane()),
                )
                .padding(10)
                .spacing(20),
            ),
            self.get_mod_list()
        )
        .padding(10)
        .spacing(10)
        .into()
    }

    fn get_mod_update_pane(&self) -> widget::Column<'_, Message, LauncherTheme> {
        if self.available_updates.is_empty() {
            widget::column!()
        } else {
            widget::column!(
                "Mod Updates Available!",
                widget::column(self.available_updates.iter().enumerate().map(
                    |(i, (_, name, is_enabled))| {
                        widget::checkbox(name, *is_enabled)
                            .on_toggle(move |b| {
                                Message::ManageMods(ManageModsMessage::UpdateCheckToggle(i, b))
                            })
                            .text_size(12)
                            .into()
                    }
                ))
                .spacing(10),
                button_with_icon(icon_manager::update(), "Update")
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
                            .on_press(Message::InstallForgeStart)
                            .width(97),
                        widget::button("NeoForge").width(98),
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
                            .on_press(Message::InstallForgeStart)
                            .width(97),
                        widget::button("NeoForge").width(98)
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
                widget::column!(widget::text(format!(
                    "Unknown mod type: {}",
                    self.config.mod_type
                )))
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
                    widget::text(format!("Uninstall {mod_type}"))
                )
                .spacing(10)
                .padding(5)
            )
            .on_press(uninstall_loader_message),
            if download_mods {
                widget::column!(
                    button_with_icon(icon_manager::download(), "Download Mods")
                        .on_press(Message::InstallMods(InstallModsMessage::Open)),
                    widget::text("Warning: the mod store is\nexperimental and may have bugs")
                        .size(12),
                    button_with_icon(icon_manager::save(), "Mod Presets...")
                        .on_press(Message::EditPresetsOpen)
                )
                .spacing(5)
            } else {
                widget::column!()
            },
        )
        .spacing(5)
    }

    fn open_mod_folder_button(selected_instance: &InstanceSelection) -> Element {
        let path = {
            if let Ok(dot_minecraft_dir) = file_utils::get_dot_minecraft_dir(selected_instance) {
                let path = dot_minecraft_dir.join("mods");
                path.exists().then_some(path.to_str().unwrap().to_owned())
            } else {
                None
            }
        };

        button_with_icon(icon_manager::folder(), "Go to Mods Folder")
            .on_press_maybe(path.map(Message::CoreOpenDir))
            .into()
    }

    fn get_mod_list(&self) -> Element {
        if self.sorted_mods_list.is_empty() {
            return widget::column!("Download some mods to get started")
                .spacing(10)
                .into();
        }

        widget::column!(
            "Select some mods to perform actions on them",
            widget::row!(
                button_with_icon(icon_manager::delete(), "Delete")
                    .on_press(Message::ManageMods(ManageModsMessage::DeleteSelected)),
                button_with_icon(icon_manager::toggle(), "Toggle On/Off")
                    .on_press(Message::ManageMods(ManageModsMessage::ToggleSelected)),
                button_with_icon(
                    icon_manager::tick(),
                    if matches!(self.selected_state, SelectedState::All) {
                        "Unselect All"
                    } else {
                        "Select All"
                    }
                )
                .on_press(Message::ManageModsSelectAll)
            )
            .spacing(5),
            self.get_mod_list_contents(),
        )
        .spacing(10)
        .into()
    }

    fn get_mod_list_contents(&self) -> Element {
        widget::scrollable(
            widget::column({
                self.sorted_mods_list
                    .iter()
                    .map(|mod_list_entry| match mod_list_entry {
                        ModListEntry::Downloaded { id, config } => widget::row!(
                            if config.manually_installed {
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
                                widget::row!(widget::text(format!(
                                    "- (DEPENDENCY) {}",
                                    config.name
                                )))
                            },
                            widget::horizontal_space(),
                            widget::text(&config.installed_version).width(100).size(12),
                        )
                        .into(),
                        ModListEntry::Local { file_name } => widget::row!(widget::checkbox(
                            file_name.clone(),
                            self.selected_mods.contains(&SelectedMod::Local {
                                file_name: file_name.clone()
                            })
                        )
                        .on_toggle(move |t| {
                            Message::ManageMods(ManageModsMessage::ToggleCheckboxLocal(
                                file_name.clone(),
                                t,
                            ))
                        }))
                        .into(),
                    })
            })
            .padding(10)
            .spacing(10),
        )
        .into()
    }
}