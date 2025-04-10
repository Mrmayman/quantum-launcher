/*
QuantumLauncher
Copyright (C) 2024  Mrmayman & Contributors

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program.  If not, see <http://www.gnu.org/licenses/>.
*/

//! QuantumLauncher is a Minecraft launcher written in Rust using the Iced GUI framework.
//!
//! For more information see the `../../README.md` file.
//!
//! This section will mainly focus on what the
//! codebase is like for any potential contributors.
//!
//! # Crate Structure
//! - `quantum_launcher` - The GUI frontend
//! - `ql_instances` - Instance management, updating and launching
//! - `ql_mod_manager` - Mod management and installation
//! - `ql_servers` - A self-hosted server management system (incomplete)
//! - `ql_core` - Core utilities and shared code
//! - `ql_java_handler` - A library to auto-install and provide java runtimes
//!
//! # Brief Overview of the codebase
//! The architecture of the launcher is based on the
//! Model-View-Controller pattern (AKA the thing used in iced).
//!
//! - The `Launcher` struct is the main controller of the application.
//! - `view()` renders the app's view based on the current state.
//! - `update()` processes messages and updates the state accordingly.
//!
//! So it's a back-and-forth between `Message`s coming from interaction,
//! and code to deal with the messages in `update()`.
//!
//! # What are `*_w()` functions?
//! Functions ending in `_w` take in arguments as owned objects.
//! For example, `String` instead of `&str` or `Vec<T>` instead
//! of `&[T]`
//!
//! They also return errors as `String` instead of the actual error type.
//! This is done to make use with `iced::Command` easier.
//!
//! # Comments
//! I tend to be loose, for better or for worse,
//! when it comes to using comments.
//!
//! Have something complicated-looking that could
//! be better explained? Add comments. Clippy bugging you
//! about not documenting something? Add doc comments.
//!
//! **The only rule of thumb is: Do it well or don't do it**.
//! Half-baked useless comments are worse than no comments
//! (yes I'm guilty of this sometimes).
//!
//! Heck, feel free to make it informal if that seems better.
//! (maybe add a `WTF: ` tag so people can search for it for fun).
//!
//! Btw, if you have any questions, feel free to ask me on discord!

#![deny(unsafe_code)]

use std::{sync::Arc, time::Duration};

use arguments::{cmd_list_available_versions, cmd_list_instances, PrintCmd};
use iced::{Settings, Task};
use launcher_state::{
    get_entries, LaunchTabId, Launcher, ManageModsMessage, MenuLaunch, MenuLauncherUpdate,
    MenuServerCreate, Message, ProgressBar, SelectedState, ServerProcess, State,
};

use ql_core::{
    err, file_utils, info, info_no_log, open_file_explorer, InstanceSelection, IntoStringError,
    Loader,
};
use ql_instances::UpdateCheckInfo;
use ql_mod_manager::loaders;
use stylesheet::styles::{LauncherTheme, LauncherThemeColor, LauncherThemeLightness};
use tokio::io::AsyncWriteExt;

mod arguments;
/// Launcher configuration
mod config;
/// Icon definitions as `iced::widget`
mod icon_manager;
mod launcher_state;
/// Code to render menus
mod menu_renderer;
mod message_handler;
mod message_update;
/// Handles mod store
mod mods_store;
/// Stylesheet definitions (launcher themes)
mod stylesheet;
/// Code to tick every frame
mod tick;
mod view;

const LAUNCHER_ICON: &[u8] = include_bytes!("../../assets/icon/ql_logo.ico");

