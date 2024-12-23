use std::collections::HashMap;

use iced::widget;
use ql_core::file_utils;

use crate::{
    icon_manager,
    launcher_state::{
        EditInstanceMessage, InstanceLog, MenuLaunch, MenuServerCreate, MenuServerManage, Message,
        ServerProcess,
    },
};

use super::{button_with_icon, Element};

impl MenuServerManage {
    pub fn view<'a>(
        &'a self,
        selected_server: Option<&'a String>,
        logs: &'a HashMap<String, InstanceLog>,
        processes: &'a HashMap<String, ServerProcess>,
    ) -> Element<'a> {
        let log_pane = MenuLaunch::get_log_pane(logs, selected_server, true);

        let button_play = Self::get_play_button(selected_server, processes);
        let button_files = Self::get_files_button(selected_server);

        let server_ops = if !self.server_list.is_empty() {
            widget::column!(
                widget::text("Select Server"),
                widget::pick_list(
                    self.server_list.as_slice(),
                    selected_server,
                    Message::ServerManageSelectedServer
                )
                .width(200),
                widget::column!(
                    widget::row!(
                        button_play,
                        button_with_icon(icon_manager::settings(), "Edit")
                            .width(98)
                            .on_press_maybe(selected_server.map(|selected_server| {
                                Message::EditInstance(EditInstanceMessage::MenuOpen(Some(
                                    selected_server.clone(),
                                )))
                            })),
                    )
                    .spacing(5),
                    widget::row!(
                        button_files,
                        button_with_icon(icon_manager::download(), "Mods").width(98),
                    )
                    .spacing(5),
                    widget::row!(
                        button_with_icon(icon_manager::delete(), "Delete")
                            .width(97)
                            .on_press_maybe((selected_server.is_some()).then(|| {
                                Message::ServerDeleteOpen(selected_server.unwrap().clone())
                            })),
                        // button_with_icon(icon_manager::settings(), "Edit"),
                    )
                    .spacing(5)
                )
                .spacing(5)
            )
            .spacing(10)
        } else {
            widget::column!(widget::text(
                "No servers found! Create a new server to get started"
            ))
        }
        .padding(10);

        widget::row!(
            widget::column!(
                button_with_icon(icon_manager::back(), "Back")
                    .on_press(Message::LaunchScreenOpen(None)),
                button_with_icon(icon_manager::create(), "New Server")
                    .on_press(Message::ServerCreateScreenOpen),
                widget::container(server_ops)
            )
            .spacing(10),
            log_pane
        )
        .padding(10)
        .spacing(10)
        .into()
    }

    fn get_play_button<'a>(
        selected_server: Option<&'a String>,
        processes: &'a HashMap<String, ServerProcess>,
    ) -> Element<'a> {
        if selected_server
            .map(|n| processes.contains_key(n))
            .unwrap_or(false)
        {
            button_with_icon(icon_manager::play(), "Stop")
                .width(97)
                .on_press_maybe(
                    (selected_server.is_some())
                        .then(|| Message::ServerManageKillServer(selected_server.unwrap().clone())),
                )
                .into()
        } else {
            widget::tooltip(
                button_with_icon(icon_manager::play(), "Start")
                    .width(97)
                    .on_press_maybe((selected_server.is_some()).then(|| {
                        Message::ServerManageStartServer(selected_server.unwrap().clone())
                    })),
                "By starting the server, you agree to the EULA",
                widget::tooltip::Position::FollowCursor,
            )
            .into()
        }
    }

    fn get_files_button(
        selected_server: Option<&String>,
    ) -> widget::Button<'_, Message, crate::stylesheet::styles::LauncherTheme> {
        button_with_icon(icon_manager::folder(), "Files")
            .width(97)
            .on_press_maybe((selected_server.is_some()).then(|| {
                let launcher_dir = file_utils::get_launcher_dir().unwrap();
                Message::CoreOpenDir(
                    launcher_dir
                        .join("servers")
                        .join(selected_server.unwrap())
                        .to_str()
                        .unwrap()
                        .to_owned(),
                )
            }))
    }
}

impl MenuServerCreate {
    pub fn view(&self) -> Element {
        match self {
            MenuServerCreate::Loading {
                progress_number, ..
            } => {
                widget::column!(
                    widget::text("Loading version list...").size(20),
                    widget::progress_bar(0.0..=16.0, *progress_number),
                    widget::text(if *progress_number >= 1.0 {
                        format!("Downloading Omniarchive list {progress_number} / 15")
                    } else {
                        "Downloading official version list".to_owned()
                    })
                )
            }
            MenuServerCreate::Loaded {
                name,
                versions,
                selected_version,
                progress_receiver: None,
                ..
            } => {
                widget::column!(
                    button_with_icon(icon_manager::back(), "Back")
                        .on_press(Message::ServerManageOpen(None)),
                    widget::text("Create new server").size(20),
                    widget::combo_box(
                        versions,
                        "Select a version...",
                        selected_version.as_ref(),
                        Message::ServerCreateVersionSelected
                    ),
                    widget::text_input("Enter server name...", name)
                        .on_input(Message::ServerCreateNameInput),
                    widget::button("Create Server").on_press_maybe(
                        (selected_version.is_some() && !name.is_empty())
                            .then(|| Message::ServerCreateStart)
                    ),
                )
            }
            MenuServerCreate::Loaded {
                progress_receiver: Some(_),
                progress_number,
                ..
            } => {
                widget::column!(
                    widget::text("Creating Server...").size(20),
                    widget::progress_bar(0.0..=3.0, *progress_number),
                    if *progress_number < 1.0 {
                        "Downloading manifest..."
                    } else if *progress_number < 2.0 {
                        "Downloading version JSON..."
                    } else {
                        "Downloading server JAR..."
                    }
                )
            }
        }
        .padding(10)
        .spacing(10)
        .into()
    }
}
