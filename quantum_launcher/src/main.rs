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
//! - `ql_plugins` - A lua-based plugin system (incomplete)
//! - `ql_servers` - A self-hosted server management system (incomplete)
//! - `ql_core` - Core utilities and shared code
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
use iced::{widget, Settings, Task};
use launcher_state::{
    get_entries, LaunchTabId, Launcher, ManageModsMessage, MenuLaunch, MenuLauncherSettings,
    MenuLauncherUpdate, MenuServerCreate, Message, ProgressBar, SelectedState, ServerProcess,
    State, NEW_ACCOUNT_NAME, OFFLINE_ACCOUNT_NAME,
};

use menu_renderer::{
    button_with_icon,
    changelog::{changelog_0_4, welcome_msg},
    view_account_login, DISCORD,
};
use ql_core::{err, file_utils, info, open_file_explorer, InstanceSelection, SelectedMod};
use ql_instances::UpdateCheckInfo;
use ql_mod_manager::{loaders, mod_manager::Loader};
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

const LAUNCHER_ICON: &[u8] = include_bytes!("../../assets/icon/ql_logo.ico");

impl Launcher {
    fn new() -> (Self, iced::Task<Message>) {
        let check_for_updates_command = Task::perform(
            ql_instances::check_for_launcher_updates_w(),
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
            Message::AccountSelected(account) => {
                if account == NEW_ACCOUNT_NAME {
                    self.state = State::GenericMessage("Loading Login...".to_owned());
                    return Task::perform(
                        ql_instances::login_1_link_w(),
                        Message::AccountResponse1,
                    );
                } else {
                    self.accounts_selected = Some(account);
                }
            }
            Message::AccountResponse1(result) => match result {
                Ok(code) => {
                    let (task, handle) = Task::perform(
                        ql_instances::login_2_wait_w(code.clone()),
                        Message::AccountResponse2,
                    )
                    .abortable();
                    self.state = State::AccountLogin {
                        url: code.verification_uri,
                        code: code.user_code,
                        cancel_handle: handle,
                    };
                    return task;
                }
                Err(err) => self.set_error(err),
            },
            Message::AccountResponse2(result) => match result {
                Ok(token) => {
                    let (sender, receiver) = std::sync::mpsc::channel();
                    self.state = State::AccountLoginProgress(ProgressBar::with_recv(receiver));
                    return Task::perform(
                        ql_instances::login_3_xbox_w(token, Some(sender)),
                        Message::AccountResponse3,
                    );
                }
                Err(err) => self.set_error(err),
            },
            Message::AccountResponse3(result) => match result {
                Ok(data) => {
                    self.accounts_dropdown.push(data.username.clone());
                    self.accounts.insert(data.username.clone(), data);
                    return self.go_to_launch_screen(None);
                }
                Err(err) => {
                    self.set_error(err);
                }
            },
            Message::ManageMods(message) => return self.update_manage_mods(message),
            Message::LaunchInstanceSelected(selected_instance) => {
                self.selected_instance = Some(InstanceSelection::Instance(selected_instance));
                self.edit_instance_w();
            }
            Message::LaunchUsernameSet(username) => self.set_username(username),
            Message::LaunchStart => {
                let account_data = if let Some(account) = &self.accounts_selected {
                    if account == NEW_ACCOUNT_NAME || account == OFFLINE_ACCOUNT_NAME {
                        None
                    } else {
                        self.accounts.get(account).cloned()
                    }
                } else {
                    None
                };
                return self.launch_game(account_data);
            }
            Message::LaunchEnd(result) => {
                return self.finish_launching(result);
            }
            Message::CreateInstance(message) => return self.update_create_instance(message),
            Message::DeleteInstanceMenu => {
                self.state = State::ConfirmAction {
                    msg1: format!(
                        "delete the instance {}",
                        self.selected_instance.as_ref().unwrap().get_name()
                    ),
                    msg2: "All your data, including worlds, will be lost".to_owned(),
                    yes: Message::DeleteInstance,
                    no: Message::LaunchScreenOpen {
                        message: None,
                        clear_selection: false,
                    },
                };
            }
            Message::DeleteInstance => return self.delete_selected_instance(),
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
            Message::EditInstance(message) => return self.update_edit_instance(message),
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
                return Task::perform(
                    loaders::forge::uninstall_w(self.selected_instance.clone().unwrap()),
                    Message::UninstallLoaderEnd,
                )
            }
            Message::UninstallLoaderOptiFineStart => {
                return Task::perform(
                    loaders::optifine::uninstall_w(
                        self.selected_instance
                            .as_ref()
                            .unwrap()
                            .get_name()
                            .to_owned(),
                    ),
                    Message::UninstallLoaderEnd,
                );
            }
            Message::UninstallLoaderFabricStart => {
                return Task::perform(
                    loaders::fabric::uninstall_w(self.selected_instance.clone().unwrap()),
                    Message::UninstallLoaderEnd,
                )
            }
            Message::UninstallLoaderEnd(result) => match result {
                Ok(loader) => {
                    let message = format!(
                        "Uninstalled {}",
                        if let Loader::Fabric = loader {
                            "Fabric/Quilt".to_owned()
                        } else {
                            loader.to_string()
                        }
                    );
                    return self.go_to_main_menu_with_message(message);
                }
                Err(err) => self.set_error(err),
            },
            Message::InstallForgeStart => {
                return self.install_forge();
            }
            Message::InstallForgeEnd(result) => match result {
                Ok(()) => {
                    return self.go_to_main_menu_with_message("Installed Forge");
                }
                Err(err) => self.set_error(err),
            },
            Message::LaunchEndedLog(result) => match result {
                Ok((status, name)) => {
                    info!("Game exited with status: {status}");
                    self.set_game_crashed(status, &name);
                }
                Err(err) => self.set_error(err),
            },
            Message::LaunchKill => {
                if let Some(process) = self
                    .client_processes
                    .remove(self.selected_instance.as_ref().unwrap().get_name())
                {
                    return Task::perform(
                        {
                            async move {
                                let mut child = process.child.lock().unwrap();
                                child.start_kill().map_err(|err| err.to_string())
                            }
                        },
                        Message::LaunchKillEnd,
                    );
                }
            }
            Message::CoreTickConfigSaved(result) | Message::LaunchKillEnd(result) => {
                if let Err(err) = result {
                    self.set_error(err);
                }
            }
            Message::LaunchCopyLog => {
                if let Some(log) = self
                    .client_logs
                    .get(self.selected_instance.as_ref().unwrap().get_name())
                {
                    return iced::clipboard::write(log.log.clone());
                }
            }
            Message::UpdateCheckResult(update_check_info) => match update_check_info {
                Ok(info) => match info {
                    UpdateCheckInfo::UpToDate => {
                        info!("Launcher is latest version. No new updates");
                    }
                    UpdateCheckInfo::NewVersion { url } => {
                        self.state = State::UpdateFound(MenuLauncherUpdate {
                            url,
                            progress: None,
                        });
                    }
                },
                Err(err) => {
                    err!("Could not check for updates: {err}");
                }
            },
            Message::UpdateDownloadStart => {
                if let State::UpdateFound(MenuLauncherUpdate { url, progress, .. }) =
                    &mut self.state
                {
                    let (sender, update_receiver) = std::sync::mpsc::channel();
                    *progress = Some(ProgressBar::with_recv_and_msg(
                        update_receiver,
                        "Starting Update".to_owned(),
                    ));

                    return Task::perform(
                        ql_instances::install_launcher_update_w(url.clone(), sender),
                        Message::UpdateDownloadEnd,
                    );
                }
            }
            Message::UpdateDownloadEnd(err) => {
                if let Err(err) = err {
                    self.set_error(err);
                } else {
                    // WTF: Yeah... no one is gonna see this.
                    return self.go_to_launch_screen(Some(
                        "Updated launcher! Close and reopen the launcher to see the new update"
                            .to_owned(),
                    ));
                }
            }
            Message::LauncherSettingsThemePicked(theme) => {
                info!("Setting color mode {theme}");
                if let Some(config) = self.config.as_mut() {
                    config.theme = Some(theme.clone());
                }
                match theme.as_str() {
                    "Light" => self.theme.lightness = LauncherThemeLightness::Light,
                    "Dark" => self.theme.lightness = LauncherThemeLightness::Dark,
                    _ => err!("Invalid color mode {theme}"),
                }
            }
            Message::LauncherSettingsOpen => {
                self.state = State::LauncherSettings;
            }
            Message::LauncherSettingsStylePicked(style) => {
                info!("Setting color scheme {style}");
                if let Some(config) = self.config.as_mut() {
                    config.style = Some(style.clone());
                }
                match style.as_str() {
                    "Purple" => self.theme.color = LauncherThemeColor::Purple,
                    "Brown" => self.theme.color = LauncherThemeColor::Brown,
                    "Sky Blue" => self.theme.color = LauncherThemeColor::SkyBlue,
                    _ => err!("Invalid color scheme {style}"),
                }
            }
            Message::ManageModsSelectAll => {
                if let State::EditMods(menu) = &mut self.state {
                    match menu.selected_state {
                        SelectedState::All => {
                            menu.selected_mods.clear();
                            menu.selected_state = SelectedState::None;
                        }
                        SelectedState::Some | SelectedState::None => {
                            menu.selected_mods = menu
                                .mods
                                .mods
                                .iter()
                                .filter_map(|(id, mod_info)| {
                                    mod_info
                                        .manually_installed
                                        .then_some(SelectedMod::Downloaded {
                                            name: mod_info.name.clone(),
                                            id: id.clone(),
                                        })
                                })
                                .chain(menu.locally_installed_mods.iter().map(|n| {
                                    SelectedMod::Local {
                                        file_name: n.clone(),
                                    }
                                }))
                                .collect();
                            menu.selected_state = SelectedState::All;
                        }
                    }
                }
            }
            Message::InstallOptifine(msg) => return self.update_install_optifine(msg),
            Message::ServerManageSelectedServer(selected) => {
                self.selected_instance = Some(InstanceSelection::Server(selected));
            }
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

                    return Task::perform(
                        ql_servers::list_versions(Some(Arc::new(sender))),
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
                        ql_servers::create_server_w(name, selected_version, Some(sender)),
                        Message::ServerCreateEnd,
                    );
                }
            }
            Message::ServerCreateEnd(result) => match result {
                Ok(name) => {
                    self.selected_instance = Some(InstanceSelection::Server(name));
                    return self.go_to_server_manage_menu(Some("Created Server".to_owned()));
                }
                Err(err) => self.set_error(err),
            },
            Message::ServerCreateVersionsLoaded(vec) => match vec {
                Ok(vec) => {
                    self.server_version_list_cache = Some(vec.clone());
                    self.state = State::ServerCreate(MenuServerCreate::Loaded {
                        versions: iced::widget::combo_box::State::new(vec),
                        selected_version: None,
                        name: String::new(),
                    });
                }
                Err(err) => self.set_error(err),
            },
            Message::ServerDeleteOpen => {
                let selected_server = self.selected_instance.as_ref().unwrap().get_name();
                self.state = State::ConfirmAction {
                    msg1: format!("delete the server {selected_server}"),
                    msg2: "All your data will be lost".to_owned(),
                    yes: Message::ServerDeleteConfirm,
                    no: Message::ServerManageOpen {
                        selected_server: Some(selected_server.to_owned()),
                        message: None,
                    },
                };
            }
            Message::ServerDeleteConfirm => {
                if let Some(InstanceSelection::Server(selected_server)) = &self.selected_instance {
                    match ql_servers::delete_server(selected_server) {
                        Ok(()) => {
                            self.selected_instance = None;
                            return self
                                .go_to_server_manage_menu(Some("Deleted Server".to_owned()));
                        }
                        Err(err) => self.set_error(err),
                    }
                }
            }
            Message::ServerManageStartServer(server) => {
                self.server_logs.remove(&server);
                let (sender, receiver) = std::sync::mpsc::channel();
                if let State::ServerManage(_) = &mut self.state {
                    self.java_recv = Some(ProgressBar::with_recv(receiver));
                }

                if self.server_processes.contains_key(&server) {
                    err!("Server is already running");
                } else {
                    return Task::perform(
                        ql_servers::run_w(server, sender),
                        Message::ServerManageStartServerFinish,
                    );
                }
            }
            Message::ServerManageStartServerFinish(result) => match result {
                Ok((child, is_classic_server)) => {
                    return self.add_server_to_processes(child, is_classic_server);
                }
                Err(err) => self.set_error(err),
            },
            Message::ServerManageEndedLog(result) => match result {
                Ok((status, name)) => {
                    info!("Server exited with status: {status}");
                    // TODO: Implement server crash handling
                    if let Some(log) = self.server_logs.get_mut(&name) {
                        log.has_crashed = !status.success();
                    }
                }
                Err(err) => self.set_error(err),
            },
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
                    let var_name = &format!("{}\n", log.command);
                    let future = stdin.write_all(var_name.as_bytes());
                    log.command.clear();

                    tokio::runtime::Runtime::new()
                        .unwrap()
                        .block_on(future)
                        .unwrap();
                }
            }
            Message::ServerEditModsOpen => match self.go_to_edit_mods_menu() {
                Ok(n) => return n,
                Err(err) => self.set_error(err),
            },
            Message::ServerManageCopyLog => {
                let name = self.selected_instance.as_ref().unwrap().get_name();
                if let Some(logs) = self.server_logs.get(name) {
                    return iced::clipboard::write(logs.log.clone());
                }
            }
            Message::InstallPaperStart => {
                self.state = State::InstallPaper;
                return Task::perform(
                    loaders::paper::install_w(
                        self.selected_instance
                            .as_ref()
                            .unwrap()
                            .get_name()
                            .to_owned(),
                    ),
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
                return Task::perform(
                    loaders::paper::uninstall_w(
                        self.selected_instance
                            .as_ref()
                            .unwrap()
                            .get_name()
                            .to_owned(),
                    ),
                    Message::UninstallLoaderEnd,
                )
            }
            Message::CoreListLoaded(result) => match result {
                Ok((list, is_server)) => {
                    if is_server {
                        self.server_list = Some(list);
                    } else {
                        self.client_list = Some(list);
                    }
                }
                Err(err) => self.set_error(err),
            },
            Message::CoreCopyText(txt) => {
                return iced::clipboard::write(txt);
            }
            Message::InstallMods(msg) => return self.update_install_mods(msg),
            Message::CoreOpenChangeLog => {
                self.state = State::ChangeLog;
            }
            Message::EditPresets(msg) => return self.update_edit_presets(msg),
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
                if let (LaunchTabId::Edit, Some(selected_instance)) =
                    (launch_tab_id, self.selected_instance.clone())
                {
                    if let Err(err) = self.edit_instance(&selected_instance) {
                        self.set_error(err);
                    }
                }
                if let State::Launch(MenuLaunch { tab, .. }) = &mut self.state {
                    *tab = launch_tab_id;
                }
            }
        }
        Task::none()
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        const UPDATES_PER_SECOND: u64 = 12;

        let tick = iced::time::every(Duration::from_millis(1000 / UPDATES_PER_SECOND))
            .map(|_| Message::CoreTick);

        let events = iced::event::listen_with(|a, b, _| Some(Message::CoreEvent(a, b)));

        iced::Subscription::batch(vec![tick, events])
    }

    fn view(&self) -> iced::Element<'_, Message, LauncherTheme, iced::Renderer> {
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
            State::AccountLogin { url, code, .. } => view_account_login(url, code),
            State::EditMods(menu) => menu.view(self.selected_instance.as_ref().unwrap(), &self.dir),
            State::Create(menu) => menu.view(),
            State::ConfirmAction {
                msg1,
                msg2,
                yes,
                no,
            } => widget::column![
                widget::text!("Are you SURE you want to {msg1}?"),
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
                    widget::button("Back").on_press(Message::LaunchScreenOpen {
                        message: None,
                        clear_selection: true
                    }),
                    widget::button("Copy Error").on_press(Message::CoreErrorCopy),
                    widget::button("Join Discord for help")
                        .on_press(Message::CoreOpenDir(DISCORD.to_owned()))
                )
                .padding(10)
                .spacing(10),
            )
            .into(),
            State::InstallFabric(menu) => menu.view(self.selected_instance.as_ref().unwrap()),
            State::InstallForge(menu) => menu.view(),
            State::UpdateFound(menu) => menu.view(),
            State::InstallJava => widget::column!(widget::text("Downloading Java").size(20),)
                .push_maybe(self.java_recv.as_ref().map(|n| n.view()))
                .padding(10)
                .spacing(10)
                .into(),
            State::ModsDownload(menu) => menu.view(&self.images),
            State::LauncherSettings => MenuLauncherSettings::view(self.config.as_ref()),
            State::RedownloadAssets { progress, .. } => widget::column!(
                widget::text("Redownloading Assets").size(20),
                progress.view()
            )
            .padding(10)
            .spacing(10)
            .into(),
            State::InstallOptifine(menu) => menu.view(),
            State::ServerManage(menu) => menu.view(
                self.server_list.as_ref(),
                self.selected_instance.as_ref(),
                &self.server_logs,
                &self.server_processes,
                &self.dir,
            ),
            State::ServerCreate(menu) => menu.view(),
            State::InstallPaper => widget::column!(widget::text("Installing Paper...").size(20))
                .padding(10)
                .spacing(10)
                .into(),
            State::ManagePresets(menu) => menu.view(),
            State::ChangeLog => widget::scrollable(
                widget::column!(
                    button_with_icon(icon_manager::back(), "Back", 16).on_press(
                        Message::LaunchScreenOpen {
                            message: None,
                            clear_selection: true
                        }
                    ),
                    changelog_0_4() // changelog_0_3_1()
                )
                .padding(10)
                .spacing(10),
            )
            .into(),
            State::Welcome => welcome_msg(),
        }
    }

    fn theme(&self) -> LauncherTheme {
        self.theme.clone()
    }

    fn scale_factor(&self) -> f64 {
        1.0
    }
}

