use std::collections::HashMap;

use iced::widget;
use ql_core::{InstanceSelection, IS_ARM_LINUX, LAUNCHER_VERSION_NAME};

use crate::{
    icon_manager,
    launcher_state::{
        CreateInstanceMessage, InstanceLog, LaunchTabId, Launcher, ManageModsMessage, MenuLaunch,
        Message, OFFLINE_ACCOUNT_NAME,
    },
    menu_renderer::DISCORD,
    message_handler::SIDEBAR_DRAG_LEEWAY,
    stylesheet::{color::Color, styles::LauncherTheme, widgets::StyleButton},
};

use super::{button_with_icon, dynamic_box, Element};

impl Launcher {
    pub fn view_main_menu<'element>(
        &'element self,
        menu: &'element MenuLaunch,
    ) -> Element<'element> {
        let selected_instance_s = match self.selected_instance.as_ref() {
            Some(InstanceSelection::Instance(n)) => Some(n),
            Some(InstanceSelection::Server(_)) => panic!("selected server in main instances menu"),
            None => None,
        };

        let username = &self.config.as_ref().unwrap().username;

        widget::row!(
            self.get_sidebar(username, selected_instance_s, menu),
            self.get_tab(username, selected_instance_s, menu)
        )
        .spacing(2)
        .into()
    }

    fn get_tab<'a>(
        &'a self,
        username: &'a str,
        selected_instance_s: Option<&'a String>,
        menu: &'a MenuLaunch,
    ) -> Element<'a> {
        let tab_selector: Element = {
            let (tab_bar, height) = dynamic_box(
                [LaunchTabId::Buttons, LaunchTabId::Edit, LaunchTabId::Log]
                    .into_iter()
                    .map(|n| render_tab(n, menu)),
                60.0,
                31.0,
                self.window_size.0 - menu.sidebar_width as f32,
                0.0,
                0.0,
            );

            let n = widget::row!(
                widget::button(
                    widget::row![
                        widget::horizontal_space(),
                        icon_manager::settings(),
                        widget::horizontal_space()
                    ]
                    .align_y(iced::Alignment::Center)
                )
                .height(31.0)
                .width(31.0)
                .style(|n, status| n.style_button(status, StyleButton::FlatDark))
                .on_press(Message::LauncherSettingsOpen),
                tab_bar,
                widget::horizontal_space()
            );

            let n = if let Some(select) = selected_instance_s {
                n.push(widget::column!(
                    widget::vertical_space(),
                    widget::text!("{select}  "),
                    widget::vertical_space()
                ))
            } else {
                n
            }
            .height(height);
            widget::container(n)
                .style(|n| n.style_container_sharp_box(0.0, Color::Dark))
                .into()
        };

        let mods_button = button_with_icon(icon_manager::download(), "Mods", 15)
            .on_press_maybe(
                (selected_instance_s.is_some())
                    .then_some(Message::ManageMods(ManageModsMessage::ScreenOpen)),
            )
            .width(98);

        let tab_body = if let Some(selected) = &self.selected_instance {
            match menu.tab {
                LaunchTabId::Buttons => {
                    let (main_buttons, _) = dynamic_box(
                        [
                            self.get_play_button(username, selected_instance_s).into(),
                            mods_button.into(),
                            self.get_files_button(selected_instance_s).into(),
                        ],
                        98.0,
                        0.0,
                        self.window_size.0 - menu.sidebar_width as f32,
                        0.0,
                        5.0,
                    );

                    widget::column!(
                        main_buttons,
                        widget::horizontal_rule(10)
                            .style(|n: &LauncherTheme| n.style_rule(Color::SecondDark, 2)),
                        get_servers_button(),
                        widget::horizontal_space(),
                        widget::vertical_space(),
                        get_footer_text(menu),
                    )
                    .padding(10)
                    .spacing(5)
                    .into()
                }
                LaunchTabId::Log => {
                    Self::get_log_pane(&self.client_logs, selected_instance_s, false).into()
                }
                LaunchTabId::Edit => {
                    if let Some(menu) = &menu.edit_instance {
                        menu.view(selected)
                    } else {
                        widget::column!("Loading...").padding(10).spacing(10).into()
                    }
                }
            }
        } else {
            widget::column!("Select an instance")
                .padding(10)
                .spacing(10)
                .into()
        };

        widget::column!(tab_selector, tab_body).spacing(5).into()
    }

    pub fn get_log_pane<'element>(
        logs: &'element HashMap<String, InstanceLog>,
        selected_instance: Option<&'element String>,
        is_server: bool,
    ) -> widget::Column<'element, Message, LauncherTheme> {
        const LOG_VIEW_LIMIT: usize = 10000;
        if let Some(Some(InstanceLog { log, has_crashed, command })) = selected_instance
            .as_ref()
            .map(|selection| logs.get(*selection))
        {
            let log_length = log.len();
            let slice = if log_length > LOG_VIEW_LIMIT {
                &log[log_length - LOG_VIEW_LIMIT..log_length]
            } else {
                log
            };
            let log = widget::scrollable(
                widget::text(slice)
                    .size(12)
                    .font(iced::Font::with_name("JetBrains Mono"))
            );
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
                            .on_input(move |n| Message::ServerManageEditCommand(selected_instance.unwrap().clone(), n))
                            .on_submit(Message::ServerManageSubmitCommand(selected_instance.unwrap().clone()))
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
        username: &'a str,
        selected_instance_s: Option<&'a String>,
        menu: &'a MenuLaunch,
    ) -> Element<'a> {
        let difference = self.mouse_pos.0 - menu.sidebar_width as f32;

        widget::container(
            widget::row!(if let Some(instances) = self.client_list.as_deref() {
                widget::column!(widget::scrollable(
                    widget::column!(
                        self.get_accounts_bar(menu, username),
                        button_with_icon(icon_manager::create(), "New", 16)
                            .style(|n, status| n.style_button(status, StyleButton::Flat))
                            .on_press(Message::CreateInstance(CreateInstanceMessage::ScreenOpen))
                            .width(menu.sidebar_width),
                        widget::column(instances.iter().map(|name| {
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
                                        n.style_button(status, StyleButton::Flat)
                                    })
                                    .on_press(Message::LaunchInstanceSelected(name.clone()))
                                    .width(menu.sidebar_width)
                                    .into()
                            }
                        })),
                    )
                    .spacing(5),
                )
                .style(LauncherTheme::style_scrollable_flat))
            } else {
                widget::column!("Loading...")
            }
            .push(widget::vertical_space()))
            .push_maybe(
                (difference < SIDEBAR_DRAG_LEEWAY && difference > 0.0).then_some(
                    widget::vertical_rule(0).style(|n: &LauncherTheme| n.style_rule(Color::Mid, 4)),
                ),
            ),
        )
        .style(|n| n.style_container_sharp_box(0.0, Color::Dark))
        .into()
    }

    fn get_accounts_bar(&self, menu: &MenuLaunch, username: &str) -> Element {
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

        widget::column!("Accounts", dropdown)
            .push_maybe(
                (self.accounts_selected.as_deref() == Some(OFFLINE_ACCOUNT_NAME)).then_some(
                    widget::text_input("Enter username...", username)
                        .on_input(Message::LaunchUsernameSet)
                        .width(menu.sidebar_width - 10),
                ),
            )
            .padding(5)
            .spacing(5)
            .into()
    }

    fn get_play_button<'a>(
        &self,
        username: &'a str,
        selected_instance: Option<&'a String>,
    ) -> widget::Column<'a, Message, LauncherTheme> {
        let play_button = button_with_icon(icon_manager::play(), "Play", 16).width(98);

        let play_button = if username.is_empty() {
            widget::column!(widget::tooltip(
                play_button,
                "Username is empty!",
                widget::tooltip::Position::FollowCursor,
            )
            .style(|n| n.style_container_sharp_box(0.0, Color::Black)))
        } else if username.contains(' ') {
            widget::column!(widget::tooltip(
                play_button,
                "Username contains spaces!",
                widget::tooltip::Position::FollowCursor,
            )
            .style(|n| n.style_container_sharp_box(0.0, Color::Black)))
        } else if let Some(selected_instance) = selected_instance {
            widget::column!(if self.client_processes.contains_key(selected_instance) {
                button_with_icon(icon_manager::play(), "Kill", 16)
                    .on_press(Message::LaunchKill)
                    .width(98)
            } else {
                play_button.on_press(Message::LaunchStart)
            })
        } else {
            widget::column!(widget::tooltip(
                play_button,
                "Select an instance first!",
                widget::tooltip::Position::FollowCursor,
            )
            .style(|n| n.style_container_sharp_box(0.0, Color::Black)))
        };
        play_button
    }

    fn get_files_button<'a>(
        &self,
        selected_instance: Option<&'a String>,
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
            .width(60)
            .into()
    } else {
        widget::button(txt)
            .style(|n, status| n.style_button(status, StyleButton::Flat))
            .on_press(Message::LaunchChangeTab(n))
            .width(60)
            .into()
    }
}

fn get_no_instance_message<'a>() -> widget::Column<'a, Message, LauncherTheme> {
    const BASE_MESSAGE: &str = "No logs found";

    if IS_ARM_LINUX || cfg!(target_os = "macos") {
        let arm_message = widget::column!(
            widget::text(
                "Note: This version is VERY experimental. If you want to get help join our discord"
            ),
            button_with_icon(icon_manager::chat(), "Join our Discord", 16)
                .on_press(Message::CoreOpenDir(DISCORD.to_owned())),
        );
        widget::column!(BASE_MESSAGE, arm_message)
    } else {
        widget::column!(BASE_MESSAGE)
    }
}

fn get_servers_button<'a>() -> Element<'a> {
    button_with_icon(icon_manager::page(), "Servers", 14)
        .on_press(Message::ServerManageOpen {
            selected_server: None,
            message: None,
        })
        .into()
}

fn get_footer_text(menu: &MenuLaunch) -> Element {
    let version_message = widget::column!(
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