impl Launcher {
    fn new() -> (Self, iced::Task<Message>) {
        let check_for_updates_command = Task::perform(
            async move { ql_instances::check_for_launcher_updates().await.strerr() },
            Message::UpdateCheckResult,
        );

        let is_new_user = file_utils::is_new_user();
        // let is_new_user = true;

        let get_entries_command = Task::perform(
            get_entries("instances".to_owned(), false),
            Message::CoreListLoaded,
        );

        (
            Launcher::load_new(None, is_new_user).unwrap_or_else(Launcher::with_error),
            Task::batch([check_for_updates_command, get_entries_command]),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Nothing => {}

            Message::CoreTickConfigSaved(result)
            | Message::LaunchKillEnd(result)
            | Message::UpdateDownloadEnd(result) => {
                if let Err(err) = result {
                    self.set_error(err);
                }
            }

            Message::ServerCreateEnd(Err(err))
            | Message::ServerCreateVersionsLoaded(Err(err))
            | Message::UninstallLoaderEnd(Err(err))
            | Message::ServerManageStartServerFinish(Err(err))
            | Message::InstallForgeEnd(Err(err))
            | Message::LaunchEndedLog(Err(err))
            | Message::ServerManageEndedLog(Err(err))
            | Message::CoreListLoaded(Err(err))
            | Message::UpdateCheckResult(Err(err)) => self.set_error(err),

            Message::Account(msg) => return self.update_account(msg),
            Message::ManageMods(message) => return self.update_manage_mods(message),
            Message::LaunchInstanceSelected { name, is_server } => {
                self.selected_instance = Some(InstanceSelection::new(&name, is_server));

                if let State::Launch(MenuLaunch { .. }) = self.state {
                    if let Err(err) = self.edit_instance() {
                        err!("Could not open edit instance menu: {err}");
                        if let State::Launch(MenuLaunch { edit_instance, .. }) = &mut self.state {
                            *edit_instance = None;
                        }
                    }
                }
            }
            Message::LaunchUsernameSet(username) => {
                self.config.username = username;
            }
            Message::LaunchStart => return self.launch_start(),
            Message::LaunchEnd(result) => return self.finish_launching(result),
            Message::CreateInstance(message) => return self.update_create_instance(message),
            Message::DeleteInstanceMenu => self.go_to_delete_instance_menu(),
            Message::DeleteInstance => return self.delete_instance_confirm(),
            Message::LaunchScreenOpen {
                message,
                clear_selection,
            } => {
                if let State::AccountLogin { cancel_handle, .. } = &self.state {
                    cancel_handle.abort();
                }
                if clear_selection {
                    self.selected_instance = None;
                }
                return self.go_to_launch_screen(message);
            }
            Message::EditInstance(message) => match self.update_edit_instance(message) {
                Ok(n) => return n,
                Err(err) => self.set_error(err),
            },
            Message::InstallFabric(message) => return self.update_install_fabric(message),
            Message::CoreOpenDir(dir) => open_file_explorer(&dir),
            Message::CoreErrorCopy => {
                if let State::Error { error } = &self.state {
                    return iced::clipboard::write(format!("QuantumLauncher Error: {error}"));
                }
            }
            Message::CoreTick => {
                let mut commands = self.get_imgs_to_load();
                let command = self.tick();
                commands.push(command);
                return Task::batch(commands);
            }
            Message::UninstallLoaderForgeStart => {
                let instance = self.selected_instance.clone().unwrap();
                return Task::perform(
                    async move { loaders::forge::uninstall(instance).await.strerr() },
                    Message::UninstallLoaderEnd,
                );
            }
            Message::UninstallLoaderOptiFineStart => {
                let instance_name = self
                    .selected_instance
                    .as_ref()
                    .unwrap()
                    .get_name()
                    .to_owned();
                return Task::perform(
                    async { loaders::optifine::uninstall(instance_name).await.strerr() },
                    Message::UninstallLoaderEnd,
                );
            }
            Message::UninstallLoaderFabricStart => {
                let instance_name = self.selected_instance.clone().unwrap();
                return Task::perform(
                    async move { loaders::fabric::uninstall(instance_name).await.strerr() },
                    Message::UninstallLoaderEnd,
                );
            }
            Message::UninstallLoaderEnd(Ok(loader)) => {
                let message = format!(
                    "Uninstalled {}",
                    if let Loader::Fabric = loader {
                        "Fabric/Quilt".to_owned()
                    } else {
                        // TODO: If the debug printing gets a bit too debug-ey
                        // in the future, this may scare off users.
                        format!("{loader:?}")
                    }
                );
                return self.go_to_main_menu_with_message(Some(message));
            }
            Message::InstallForgeStart { is_neoforge } => {
                return self.install_forge(is_neoforge);
            }
            Message::InstallForgeEnd(Ok(())) => {
                return self.go_to_main_menu_with_message(Some("Installed Forge/NeoForge"));
            }
            Message::LaunchEndedLog(Ok((status, name))) => {
                info!("Game exited with status: {status}");
                self.set_game_crashed(status, &name);
            }
            Message::LaunchKill => return self.kill_selected_instance(),
            Message::LaunchCopyLog => {
                if let Some(log) = self
                    .client_logs
                    .get(self.selected_instance.as_ref().unwrap().get_name())
                {
                    return iced::clipboard::write(
                        log.log.iter().fold(String::new(), |n, v| n + v + "\n"),
                    );
                }
            }
            Message::UpdateCheckResult(Ok(info)) => match info {
                UpdateCheckInfo::UpToDate => {
                    info_no_log!("Launcher is latest version. No new updates");
                }
                UpdateCheckInfo::NewVersion { url } => {
                    self.state = State::UpdateFound(MenuLauncherUpdate {
                        url,
                        progress: None,
                    });
                }
            },
            Message::UpdateDownloadStart => return self.update_download_start(),
            Message::LauncherSettingsThemePicked(theme) => {
                info!("Setting color mode {theme}");
                self.config.theme = Some(theme.clone());

                match theme.as_str() {
                    "Light" => self.theme.lightness = LauncherThemeLightness::Light,
                    "Dark" => self.theme.lightness = LauncherThemeLightness::Dark,
                    _ => err!("Invalid color mode {theme}"),
                }
            }
            Message::LauncherSettingsOpen => self.state = State::LauncherSettings,
            Message::LauncherSettingsStylePicked(style) => {
                info!("Setting color scheme {style}");
                self.config.style = Some(style.clone());

                match style.as_str() {
                    "Purple" => self.theme.color = LauncherThemeColor::Purple,
                    "Brown" => self.theme.color = LauncherThemeColor::Brown,
                    "Sky Blue" => self.theme.color = LauncherThemeColor::SkyBlue,
                    _ => err!("Invalid color scheme {style}"),
                }
            }
            Message::InstallOptifine(msg) => return self.update_install_optifine(msg),
            Message::ServerManageOpen {
                selected_server,
                message,
            } => {
                self.selected_instance = selected_server.map(InstanceSelection::Server);
                return self.go_to_server_manage_menu(message);
            }
            Message::ServerCreateScreenOpen => {
                if let Some(cache) = &self.server_version_list_cache {
                    self.state = State::ServerCreate(MenuServerCreate::Loaded {
                        name: String::new(),
                        versions: iced::widget::combo_box::State::new(cache.clone()),
                        selected_version: None,
                    });
                } else {
                    let (sender, receiver) = std::sync::mpsc::channel();
                    self.state = State::ServerCreate(MenuServerCreate::LoadingList {
                        progress_receiver: receiver,
                        progress_number: 0.0,
                    });

                    let sender = Some(Arc::new(sender));
                    return Task::perform(
                        async move { ql_servers::list(sender).await.strerr() },
                        Message::ServerCreateVersionsLoaded,
                    );
                }
            }
            Message::ServerCreateNameInput(new_name) => {
                if let State::ServerCreate(MenuServerCreate::Loaded { name, .. }) = &mut self.state
                {
                    *name = new_name;
                }
            }
            Message::ServerCreateVersionSelected(list_entry) => {
                if let State::ServerCreate(MenuServerCreate::Loaded {
                    selected_version, ..
                }) = &mut self.state
                {
                    *selected_version = Some(list_entry);
                }
            }
            Message::ServerCreateStart => {
                if let State::ServerCreate(MenuServerCreate::Loaded {
                    name,
                    selected_version: Some(selected_version),
                    ..
                }) = &mut self.state
                {
                    let (sender, receiver) = std::sync::mpsc::channel();

                    let name = name.clone();
                    let selected_version = selected_version.clone();
                    self.state = State::ServerCreate(MenuServerCreate::Downloading {
                        progress: ProgressBar::with_recv(receiver),
                    });
                    return Task::perform(
                        async move {
                            ql_servers::create_server(name, selected_version, Some(sender))
                                .await
                                .strerr()
                        },
                        Message::ServerCreateEnd,
                    );
                }
            }
            Message::ServerCreateEnd(Ok(name)) => {
                self.selected_instance = Some(InstanceSelection::Server(name));
                return self.go_to_server_manage_menu(Some("Created Server".to_owned()));
            }
            Message::ServerCreateVersionsLoaded(Ok(vec)) => {
                self.server_version_list_cache = Some(vec.clone());
                self.state = State::ServerCreate(MenuServerCreate::Loaded {
                    versions: iced::widget::combo_box::State::new(vec),
                    selected_version: None,
                    name: String::new(),
                });
            }
            Message::ServerManageStartServer(server) => {
                self.server_logs.remove(&server);
                let (sender, receiver) = std::sync::mpsc::channel();
                self.java_recv = Some(ProgressBar::with_recv(receiver));

                if self.server_processes.contains_key(&server) {
                    err!("Server is already running");
                } else {
                    return Task::perform(
                        async move { ql_servers::run(server, sender).await.strerr() },
                        Message::ServerManageStartServerFinish,
                    );
                }
            }
            Message::ServerManageStartServerFinish(Ok((child, is_classic_server))) => {
                self.java_recv = None;
                return self.add_server_to_processes(child, is_classic_server);
            }
            Message::ServerManageEndedLog(Ok((status, name))) => {
                if status.success() {
                    info!("Server {name} stopped.");
                } else {
                    info!("Server {name} crashed with status: {status}");
                }

                // TODO: Implement server crash handling
                if let Some(log) = self.server_logs.get_mut(&name) {
                    log.has_crashed = !status.success();
                }
            }
            Message::ServerManageKillServer(server) => {
                if let Some(ServerProcess {
                    stdin: Some(stdin),
                    is_classic_server,
                    child,
                    has_issued_stop_command,
                    ..
                }) = self.server_processes.get_mut(&server)
                {
                    *has_issued_stop_command = true;
                    if *is_classic_server {
                        if let Err(err) = child.lock().unwrap().start_kill() {
                            err!("Could not kill classic server: {err}");
                        }
                    } else {
                        let future = stdin.write_all("stop\n".as_bytes());
                        tokio::runtime::Runtime::new()
                            .unwrap()
                            .block_on(future)
                            .unwrap();
                    };
                }
            }
            Message::ServerManageEditCommand(selected_server, command) => {
                if let Some(log) = self.server_logs.get_mut(&selected_server) {
                    log.command = command;
                }
            }
            Message::ServerManageSubmitCommand(selected_server) => {
                if let (
                    Some(log),
                    Some(ServerProcess {
                        stdin: Some(stdin), ..
                    }),
                ) = (
                    self.server_logs.get_mut(&selected_server),
                    self.server_processes.get_mut(&selected_server),
                ) {
                    let log_cloned = format!("{}\n", log.command);
                    let future = stdin.write_all(log_cloned.as_bytes());
                    // Make the input command visible in the log
                    log.log.push(format!("> {}", log.command));

                    log.command.clear();
                    tokio::runtime::Runtime::new()
                        .unwrap()
                        .block_on(future)
                        .unwrap();
                }
            }
            Message::ServerManageCopyLog => {
                let name = self.selected_instance.as_ref().unwrap().get_name();
                if let Some(logs) = self.server_logs.get(name) {
                    return iced::clipboard::write(
                        logs.log.iter().fold(String::new(), |n, v| n + v + "\n"),
                    );
                }
            }
            Message::InstallPaperStart => {
                self.state = State::InstallPaper;
                let instance_name = self
                    .selected_instance
                    .as_ref()
                    .unwrap()
                    .get_name()
                    .to_owned();
                return Task::perform(
                    async move { loaders::paper::install(instance_name).await.strerr() },
                    Message::InstallPaperEnd,
                );
            }
            Message::InstallPaperEnd(result) => {
                if let Err(err) = result {
                    self.set_error(err);
                } else {
                    return self.go_to_server_manage_menu(Some("Installed Paper".to_owned()));
                }
            }
            Message::UninstallLoaderPaperStart => {
                let get_name = self
                    .selected_instance
                    .as_ref()
                    .unwrap()
                    .get_name()
                    .to_owned();
                return Task::perform(
                    async move { loaders::paper::uninstall(get_name).await.strerr() },
                    Message::UninstallLoaderEnd,
                );
            }
            Message::CoreListLoaded(Ok((list, is_server))) => {
                if is_server {
                    self.server_list = Some(list);
                } else {
                    self.client_list = Some(list);
                }
            }
            Message::CoreCopyText(txt) => {
                return iced::clipboard::write(txt);
            }
            Message::InstallMods(msg) => return self.update_install_mods(msg),
            Message::CoreOpenChangeLog => {
                self.state = State::ChangeLog;
            }
            Message::EditPresets(msg) => match self.update_edit_presets(msg) {
                Ok(n) => return n,
                Err(err) => self.set_error(err),
            },
            Message::UninstallLoaderConfirm(msg, name) => {
                self.state = State::ConfirmAction {
                    msg1: format!("uninstall {name}?"),
                    msg2: "This should be fine, you can always reinstall it later".to_owned(),
                    yes: (*msg).clone(),
                    no: Message::ManageMods(ManageModsMessage::ScreenOpen),
                }
            }
            Message::CoreEvent(event, status) => return self.iced_event(event, status),
            Message::LaunchChangeTab(launch_tab_id) => {
                if let LaunchTabId::Edit = launch_tab_id {
                    if let Err(err) = self.edit_instance() {
                        err!("Could not open edit instance menu: {err}");
                        if let State::Launch(MenuLaunch { edit_instance, .. }) = &mut self.state {
                            *edit_instance = None;
                        }
                    }
                }
                if let State::Launch(MenuLaunch { tab, .. }) = &mut self.state {
                    *tab = launch_tab_id;
                }
            }
            Message::CoreLogToggle => {
                self.is_log_open = !self.is_log_open;
            }
            Message::CoreLogScroll(lines) => {
                let new_scroll = self.log_scroll - lines;
                if new_scroll >= 0 {
                    self.log_scroll = new_scroll;
                }
            }
            Message::CoreLogScrollAbsolute(lines) => {
                self.log_scroll = lines;
            }
            Message::LaunchLogScroll(lines) => {
                if let State::Launch(MenuLaunch { log_scroll, .. }) = &mut self.state {
                    let new_scroll = *log_scroll - lines;
                    if new_scroll >= 0 {
                        *log_scroll = new_scroll;
                    }
                }
            }
            Message::LaunchLogScrollAbsolute(lines) => {
                if let State::Launch(MenuLaunch { log_scroll, .. }) = &mut self.state {
                    *log_scroll = lines;
                }
            }
        }
        Task::none()
    }

