use iced::{widget, Length};
use ql_core::LAUNCHER_DIR;

use crate::{
    config::LauncherConfig,
    icon_manager,
    state::{LauncherSettingsMessage, LauncherSettingsTab, MenuLauncherSettings, Message},
    stylesheet::{
        color::Color,
        styles::{LauncherTheme, LauncherThemeColor},
        widgets::StyleButton,
    },
};

use super::{back_button, button_with_icon, get_theme_selector, Element, DISCORD, GITHUB};

const SETTINGS_SPACING: f32 = 7.0;
const PADDING_NOT_BOTTOM: iced::Padding = iced::Padding {
    top: 10.0,
    bottom: 0.0,
    left: 10.0,
    right: 10.0,
};
const PADDING_LEFT: iced::Padding = iced::Padding {
    top: 0.0,
    right: 0.0,
    bottom: 0.0,
    left: 10.0,
};

impl MenuLauncherSettings {
    pub fn view<'a>(&'a self, config: &'a LauncherConfig) -> Element<'a> {
        widget::row![
            widget::container(
                widget::column![
                    widget::column!(back_button().on_press(Message::LaunchScreenOpen {
                        message: None,
                        clear_selection: false
                    }))
                    .padding(PADDING_NOT_BOTTOM),
                    widget::row![
                        icon_manager::settings_with_size(20),
                        widget::text("Settings").size(20),
                    ]
                    .padding(iced::Padding {
                        top: 5.0,
                        right: 0.0,
                        bottom: 2.0,
                        left: 10.0,
                    })
                    .spacing(10),
                    widget::column(LauncherSettingsTab::ALL.iter().map(|tab| {
                        let text = widget::text(tab.to_string());
                        if *tab == self.selected_tab {
                            widget::container(widget::row!(widget::Space::with_width(5), text))
                                .style(LauncherTheme::style_container_selected_flat_button)
                                .width(Length::Fill)
                                .padding(5)
                                .into()
                        } else {
                            widget::button(text)
                                .on_press(Message::LauncherSettings(
                                    LauncherSettingsMessage::ChangeTab(*tab),
                                ))
                                .style(|n: &LauncherTheme, status| {
                                    n.style_button(status, StyleButton::FlatExtraDark)
                                })
                                .width(Length::Fill)
                                .into()
                        }
                    }))
                ]
                .spacing(10)
            )
            .height(Length::Fill)
            .width(180)
            .style(|n: &LauncherTheme| n.style_container_sharp_box(0.0, Color::ExtraDark)),
            widget::scrollable(self.selected_tab.view(config, self))
                .width(Length::Fill)
                .style(LauncherTheme::style_scrollable_flat_dark)
        ]
        .into()
    }

    fn view_options<'a>(&'a self, config: &'a LauncherConfig) -> Element<'a> {
        let (light, dark) = get_theme_selector(config);

        let color_scheme_picker = LauncherThemeColor::ALL.iter().map(|color| {
            widget::button(widget::text(color.to_string()).size(14))
                .style(|theme: &LauncherTheme, s| {
                    LauncherTheme {
                        lightness: theme.lightness,
                        color: *color,
                    }
                    .style_button(s, StyleButton::Round)
                })
                .on_press(Message::LauncherSettings(
                    LauncherSettingsMessage::StylePicked(color.to_string()),
                ))
                .into()
        });

        widget::column!(
            widget::column![widget::text("User Interface").size(20)].padding(PADDING_NOT_BOTTOM),
            widget::column!("Theme:", widget::row![light, dark].spacing(5))
                .padding(iced::Padding {
                    top: 0.0,
                    bottom: 10.0,
                    left: 10.0,
                    right: 10.0,
                })
                .spacing(5),
            widget::horizontal_rule(1),
            widget::column!(
                "Color scheme:",
                widget::row(color_scheme_picker).spacing(5).wrap()
            )
            .padding(10)
            .spacing(5),
            widget::horizontal_rule(1),
            widget::column![
                widget::row![
                    widget::text!("UI Scale ({:.2}x)  ", self.temp_scale),
                    widget::button(widget::text("Apply").size(13)).on_press(
                        Message::LauncherSettings(LauncherSettingsMessage::UiScaleApply)
                    ),
                ]
                .align_y(iced::Alignment::Center),
                widget::slider(0.5..=2.0, self.temp_scale, |n| Message::LauncherSettings(
                    LauncherSettingsMessage::UiScale(n)
                ))
                .step(0.1),
                widget::text("Warning: slightly buggy").size(12),
            ]
            .padding(10)
            .spacing(5),
        )
        .spacing(SETTINGS_SPACING)
        .into()
    }
}