const WINDOW_HEIGHT: f32 = 400.0;
const WINDOW_WIDTH: f32 = 600.0;

fn main() {
    /*let (sender, recv) = std::sync::mpsc::channel();
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(ql_plugins::install_plugins())
        .unwrap();
    let handle = std::thread::spawn(|| {
        let plugin = ql_plugins::Plugin::new("OptiFine Installer", Some("1.0")).unwrap();
        plugin.set_generic_progress(sender, "javaprog").unwrap();
        plugin
            .set_selected_instance(
                InstanceSelection::Instance("1.21.4 quilt".to_owned()),
                "optifine_instance",
            )
            .unwrap();
        plugin
            .set_bytes(
                include_bytes!("../../preview_OptiFine_1.21.4_HD_U_J3_pre15.jar"),
                "optifine_installer_bytes",
            )
            .unwrap();
        plugin.init().unwrap();
    });

    while let Ok(msg) = recv.recv() {
        println!("msg: {msg:?}")
    }
    handle.join().unwrap();
    return;*/

    let command = arguments::command();
    let matches = command.clone().get_matches();
    if let Some(subcommand) = matches.subcommand() {
        match subcommand.0 {
            "list-instances" => {
                let command = get_list_instance_subcommand(subcommand);
                cmd_list_instances(command, "instances");
                return;
            }
            "list-servers" => {
                let command = get_list_instance_subcommand(subcommand);
                cmd_list_instances(command, "servers");
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

    info!("Starting up the launcher...");

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
                    err!("Invalid subcommand {invalid}! Use any combination of name, version and loader separated by hyphen '-'");
                    std::process::exit(1);
                }
            }
        }
        cmds
    } else {
        vec![PrintCmd::Name]
    }
}