    // Iced expects a `fn(&self)` so we're putting `&self`
    // even when not needed.
    #[allow(clippy::unused_self)]
    fn subscription(&self) -> iced::Subscription<Message> {
        const UPDATES_PER_SECOND: u64 = 5;

        let tick = iced::time::every(Duration::from_millis(1000 / UPDATES_PER_SECOND))
            .map(|_| Message::CoreTick);

        let events = iced::event::listen_with(|a, b, _| Some(Message::CoreEvent(a, b)));

        iced::Subscription::batch(vec![tick, events])
    }

    fn theme(&self) -> LauncherTheme {
        self.theme.clone()
    }

    fn scale_factor(&self) -> f64 {
        self.config.ui_scale.unwrap_or(1.0)
    }

    fn split_string_at_intervals(input: &str, interval: usize) -> Vec<String> {
        input
            .chars()
            .collect::<Vec<char>>()
            .chunks(interval)
            .map(|chunk| chunk.iter().collect())
            .collect()
    }
}

const DEBUG_LOG_BUTTON_HEIGHT: f32 = 16.0;

const WINDOW_HEIGHT: f32 = 400.0;
const WINDOW_WIDTH: f32 = 600.0;

fn main() {
    let command = arguments::command();
    let matches = command.clone().get_matches();

    if let Some(subcommand) = matches.subcommand() {
        match subcommand.0 {
            "list-instances" => {
                let command = get_list_instance_subcommand(subcommand);
                cmd_list_instances(&command, "instances");
                return;
            }
            "list-servers" => {
                let command = get_list_instance_subcommand(subcommand);
                cmd_list_instances(&command, "servers");
                return;
            }
            "list-available-versions" => {
                cmd_list_available_versions();
                return;
            }
            "--no-sandbox" => {
                err!("Unknown command --no-sandbox, ignoring...");
            }
            err => panic!("Unimplemented command! {err}"),
        }
    } else {
        arguments::print_intro();
    }

    info_no_log!("Starting up the launcher...");

    let icon =
        iced::window::icon::from_file_data(LAUNCHER_ICON, Some(image::ImageFormat::Ico)).ok();

    iced::application("QuantumLauncher", Launcher::update, Launcher::view)
        .subscription(Launcher::subscription)
        .scale_factor(Launcher::scale_factor)
        .theme(Launcher::theme)
        .settings(Settings {
            fonts: vec![
                include_bytes!("../../assets/fonts/Inter-Regular.ttf")
                    .as_slice()
                    .into(),
                include_bytes!("../../assets/fonts/launcher-icons.ttf")
                    .as_slice()
                    .into(),
                include_bytes!("../../assets/fonts/JetBrainsMono-Regular.ttf")
                    .as_slice()
                    .into(),
            ],
            default_font: iced::Font::with_name("Inter"),
            ..Default::default()
        })
        .window(iced::window::Settings {
            icon,
            exit_on_close_request: false,
            size: iced::Size {
                width: WINDOW_WIDTH,
                height: WINDOW_HEIGHT,
            },
            ..Default::default()
        })
        .run_with(Launcher::new)
        .unwrap();
    // Launcher::run(Settings {
    //     window: iced::window::Settings {
    //         size: iced::Size {
    //             width: WINDOW_WIDTH,
    //             height: WINDOW_HEIGHT,
    //         },
    //         resizable: true,
    //         ..Default::default()
    //     },
    //     fonts: vec![
    //         include_bytes!("../../assets/fonts/Inter-Regular.ttf")
    //             .as_slice()
    //             .into(),
    //         include_bytes!("../../assets/fonts/launcher-icons.ttf")
    //             .as_slice()
    //             .into(),
    //         include_bytes!("../../assets/fonts/JetBrainsMono-Regular.ttf")
    //             .as_slice()
    //             .into(),
    //     ],
    //     default_font: iced::Font::with_name("Inter"),
    //     ..Default::default()
    // })
    // .unwrap();
}

fn get_list_instance_subcommand(subcommand: (&str, &clap::ArgMatches)) -> Vec<PrintCmd> {
    if let Some((cmd, _)) = subcommand.1.subcommand() {
        let mut cmds = Vec::new();
        for cmd in cmd.split('-') {
            match cmd {
                "name" => cmds.push(PrintCmd::Name),
                "version" => cmds.push(PrintCmd::Version),
                "loader" => cmds.push(PrintCmd::Loader),
                invalid => {
                    panic!("Invalid subcommand {invalid}! Use any combination of name, version and loader separated by hyphen '-'");
                }
            }
        }
        cmds
    } else {
        vec![PrintCmd::Name]
    }
}
