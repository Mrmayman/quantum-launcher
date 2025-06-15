use iced::{widget, Length};
use ql_core::LOGGER;

use crate::{
    icon_manager,
    menu_renderer::{
        back_button, button_with_icon, changelog::changelog_0_4_1, view_account_login, Element,
        DISCORD,
    },
    state::{AccountMessage, Launcher, Message, State},
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
                    logger.text.clone()
                } else {
                    Vec::new()
                }
            };

            let (text_len, column) = self.view_launcher_log(
                &text,
                TEXT_SIZE,
                self.log_scroll,
                0.0,
                self.window_size.1 / 2.0,
            );

            widget::mouse_area(
                widget::container(widget::row![
                    widget::column!(column).height(self.window_size.1 / 2.0),
                    widget::vertical_slider(
                        0.0..=text_len,
                        text_len - self.log_scroll as f64,
                        move |val| {
                            Message::CoreLogScrollAbsolute(text_len as isize - val as isize)
                        }
                    )
                ])
                .style(|n: &LauncherTheme| n.style_container_sharp_box(0.0, Color::ExtraDark)),
            )
            .on_scroll(move |n| {
                let lines = match n {
                    iced::mouse::ScrollDelta::Lines { y, .. } => y as isize,
                    iced::mouse::ScrollDelta::Pixels { y, .. } => (y / TEXT_SIZE) as isize,
                };
                Message::CoreLogScroll(lines)
            })
        }))
        .into()
    }

    fn view_menu(&self) -> Element {
        match &self.state {
            State::Launch(menu) => self.view_main_menu(menu),
            State::AccountLoginProgress(progress) => widget::column![
                widget::text("Logging into microsoft account").size(20),
                progress.view()
            ]
            .spacing(10)
            .padding(10)
            .into(),
            State::GenericMessage(msg) => widget::column![widget::text(msg)].padding(10).into(),
            State::MSAccountLogin { url, code, .. } => view_account_login(url, code),
            State::AccountLogin => widget::column![
                back_button().on_press(Message::LaunchScreenOpen {
                    message: None,
                    clear_selection: false
                }),
                widget::vertical_space(),
                widget::row![
                    widget::horizontal_space(),
                    widget::column![
                        widget::text("Login").size(20),
                        widget::button("Login with Microsoft")
                            .on_press(Message::Account(AccountMessage::OpenMicrosoft)),
                        widget::button("Login with ely.by")
                            .on_press(Message::Account(AccountMessage::OpenElyBy)),
                    ]
                    .align_x(iced::Alignment::Center)
                    .spacing(5),
                    widget::horizontal_space(),
                ],
                widget::vertical_space(),
            ]
            .padding(10)
            .spacing(5)
            .into(),
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
            State::ElyByLogin(menu) => menu.view(),
            State::CurseforgeManualDownload(menu) => menu.view(),
            State::ExportInstance(menu) => menu.view(self.tick_timer),
        }
    }
}
