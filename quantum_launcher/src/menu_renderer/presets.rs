use std::collections::HashSet;

use iced::widget;
use ql_core::SelectedMod;

use crate::{
    icon_manager,
    launcher_state::{
        EditPresetsMessage, ManageModsMessage, MenuEditPresets, MenuEditPresetsInner, Message,
        ModListEntry, SelectedState,
    },
    menu_renderer::button_with_icon,
    stylesheet::{color::Color, styles::LauncherTheme, widgets::StyleButton},
};

use super::{launch::TAB_HEIGHT, Element};

impl MenuEditPresets {
    pub fn view(&self, window_size: (f32, f32)) -> Element {
        if let Some(progress) = &self.progress {
            return widget::column!(
                widget::text("Installing mods").size(20),
                progress.view(),
                widget::text("Check debug log (at the bottom) for more info").size(12),
            )
            .padding(10)
            .spacing(10)
            .into();
        }

        if let MenuEditPresetsInner::Build {
            is_building: true, ..
        } = &self.inner
        {
            return widget::column!(widget::text("Building Preset").size(20),)
                .padding(10)
                .spacing(10)
                .into();
        }

        widget::column![
            widget::container(
                widget::row![
                    widget::Space::with_width(16.0),
                    create_generic_tab_button(
                        widget::row![icon_manager::back(), "Back"]
                            .padding(5)
                            .spacing(10)
                            .into()
                    )
                    .on_press(Message::ManageMods(ManageModsMessage::ScreenOpen)),
                    widget::Space::with_width(16.0),

                    self.get_tab_button("Create"),
                    self.get_tab_button("Recommended"),

                    widget::horizontal_space(),

                    widget::tooltip(
                        create_generic_tab_button(
                            widget::row![icon_manager::folder(), "Import"]
                                .spacing(10)
                                .padding(5)
                                .into()
                        )
                        .on_press(Message::EditPresets(EditPresetsMessage::Load)),
                        widget::column![
                            widget::text("Note: Sideloaded .jar mods in untrusted presets could have viruses").size(12),
                            widget::text("To get rid of them, after installing remove all mods in the list ending in \".jar\"").size(12)
                        ],
                        widget::tooltip::Position::Bottom)
                            .style(|n: &LauncherTheme| n.style_container_sharp_box(0.0, Color::ExtraDark)),
                ]
            )
            .style(|n| n.style_container_sharp_box(0.0, Color::ExtraDark)),

            widget::scrollable(
                widget::container(
                    self.get_create_preset_page()
                )
                .padding(10)
                .width(window_size.0)
                .style(|n: &LauncherTheme| n.style_container_sharp_box(0.0, Color::Dark))
            )
        ]
        .into()
    }

    fn get_tab_button<'a>(&'a self, n: &'a str) -> Element<'a> {
        if self.inner.id() == n {
            widget::container(n)
                .style(LauncherTheme::style_container_selected_flat_button)
                .padding(iced::Padding {
                    top: 5.0,
                    bottom: 5.0,
                    right: 10.0,
                    left: 10.0,
                })
                .height(TAB_HEIGHT)
                .into()
        } else {
            widget::button(n)
                .style(|n: &LauncherTheme, status| {
                    n.style_button(status, StyleButton::FlatExtraDark)
                })
                .on_press(Message::EditPresets(EditPresetsMessage::TabChange(
                    n.to_owned(),
                )))
                .height(TAB_HEIGHT)
                .into()
        }
    }

    fn get_create_preset_page(&self) -> Element {
        match &self.inner {
            MenuEditPresetsInner::Build {
                selected_state,
                selected_mods,
                ..
            } => widget::column!(
                "Presets are small bundles of mods and their configuration that you can share with anyone.",
                "You can import presets, create them or download recommended mods (if you haven't installed any yet).",
                if selected_mods.is_empty() {
                    widget::column!["You have no mods installed! Go to Recommended to find some good ones."]
                } else {
                    widget::column![
                        widget::text("Create Preset").size(20),
                        "Select Mods to keep",
                        widget::button(if let SelectedState::All = selected_state {
                            "Unselect All"
                        } else {
                            "Select All"
                        })
                        .on_press(Message::EditPresets(EditPresetsMessage::SelectAll)),
                        widget::container(self.get_mods_list(selected_mods).padding(10)),
                        button_with_icon(icon_manager::save(), "Build Preset", 16)
                            .on_press(Message::EditPresets(EditPresetsMessage::BuildYourOwn)),
                    ]
                }.spacing(10)
            )
            .spacing(10)
            .into(),
            MenuEditPresetsInner::Recommended {
                progress,
                error,
            } => {
                if let Some(error) = error {
                    widget::column!(
                        widget::text!("Error loading presets: {error}"),
                        widget::button("Copy Error").on_press(Message::CoreCopyText(error.clone()))
                    )
                    .spacing(10)
                    .into()
                } else if let Some(mods) = &self.recommended_mods {
                    widget::column!(
                        button_with_icon(icon_manager::download(), "Download Recommended Mods", 16)
                            .on_press(Message::EditPresets(
                                EditPresetsMessage::RecommendedDownload
                            )),
                        widget::column(mods.iter().enumerate().map(|(i, (e, n))| {
                            let elem: Element = if n.enabled_by_default {
                                widget::text!("- {}", n.name).into()
                            } else {
                                widget::checkbox(n.name, *e)
                                    .on_toggle(move |n| {
                                        Message::EditPresets(EditPresetsMessage::RecommendedToggle(
                                            i, n,
                                        ))
                                    })
                                    .into()
                            };
                            widget::column!(elem, widget::text(n.description).size(12))
                                .spacing(5)
                                .into()
                        }))
                        .spacing(10)
                    )
                    .spacing(10)
                    .into()
                } else {
                    progress.view()
                }
            }
        }
    }

    fn get_mods_list<'a>(
        &'a self,
        selected_mods: &'a HashSet<SelectedMod>,
    ) -> widget::Column<'a, Message, LauncherTheme, iced::Renderer> {
        widget::column(self.sorted_mods_list.iter().map(|entry| {
            if entry.is_manually_installed() {
                widget::checkbox(entry.name(), selected_mods.contains(&entry.id()))
                    .on_toggle(move |t| match entry {
                        ModListEntry::Downloaded { id, config } => {
                            Message::EditPresets(EditPresetsMessage::ToggleCheckbox(
                                (config.name.clone(), id.clone()),
                                t,
                            ))
                        }
                        ModListEntry::Local { file_name } => Message::EditPresets(
                            EditPresetsMessage::ToggleCheckboxLocal(file_name.clone(), t),
                        ),
                    })
                    .into()
            } else {
                widget::text!(" - (DEPENDENCY) {}", entry.name()).into()
            }
        }))
        .spacing(5)
    }
}

fn create_generic_tab_button(n: Element) -> widget::Button<'_, Message, LauncherTheme> {
    widget::button(n)
        .padding(0)
        .style(|n, status| n.style_button(status, StyleButton::FlatExtraDark))
        .height(TAB_HEIGHT)
}
