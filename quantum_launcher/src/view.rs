use iced::{widget, Length};
use ql_core::LOGGER;

use crate::{
    icon_manager,
    menu_renderer::{
        button_with_icon, changelog::changelog_0_4_1, view_account_login, Element, DISCORD,
    },
    state::{Launcher, Message, State},
    stylesheet::{color::Color, styles::LauncherTheme, widgets::StyleButton},
    DEBUG_LOG_BUTTON_HEIGHT,
};

impl Launcher {
    pub fn view(&self) -> Element {
        widget::column![
            widget::column![self.view_menu()].height(
                (self.window_size.1 / if self.is_log_open { 2.0 } else { 1.0 })
                    - DEBUG_LOG_BUTTON_HEIGHT
            ),
            widget::tooltip(
                widget::button(widget::row![
                    widget::horizontal_space(),
                    widget::text(if self.is_log_open { "v" } else { "^" }).size(10),
                    widget::horizontal_space()
                ])
                .padding(0)
                .height(DEBUG_LOG_BUTTON_HEIGHT)
                .style(|n: &LauncherTheme, status| n.style_button(status, StyleButton::FlatDark))
                .on_press(Message::CoreLogToggle),
                widget::text(if self.is_log_open {
                    "Close launcher log"
                } else {
                    "Open launcher debug log (troubleshooting)"
                })
                .size(12),
                widget::tooltip::Position::Top
            )
            .style(|n| n.style_container_sharp_box(0.0, Color::ExtraDark)),
        ]
        .push_maybe(self.is_log_open.then(|| {
            const TEXT_SIZE: f32 = 12.0;

            let text = {
                if let Some(logger) = LOGGER.as_ref() {
                    let logger = logger.lock().unwrap();
                    logger.text.iter().cloned().map(|n| n.0).collect()
                } else {
                    Vec::new()
                }
            };

            Self::view_launcher_log(
                text,
                TEXT_SIZE,
                self.log_scroll,
                Message::CoreLogScroll,
                Message::CoreLogScrollAbsolute,
            )
        }))
        .into()
    }

    fn view_menu(&self) -> Element {
        match &self.state {
            State::Launch(menu) => self.view_main_menu(menu),
            State::AccountLoginProgress(progress) => widget::column![
                widget::text("Logging into Microsoft account").size(20),
                progress.view()
            ]
            .spacing(10)
            .padding(10)
            .into(),
            State::GenericMessage(msg) => widget::column![widget::text(msg)].padding(10).into(),
            State::LoginMS(menu) => menu.view(),
            State::AccountLogin => view_account_login(),
            State::EditMods(menu) => {
                menu.view(self.selected_instance.as_ref().unwrap(), self.tick_timer)
            }
            State::Create(menu) => menu.view(),
            State::ConfirmAction {
                msg1,
                msg2,
                yes,
                no,
            } => widget::column![
                widget::text!("Are you SURE you want to {msg1}?").size(20),
                msg2.as_str(),
                widget::button("Yes").on_press(yes.clone()),
                widget::button("No").on_press(no.clone()),
            ]
            .padding(10)
            .spacing(10)
            .into(),
            State::Error { error } => widget::scrollable(
                widget::column!(
                    widget::text!("Error: {error}"),
                    widget::row![
                        widget::button("Back").on_press(Message::LaunchScreenOpen {
                            message: None,
                            clear_selection: true
                        }),
                        widget::button("Copy Error").on_press(Message::CoreErrorCopy),
                        widget::button("Copy Error + Log").on_press(Message::CoreErrorCopyLog),
                        widget::button("Join Discord for help")
                            .on_press(Message::CoreOpenLink(DISCORD.to_owned()))
                    ]
                    .spacing(5)
                    .wrap()
                )
                .padding(10)
                .spacing(10),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .style(LauncherTheme::style_scrollable_flat_extra_dark)
            .into(),
            State::InstallFabric(menu) => {
                menu.view(self.selected_instance.as_ref().unwrap(), self.tick_timer)
            }
            State::InstallForge(menu) => menu.view(),
            State::UpdateFound(menu) => menu.view(),
            State::InstallJava => widget::column!(widget::text("Downloading Java").size(20),)
                .push_maybe(self.java_recv.as_ref().map(|n| n.view()))
                .padding(10)
                .spacing(10)
                .into(),
            // TODO: maybe remove window_size argument?
            // It's not needed right now, but could be in the future.
            State::ModsDownload(menu) => menu.view(&self.images, self.window_size, self.tick_timer),
            State::LauncherSettings(menu) => menu.view(&self.config),
            State::InstallOptifine(menu) => menu.view(),
            State::ServerCreate(menu) => menu.view(),
            State::InstallPaper => {
                let dots = ".".repeat((self.tick_timer % 3) + 1);
                widget::column!(widget::text!("Installing Paper{dots}").size(20))
                    .padding(10)
                    .spacing(10)
                    .into()
            }
            State::ManagePresets(menu) => menu.view(self.window_size),
            State::ChangeLog => widget::scrollable(
                widget::column!(
                    button_with_icon(icon_manager::back(), "Skip", 16).on_press(
                        Message::LaunchScreenOpen {
                            message: None,
                            clear_selection: true
                        }
                    ),
                    changelog_0_4_1(), // changelog_0_4(), // changelog_0_3_1(),
                    button_with_icon(icon_manager::back(), "Continue", 16).on_press(
                        Message::LaunchScreenOpen {
                            message: None,
                            clear_selection: true
                        }
                    ),
                )
                .padding(10)
                .spacing(10),
            )
            .height(Length::Fill)
            .into(),
            State::Welcome(menu) => menu.view(&self.config),
            State::EditJarMods(menu) => menu.view(self.selected_instance.as_ref().unwrap()),
            State::ImportModpack(progress) => {
                widget::column![widget::text("Installing mods..."), progress.view()]
                    .padding(10)
                    .spacing(10)
                    .into()
            }
            State::LoginElyBy(menu) => menu.view(self.tick_timer),
            State::CurseforgeManualDownload(menu) => menu.view(),
            State::ExportInstance(menu) => menu.view(self.tick_timer),
            State::License(menu) => menu.view(),
        }
    }
}
