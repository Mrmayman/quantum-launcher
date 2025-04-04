use std::collections::HashMap;

use iced::widget;
use ql_core::{LogType, IS_ARM_LINUX, LAUNCHER_VERSION_NAME};

use crate::{
    icon_manager,
    launcher_state::{
        CreateInstanceMessage, InstanceLog, LaunchTabId, Launcher, ManageModsMessage, MenuLaunch,
        Message, State, NEW_ACCOUNT_NAME, OFFLINE_ACCOUNT_NAME,
    },
    menu_renderer::DISCORD,
    message_handler::SIDEBAR_DRAG_LEEWAY,
    stylesheet::{color::Color, styles::LauncherTheme, widgets::StyleButton},
    DEBUG_LOG_BUTTON_HEIGHT,
};

use super::{button_with_icon, Element};

pub const TAB_HEIGHT: f32 = 31.0;

impl Launcher {
    pub fn view_main_menu<'element>(
        &'element self,
        menu: &'element MenuLaunch,
    ) -> Element<'element> {
        let selected_instance_s = self.selected_instance.as_ref().map(|n| n.get_name());

        let difference = self.mouse_pos.0 - f32::from(menu.sidebar_width);
        let hovered = difference < SIDEBAR_DRAG_LEEWAY && difference > 0.0;

        widget::row!(
            self.get_sidebar(selected_instance_s, menu),
            self.get_tab(selected_instance_s, menu)
        )
        .spacing(if hovered || menu.sidebar_dragging {
            2
        } else {
            0
        })
        .into()
    }

    fn get_tab<'a>(
        &'a self,
        selected_instance_s: Option<&'a str>,
        menu: &'a MenuLaunch,
    ) -> Element<'a> {
        let tab_selector = get_tab_selector(selected_instance_s, menu);

        let last_parts = widget::column![
            widget::horizontal_space(),
            widget::row![
                widget::column![
                    widget::vertical_space(),
                    if menu.is_viewing_server {
                        widget::button("View Instances...").on_press(Message::LaunchScreenOpen {
                            message: None,
                            clear_selection: true,
                        })
                    } else {
                        widget::button("View Servers...").on_press(Message::ServerManageOpen {
                            selected_server: None,
                            message: None,
                        })
                    },
                ],
                get_footer_text(menu),
            ],
        ]
        .spacing(5);

        let tab_body = if let Some(selected) = &self.selected_instance {
            match menu.tab {
                LaunchTabId::Buttons => {
                    let main_buttons = widget::row![
                        if menu.is_viewing_server {
                            self.get_server_play_button(selected_instance_s)
                        } else {
                            self.get_client_play_button(selected_instance_s)
                        },
                        get_mods_button(selected_instance_s),
                        self.get_files_button(selected_instance_s),
                    ]
                    .spacing(5)
                    .wrap();

                    widget::column!(
                        main_buttons,
                        widget::horizontal_rule(10)
                            .style(|n: &LauncherTheme| n.style_rule(Color::SecondDark, 2)),
                        last_parts,
                    )
                    .padding(10)
                    .spacing(5)
                    .into()
                }
                LaunchTabId::Log => self
                    .get_log_pane(
                        if menu.is_viewing_server {
                            &self.server_logs
                        } else {
                            &self.client_logs
                        },
                        selected_instance_s,
                        menu.is_viewing_server,
                    )
                    .into(),
                LaunchTabId::Edit => {
                    if let Some(menu) = &menu.edit_instance {
                        menu.view(selected)
                    } else {
                        widget::column!("Loading...").padding(10).spacing(10).into()
                    }
                }
            }
        } else {
            widget::column!("Select an instance", last_parts)
                .padding(10)
                .spacing(10)
                .into()
        };

        widget::column!(tab_selector, tab_body).spacing(5).into()
    }

    pub fn get_log_pane<'element>(
        &'element self,
        logs: &'element HashMap<String, InstanceLog>,
        selected_instance: Option<&'element str>,
        is_server: bool,
    ) -> widget::Column<'element, Message, LauncherTheme> {
        // const LOG_VIEW_LIMIT: usize = 20000;

        let (scroll, sidebar_width) = if let State::Launch(MenuLaunch {
            log_scroll,
            sidebar_width,
            ..
        }) = &self.state
        {
            (*log_scroll, *sidebar_width)
        } else {
            (0, 0)
        };

        if let Some(Some(InstanceLog { log, has_crashed, command })) = selected_instance
            .as_ref()
            .map(|selection| logs.get(*selection))
        {
            const TEXT_SIZE: f32 = 12.0;

            let log_new: Vec<(String, LogType)> = log.iter().map(|n| (n.clone(), LogType::Point)).collect();
            let height_reduction = self.window_size.1 / 3.0 /*+ if self.is_log_open { self.window_size.1 / 2.0 } else { 0.0 }*/;

            let (text_len, column) =
                self.view_launcher_log(&log_new,
                    TEXT_SIZE,
                    scroll,
                    sidebar_width as f32 + 16.0,
                    height_reduction
                );

            // TODO: Make scrolling precise when bottom launcher log bar is open
            let screen_height_lines = (self.window_size.1 - height_reduction - 70.0) as f64 / 18.0;
            let new_text_len = text_len - screen_height_lines;

            let log = widget::mouse_area(
                widget::container(widget::row![
                    column,
                    widget::vertical_slider(
                        0.0..=new_text_len,
                        new_text_len - scroll as f64,
                        move |val| { Message::LaunchLogScrollAbsolute(new_text_len.ceil() as i64 - val as i64) }
                    )
                ])
                .style(|n: &LauncherTheme| n.style_container_sharp_box(0.0, Color::ExtraDark)),
            )
            .on_scroll(move |n| {
                let lines = match n {
                    iced::mouse::ScrollDelta::Lines { y, .. } => y as i64,
                    iced::mouse::ScrollDelta::Pixels { y, .. } => (y / TEXT_SIZE) as i64,
                };
                Message::LaunchLogScroll(lines)
            });

            widget::column!(
                widget::row!(
                    widget::button("Copy Log").on_press(if is_server {Message::ServerManageCopyLog} else {Message::LaunchCopyLog}),
                    widget::text("Having issues? Copy and send the game log for support").size(12),
                ).spacing(10),
                if *has_crashed {
                    widget::column!(
                        widget::text!("The {} has crashed!", if is_server {"server"} else {"game"}).size(14),
                        widget::text("Go to Edit -> Enable Logging (disable it) then launch the game again.").size(12),
                        widget::text("Then copy the text in the second terminal window for crash information").size(12),
                        log
                    )
                } else if is_server {
                    widget::column!(
                        widget::text_input("Enter command...", command)
                            .on_input(move |n| Message::ServerManageEditCommand(selected_instance.unwrap().to_owned(), n))
                            .on_submit(Message::ServerManageSubmitCommand(selected_instance.unwrap().to_owned()))
                            .width(190),
                        log
                    )
                } else {
                    widget::column![
                        log,
                    ]
                },
            )
        } else {
            get_no_instance_message()
        }
        .padding(10)
        .spacing(10)
    }

    fn get_sidebar<'a>(
        &'a self,
        selected_instance_s: Option<&'a str>,
        menu: &'a MenuLaunch,
    ) -> Element<'a> {
        let difference = self.mouse_pos.0 - f32::from(menu.sidebar_width);

        let list = if menu.is_viewing_server {
            self.server_list.as_deref()
        } else {
            self.client_list.as_deref()
        };

        let is_hovered = difference < SIDEBAR_DRAG_LEEWAY
            && difference > 0.0
            && (!self.is_log_open || (self.mouse_pos.1 < self.window_size.1 / 2.0));

        widget::container(
            widget::row!(if let Some(instances) = list {
                widget::column![
                    widget::button(
                        widget::row![icon_manager::create(), widget::text("New").size(16)]
                            .align_y(iced::alignment::Vertical::Center)
                            .height(TAB_HEIGHT - 10.0)
                            .spacing(10),
                    )
                    .style(|n, status| n.style_button(status, StyleButton::FlatDark))
                    .on_press(if menu.is_viewing_server {
                        Message::ServerCreateScreenOpen
                    } else {
                        Message::CreateInstance(CreateInstanceMessage::ScreenOpen)
                    })
                    .width(menu.sidebar_width),
                    widget::scrollable(widget::column(instances.iter().map(|name| {
                        let text = widget::text(name).size(16);
                        if selected_instance_s == Some(name) {
                            widget::container(widget::row!(widget::Space::with_width(5), text))
                                .style(LauncherTheme::style_container_selected_flat_button)
                                .width(menu.sidebar_width)
                                .padding(5)
                                .into()
                        } else {
                            widget::button(text)
                                .style(|n: &LauncherTheme, status| {
                                    n.style_button(status, StyleButton::FlatExtraDark)
                                })
                                .on_press(if menu.is_viewing_server {
                                    Message::ServerManageSelectedServer(name.clone())
                                } else {
                                    Message::LaunchInstanceSelected(name.clone())
                                })
                                .width(menu.sidebar_width)
                                .into()
                        }
                    })))
                    .height(
                        (self.window_size.1 / if self.is_log_open { 2.0 } else { 1.0 })
                            - TAB_HEIGHT
                            - if self.accounts_selected.as_deref() == Some(OFFLINE_ACCOUNT_NAME) {
                                115.0
                            } else {
                                80.0
                            }
                            - DEBUG_LOG_BUTTON_HEIGHT
                    )
                    .style(LauncherTheme::style_scrollable_flat_extra_dark),
                    widget::vertical_space(),
                    self.get_accounts_bar(menu),
                ]
                .spacing(5)
            } else {
                widget::column!["Loading..."]
            }
            .width(menu.sidebar_width))
            .push_maybe(is_hovered.then_some(
                widget::vertical_rule(0).style(|n: &LauncherTheme| n.style_rule(Color::Mid, 4)),
            )),
        )
        .style(|n| n.style_container_sharp_box(0.0, Color::ExtraDark))
        .into()
    }

    fn get_accounts_bar(&self, menu: &MenuLaunch) -> Element {
        let something_is_happening =
            self.java_recv.is_some() || menu.asset_recv.is_some() || menu.login_progress.is_some();

        let dropdown: Element = if something_is_happening {
            widget::text_input("", self.accounts_selected.as_deref().unwrap_or_default())
                .width(menu.sidebar_width - 10)
                .into()
        } else {
            widget::pick_list(
                self.accounts_dropdown.clone(),
                self.accounts_selected.clone(),
                Message::AccountSelected,
            )
            .width(menu.sidebar_width - 10)
            .into()
        };

        widget::column![
            widget::row![
                widget::text(" Accounts:").size(14),
                widget::horizontal_space(),
            ]
            .push_maybe(
                self.is_account_selected().then_some(
                    widget::button(widget::text("Logout").size(11))
                        .padding(iced::Padding {
                            top: 3.0,
                            right: 8.0,
                            bottom: 3.0,
                            left: 8.0
                        })
                        .on_press(Message::AccountLogoutCheck)
                        .style(|n: &LauncherTheme, status| n
                            .style_button(status, StyleButton::FlatExtraDark))
                )
            )
            .width(menu.sidebar_width - 10),
            dropdown
        ]
        .push_maybe(
            (self.accounts_selected.as_deref() == Some(OFFLINE_ACCOUNT_NAME)).then_some(
                widget::text_input("Enter username...", &self.config.username)
                    .on_input(Message::LaunchUsernameSet)
                    .width(menu.sidebar_width - 10),
            ),
        )
        .padding(5)
        .spacing(5)
        .into()
    }

    pub fn is_account_selected(&self) -> bool {
        !(self.accounts_selected.is_none()
            || self.accounts_selected.as_deref() == Some(NEW_ACCOUNT_NAME)
            || self.accounts_selected.as_deref() == Some(OFFLINE_ACCOUNT_NAME))
    }

    fn get_client_play_button(&self, selected_instance: Option<&str>) -> Element {
        let play_button = button_with_icon(icon_manager::play(), "Play", 16).width(98);

        let is_account_selected = self.is_account_selected();

        let play_button = if self.config.username.is_empty() && !is_account_selected {
            widget::column![widget::tooltip(
                play_button,
                "Username is empty!",
                widget::tooltip::Position::FollowCursor,
            )
            .style(|n| n.style_container_sharp_box(0.0, Color::ExtraDark))]
        } else if self.config.username.contains(' ') && !is_account_selected {
            widget::column![widget::tooltip(
                play_button,
                "Username contains spaces!",
                widget::tooltip::Position::FollowCursor,
            )
            .style(|n| n.style_container_sharp_box(0.0, Color::ExtraDark))]
        } else if let Some(selected_instance) = selected_instance {
            widget::column![if self.client_processes.contains_key(selected_instance) {
                button_with_icon(icon_manager::play(), "Kill", 16)
                    .on_press(Message::LaunchKill)
                    .width(98)
            } else {
                play_button.on_press(Message::LaunchStart)
            }]
        } else {
            widget::column![widget::tooltip(
                play_button,
                "Select an instance first!",
                widget::tooltip::Position::FollowCursor,
            )
            .style(|n| n.style_container_sharp_box(0.0, Color::ExtraDark))]
        };
        play_button.into()
    }

    fn get_files_button<'a>(
        &self,
        selected_instance: Option<&'a str>,
    ) -> widget::Button<'a, Message, LauncherTheme> {
        button_with_icon(icon_manager::folder(), "Files", 16)
            .on_press_maybe((selected_instance.is_some()).then(|| {
                Message::CoreOpenDir(
                    self.dir
                        .join("instances")
                        .join(selected_instance.as_ref().unwrap())
                        .join(".minecraft")
                        .to_str()
                        .unwrap()
                        .to_owned(),
                )
            }))
            .width(97)
    }

    fn get_server_play_button<'a>(&self, selected_server: Option<&'a str>) -> Element<'a> {
        if selected_server.is_some_and(|n| self.server_processes.contains_key(n)) {
            button_with_icon(icon_manager::play(), "Stop", 16)
                .width(97)
                .on_press_maybe(
                    (selected_server.is_some()).then(|| {
                        Message::ServerManageKillServer(selected_server.unwrap().to_owned())
                    }),
                )
                .into()
        } else {
            widget::tooltip(
                button_with_icon(icon_manager::play(), "Start", 16)
                    .width(97)
                    .on_press_maybe((selected_server.is_some()).then(|| {
                        Message::ServerManageStartServer(selected_server.unwrap().to_owned())
                    })),
                "By starting the server, you agree to the EULA",
                widget::tooltip::Position::FollowCursor,
            )
            .style(|n: &LauncherTheme| n.style_container_sharp_box(0.0, Color::ExtraDark))
            .into()
        }
    }
}

