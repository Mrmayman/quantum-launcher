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
    stylesheet::{
        color::Color,
        styles::{LauncherTheme, StyleButton, StyleContainer, StyleFlatness, StyleRule},
    },
};

use super::{button_with_icon, Element};

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
            let n = widget::row!(
                widget::button(
                    widget::row![
                        widget::horizontal_space(),
                        icon_manager::settings(),
                        widget::horizontal_space()
                    ]
                    .align_items(iced::Alignment::Center)
                )
                .height(31.0)
                .width(31.0)
                .style(StyleButton::FlatDark)
                .on_press(Message::LauncherSettingsOpen),
                widget::row(
                    [LaunchTabId::Buttons, LaunchTabId::Edit, LaunchTabId::Log]
                        .into_iter()
                        .map(|n| {
                            let txt = widget::row!(
                                widget::horizontal_space(),
                                widget::text(n.to_string()),
                                widget::horizontal_space(),
                            );
                            if menu.tab == n {
                                widget::container(txt)
                                    .style(StyleContainer::SelectedFlatButton)
                                    .padding(5)
                                    .width(60)
                                    .into()
                            } else {
                                widget::button(txt)
                                    .style(StyleButton::Flat)
                                    .on_press(Message::LaunchChangeTab(n))
                                    .width(60)
                                    .into()
                            }
                        }),
                ),
                widget::horizontal_space()
            );
            let n = if let Some(select) = selected_instance_s {
                n.push(widget::column!(
                    widget::vertical_space(),
                    widget::text(format!("{select}  ")),
                    widget::vertical_space()
                ))
            } else {
                n
            }
            .height(31);
            widget::container(n)
                .style(StyleContainer::SharpBox(Color::Dark, 0.0))
                .into()
        };

        let mods_button = button_with_icon(icon_manager::download(), "Mods")
            .on_press_maybe(
                (selected_instance_s.is_some())
                    .then_some(Message::ManageMods(ManageModsMessage::ScreenOpen)),
            )
            .width(98);

        let tab_body = if let Some(selected) = &self.selected_instance {
            match menu.tab {
                LaunchTabId::Buttons => {
                    let main_buttons: Element =
                        if self.window_size.0 < 220 + menu.sidebar_width as u32 {
                            widget::column!(
                                self.get_play_button(username, selected_instance_s),
                                mods_button,
                                self.get_files_button(selected_instance_s),
                            )
                            .spacing(5)
                            .into()
                        } else if self.window_size.0 < 320 + menu.sidebar_width as u32 {
                            widget::column!(
                                widget::row!(
                                    self.get_play_button(username, selected_instance_s,),
                                    mods_button,
                                )
                                .spacing(5),
                                self.get_files_button(selected_instance_s),
                            )
                            .spacing(5)
                            .into()
                        } else {
                            widget::row![
                                self.get_play_button(username, selected_instance_s),
                                mods_button,
                                self.get_files_button(selected_instance_s),
                            ]
                            .spacing(5)
                            .into()
                        };

                    widget::column!(
                        main_buttons,
                        widget::horizontal_rule(10),
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
        logs: &HashMap<String, InstanceLog>,
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
                        widget::text(format!("The {} has crashed!", if is_server {"server"} else {"game"})).size(14),
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
        &self,
        username: &'a str,
        selected_instance_s: Option<&'a String>,
        menu: &'a MenuLaunch,
    ) -> Element<'a> {
        let difference = self.mouse_pos.0 - menu.sidebar_width as f32;

        widget::container(
            widget::row!(if let Some(instances) = self.client_list.as_deref() {
                widget::column!(widget::scrollable(
                    widget::column!(
                        widget::column!(
                            "Accounts",
                            widget::pick_list(
                                self.accounts_dropdown.clone(),
                                self.accounts_selected.clone(),
                                Message::HomeAccountSelected
                            )
                            .width(menu.sidebar_width - 10)
                        )
                        .push_maybe(
                            (self.accounts_selected.as_deref() == Some(OFFLINE_ACCOUNT_NAME))
                                .then_some(
                                    widget::text_input("Enter username...", username)
                                        .on_input(Message::LaunchUsernameSet)
                                        .width(menu.sidebar_width - 10)
                                )
                        )
                        .height(110)
                        .padding(5)
                        .spacing(5),
                        button_with_icon(icon_manager::create(), "New")
                            .style(StyleButton::Flat)
                            .on_press(Message::CreateInstance(CreateInstanceMessage::ScreenOpen))
                            .width(menu.sidebar_width),
                        widget::column(instances.iter().map(|name| {
                            if selected_instance_s == Some(name) {
                                widget::container(widget::text(name))
                                    .style(StyleContainer::SelectedFlatButton)
                                    .width(menu.sidebar_width)
                                    .padding(5)
                                    .into()
                            } else {
                                widget::button(widget::text(name).size(16))
                                    .style(StyleButton::Flat)
                                    .on_press(Message::LaunchInstanceSelected(name.clone()))
                                    .width(menu.sidebar_width)
                                    .into()
                            }
                        })),
                    )
                    .spacing(5),
                )
                .style(StyleFlatness::Flat),)
            } else {
                widget::column!("Loading...")
            }
            .push(widget::vertical_space()))
            .push_maybe(
                (difference < SIDEBAR_DRAG_LEEWAY && difference > 0.0).then_some(
                    widget::vertical_rule(0).style(StyleRule {
                        thickness: 4,
                        color: Color::Mid,
                    }),
                ),
            ),
        )
        .style(StyleContainer::SharpBox(Color::Dark, 0.0))
        .into()
    }

    fn get_play_button<'a>(
        &self,
        username: &'a str,
        selected_instance: Option<&'a String>,
    ) -> widget::Column<'a, Message, LauncherTheme> {
        let play_button = button_with_icon(icon_manager::play(), "Play").width(98);

        let play_button = if username.is_empty() {
            widget::column!(widget::tooltip(
                play_button,
                "Username is empty!",
                widget::tooltip::Position::FollowCursor,
            ))
        } else if username.contains(' ') {
            widget::column!(widget::tooltip(
                play_button,
                "Username contains spaces!",
                widget::tooltip::Position::FollowCursor,
            ))
        } else if let Some(selected_instance) = selected_instance {
            widget::column!(if self.client_processes.contains_key(selected_instance) {
                button_with_icon(icon_manager::play(), "Kill")
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
            ))
        };
        play_button
    }

    fn get_files_button<'a>(
        &self,
        selected_instance: Option<&'a String>,
    ) -> widget::Button<'a, Message, LauncherTheme> {
        button_with_icon(icon_manager::folder(), "Files")
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

fn get_no_instance_message<'a>() -> widget::Column<'a, Message, LauncherTheme> {
    const BASE_MESSAGE: &str = "No logs found";

    if IS_ARM_LINUX || cfg!(target_os = "macos") {
        let arm_message = widget::column!(
            widget::text(
                "Note: This version is VERY experimental. If you want to get help join our discord"
            ),
            button_with_icon(icon_manager::chat(), "Join our Discord")
                .on_press(Message::CoreOpenDir(DISCORD.to_owned())),
        );
        widget::column!(BASE_MESSAGE, arm_message)
    } else {
        widget::column!(BASE_MESSAGE)
    }
}

fn get_servers_button<'a>() -> Element<'a> {
    widget::button(
        widget::row![icon_manager::page(), widget::text("Servers").size(14)]
            .spacing(10)
            .padding(5),
    )
    .width(98)
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
            widget::text(format!("QuantumLauncher v{LAUNCHER_VERSION_NAME}")).size(12)
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
