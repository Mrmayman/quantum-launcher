use iced::{widget, Length};
use ql_core::InstanceSelection;

use crate::{
    icon_manager,
    menu_renderer::{button_with_icon, Element},
    state::{ManageJarModsMessage, ManageModsMessage, MenuEditJarMods, Message, SelectedState},
    stylesheet::{color::Color, styles::LauncherTheme},
};

impl MenuEditJarMods {
    pub fn view(&self, selected_instance: &InstanceSelection) -> Element {
        let menu_main = widget::row!(
            widget::container(
                widget::scrollable(
                    widget::column!(
                        button_with_icon(icon_manager::back_with_size(14), "Back", 14)
                            .on_press(Message::ManageMods(ManageModsMessage::ScreenOpen)),
                        widget::column![
                            {
                                let path = {
                                    let path =
                                        selected_instance.get_instance_path().join("jarmods");
                                    path.exists().then_some(path)
                                };

                                button_with_icon(
                                    icon_manager::folder_with_size(14),
                                    "Open Folder",
                                    15,
                                )
                                .on_press_maybe(path.map(Message::CoreOpenPath))
                            },
                            button_with_icon(icon_manager::create(), "Add file", 15)
                                .on_press(Message::ManageJarMods(ManageJarModsMessage::AddFile)),
                        ]
                        .spacing(5),
                        widget::row![
                            "You can find some good jar mods at McArchive",
                            widget::button("Open").on_press(Message::CoreOpenLink(
                                "https://mcarchive.net".to_owned()
                            ))
                        ]
                        .spacing(5)
                        .wrap(),
                        widget::column![
                            "WARNING: Jarmods are mainly for OLD Minecraft versions.",
                            widget::text(
                                "This is easier than copying .class files into Minecraft's jar"
                            )
                            .size(12),
                            widget::text(
                                "If you just want some mods (for newer Minecraft), click Back"
                            )
                            .size(12),
                        ],
                    )
                    .padding(10)
                    .spacing(10)
                )
                .style(LauncherTheme::style_scrollable_flat_dark)
                .height(Length::Fill)
            )
            .width(250)
            .style(|n| n.style_container_sharp_box(0.0, Color::Dark)),
            self.get_mod_list()
        );

        if self.drag_and_drop_hovered {
            widget::stack!(
                menu_main,
                widget::center(widget::button(
                    widget::text("Drag and drop jarmod files to add them").size(20)
                ))
            )
            .into()
        } else {
            menu_main.into()
        }
    }

    fn get_mod_list(&self) -> Element {
        if self.jarmods.mods.is_empty() {
            return widget::column!("Add some mods to get started")
                .spacing(10)
                .padding(10)
                .width(Length::Fill)
                .into();
        }

        widget::container(
            widget::column!(
                widget::column![
                    widget::text("Select some jarmods to perform actions on them").size(14),
                    widget::row![
                        widget::button("Delete")
                            .on_press(Message::ManageJarMods(ManageJarModsMessage::DeleteSelected)),
                        widget::button("Toggle")
                            .on_press(Message::ManageJarMods(ManageJarModsMessage::ToggleSelected)),
                        widget::button(if matches!(self.selected_state, SelectedState::All) {
                            "Unselect All"
                        } else {
                            "Select All"
                        })
                        .on_press(Message::ManageJarMods(ManageJarModsMessage::SelectAll)),
                        widget::button(icon_manager::arrow_up())
                            .on_press(Message::ManageJarMods(ManageJarModsMessage::MoveUp)),
                        widget::button(icon_manager::arrow_down())
                            .on_press(Message::ManageJarMods(ManageJarModsMessage::MoveDown)),
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
            widget::column({
                self.jarmods.mods.iter().map(|jarmod| {
                    widget::checkbox(
                        format!(
                            "{}{}",
                            if jarmod.enabled { "" } else { "(DISABLED) " },
                            jarmod.filename
                        ),
                        self.selected_mods.contains(&jarmod.filename),
                    )
                    .on_toggle(move |t| {
                        Message::ManageJarMods(ManageJarModsMessage::ToggleCheckbox(
                            jarmod.filename.clone(),
                            t,
                        ))
                    })
                    .into()
                })
            })
            .padding(10)
            .spacing(10),
        )
        .style(LauncherTheme::style_scrollable_flat_extra_dark)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}