fn get_tab_selector<'a>(selected_instance_s: Option<&'a str>, menu: &'a MenuLaunch) -> Element<'a> {
    let tab_bar = widget::row(
        [LaunchTabId::Buttons, LaunchTabId::Edit, LaunchTabId::Log]
            .into_iter()
            .map(|n| render_tab(n, menu)),
    )
    .wrap();

    let n = widget::row!(
        widget::button(
            widget::row![
                widget::horizontal_space(),
                icon_manager::settings(),
                widget::horizontal_space()
            ]
            .align_y(iced::Alignment::Center)
        )
        .height(TAB_HEIGHT)
        .width(TAB_HEIGHT)
        .style(|n, status| n.style_button(status, StyleButton::FlatExtraDark))
        .on_press(Message::LauncherSettingsOpen),
        tab_bar,
        widget::horizontal_space()
    );

    let n = if let Some(select) = selected_instance_s {
        n.push(
            widget::column!(
                widget::vertical_space(),
                widget::text!("{select}  ").size(14),
                widget::vertical_space()
            )
            .height(TAB_HEIGHT),
        )
    } else {
        n
    };

    widget::container(n)
        .style(|n| n.style_container_sharp_box(0.0, Color::ExtraDark))
        .into()
}

fn get_mods_button(
    selected_instance_s: Option<&str>,
) -> widget::Button<'_, Message, LauncherTheme> {
    button_with_icon(icon_manager::download(), "Mods", 15)
        .on_press_maybe(
            (selected_instance_s.is_some())
                .then_some(Message::ManageMods(ManageModsMessage::ScreenOpen)),
        )
        .width(98)
}