impl LauncherSettingsTab {
    pub fn view<'a>(
        &'a self,
        config: &'a crate::config::LauncherConfig,
        menu: &'a MenuLauncherSettings,
    ) -> Element<'a> {
        match self {
            LauncherSettingsTab::UserInterface => menu.view_options(config),
            LauncherSettingsTab::Internal => widget::column![
                widget::column![
                    widget::text("Advanced").size(20),
                    button_with_icon(icon_manager::folder(), "Open Launcher Folder", 16)
                        .on_press(Message::CoreOpenPath(LAUNCHER_DIR.clone()))
                ]
                .spacing(10)
                .padding(10),
                widget::horizontal_rule(1),
                widget::column![
                    button_with_icon(icon_manager::delete(), "Clear Java installs", 16).on_press(
                        Message::LauncherSettings(LauncherSettingsMessage::ClearJavaInstalls)
                    ),
                    widget::text(
                        "Might fix some Java problems.\nPerfectly safe, will be redownloaded."
                    )
                    .size(12),
                ]
                .padding(10)
                .spacing(10),
            ]
            .spacing(SETTINGS_SPACING)
            .into(),
            LauncherSettingsTab::About => {
                let gpl3_button =
                    // widget::button(widget::rich_text![widget::span("GNU GPLv3 License").underline(true)].size(12))
                    // iced bug (or maybe some dumb mistake I made),
                    // putting underlines in buttons makes them unclickable.
                    widget::button(widget::text("GNU GPLv3 License").size(12))
                        .padding(0)
                        // .style(|n: &LauncherTheme, status| n.style_button(status, StyleButton::FlatExtraDark))
                        // Since I can't underline the buttons,
                        // I have to resort to making them pop out.
                        .style(|n: &LauncherTheme, status| n.style_button(status, StyleButton::Flat))
                        .on_press(Message::LicenseChangeTab(crate::state::LicenseTab::Gpl3));

                let links = widget::row![
                    button_with_icon(icon_manager::page(), "Website", 16).on_press(
                        Message::CoreOpenLink(
                            "https://mrmayman.github.io/quantumlauncher".to_owned()
                        )
                    ),
                    button_with_icon(icon_manager::github(), "Github", 16)
                        .on_press(Message::CoreOpenLink(GITHUB.to_owned())),
                    button_with_icon(icon_manager::chat(), "Discord", 16)
                        .on_press(Message::CoreOpenLink(DISCORD.to_owned())),
                ]
                .padding(iced::Padding {
                    top: 0.0,
                    right: 0.0,
                    bottom: 10.0,
                    left: 10.0,
                })
                .spacing(5)
                .wrap();

                let menus = widget::row![
                    widget::button("Changelog").on_press(Message::CoreOpenChangeLog),
                    widget::button("Welcome Screen").on_press(Message::CoreOpenIntro),
                    widget::button("Licenses").on_press(Message::LicenseOpen),
                ]
                .padding(PADDING_LEFT)
                .spacing(5)
                .wrap();

                widget::column![
                    widget::column![widget::text("About QuantumLauncher").size(20)]
                        .padding(PADDING_NOT_BOTTOM),
                    menus,
                    links,
                    widget::horizontal_rule(1),
                    widget::column![
                        widget::row![
                            widget::text(
                                "QuantumLauncher is free and open source software under the "
                            )
                            .size(12),
                            gpl3_button,
                        ]
                        .wrap(),
                        widget::text(
                            r"No warranty is provided for this software.
You're free to share, modify, and redistribute it under the same license."
                        )
                        .size(12),
                        widget::text(
                            r"If you like this launcher, consider sharing it with your friends.
Every new user motivates me to keep working on this :)"
                        )
                        .size(12),
                    ]
                    .padding(iced::Padding {
                        top: 10.0,
                        bottom: 10.0,
                        left: 15.0,
                        right: 10.0,
                    })
                    .spacing(5),
                ]
                .spacing(SETTINGS_SPACING)
                .into()
            }
        }
    }
}