fn render_tab(n: LaunchTabId, menu: &MenuLaunch) -> Element {
    let txt = widget::row!(
        widget::horizontal_space(),
        widget::text(n.to_string()),
        widget::horizontal_space(),
    );
    if menu.tab == n {
        widget::container(txt)
            .style(LauncherTheme::style_container_selected_flat_button)
            .padding(5)
            .width(70)
            .height(TAB_HEIGHT)
            .into()
    } else {
        widget::button(txt)
            .style(|n, status| n.style_button(status, StyleButton::FlatExtraDark))
            .on_press(Message::LaunchChangeTab(n))
            .width(70)
            .height(TAB_HEIGHT)
            .into()
    }
}

fn get_no_instance_message<'a>() -> widget::Column<'a, Message, LauncherTheme> {
    const BASE_MESSAGE: &str = "No logs found";

    if IS_ARM_LINUX
        || cfg!(target_os = "macos")
        || cfg!(target_arch = "aarch64")
        || cfg!(target_arch = "x86")
    {
        let arm_message = widget::column!(
            widget::text(
                "Note: This version is experimental. If you want to get help join our discord"
            ),
            button_with_icon(icon_manager::chat(), "Join Discord", 16)
                .on_press(Message::CoreOpenDir(DISCORD.to_owned())),
        );
        widget::column!(BASE_MESSAGE, arm_message)
    } else {
        widget::column!(BASE_MESSAGE)
    }
}

fn get_footer_text(menu: &MenuLaunch) -> Element {
    let version_message = widget::column!(
        widget::vertical_space(),
        widget::row!(
            widget::horizontal_space(),
            widget::text!("QuantumLauncher v{LAUNCHER_VERSION_NAME}").size(12)
        ),
        widget::row!(
            widget::horizontal_space(),
            widget::text("A Minecraft Launcher by Mrmayman").size(10)
        ),
    );

    if menu.message.is_empty() {
        widget::column!(version_message)
    } else {
        widget::column!(
            widget::row!(
                widget::horizontal_space(),
                widget::container(widget::text(&menu.message).size(14))
                    .width(190)
                    .padding(10)
            ),
            version_message
        )
    }
    .spacing(10)
    .into()
}
