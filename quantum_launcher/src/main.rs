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
//! # Crate Structure
//! - `quantum_launcher` - The GUI frontend
//! - `ql_instances` - Instance management, updating and launching
//! - `ql_mod_manager` - Mod management and installation
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
//!
//! This is done to make use with `iced::Command` easier.

use std::{sync::Arc, time::Duration};

use colored::Colorize;
use iced::{
    executor,
    widget::{self, image::Handle},
    Application, Command, Settings,
};
use launcher_state::{
    get_entries, Launcher, ManageModsMessage, MenuEditMods, MenuInstallForge, MenuInstallOptifine,
    MenuLaunch, MenuLauncherSettings, MenuLauncherUpdate, MenuServerCreate, MenuServerManage,
    Message, OptifineInstallProgressData, SelectedMod, SelectedState, ServerProcess, State,
    UpdateModsProgress,
};

use menu_renderer::{button_with_icon, menu_delete_instance_view};
use message_handler::open_file_explorer;
use ql_core::{
    err, file_utils, info,
    json::{instance_config::InstanceConfigJson, version::VersionDetails},
    InstanceSelection,
};
use ql_instances::{UpdateCheckInfo, LAUNCHER_VERSION_NAME};
use ql_mod_manager::{
    instance_mod_installer,
    mod_manager::{Loader, ModIndex, ProjectInfo},
};
use stylesheet::styles::{LauncherStyle, LauncherTheme};
use tokio::io::AsyncWriteExt;

mod config;
mod icon_manager;
mod launcher_state;
mod menu_renderer;
mod message_handler;
mod message_update;
mod mods_store;
mod stylesheet;
mod tick;

const LAUNCHER_ICON: &[u8] = include_bytes!("../../assets/icon/ql_logo.ico");

impl Application for Launcher {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = LauncherTheme;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, iced::Command<Self::Message>) {
        let load_icon_command = load_window_icon();
        let check_for_updates_command = Command::perform(
            ql_instances::check_for_launcher_updates_w(),
            Message::UpdateCheckResult,
        );

        let mut command = Command::batch(vec![load_icon_command, check_for_updates_command]);

        let launcher = match Launcher::load_new(None) {
            Ok((launcher, new_command)) => {
                command = Command::batch(vec![command, new_command]);
                launcher
            }
            Err(error) => Launcher::with_error(&error.to_string()),
        };
        (launcher, command)
    }

    fn title(&self) -> String {
        "Quantum Launcher".to_owned()
    }

    fn update(&mut self, message: Self::Message) -> iced::Command<Self::Message> {
        match message {
            Message::ManageMods(message) => return self.update_manage_mods(message),
            Message::LaunchInstanceSelected(selected_instance) => {
                self.selected_instance = Some(InstanceSelection::Instance(selected_instance));
            }
            Message::LaunchUsernameSet(username) => self.set_username(username),
            Message::LaunchStart => return self.launch_game(),
            Message::LaunchEnd(result) => {
                return self.finish_launching(result);
            }
            Message::CreateInstance(message) => return self.update_create_instance(message),
            Message::DeleteInstanceMenu => {
                self.state = State::DeleteInstance;
            }
            Message::DeleteInstance => return self.delete_selected_instance(),
            Message::LaunchScreenOpen {
                message,
                clear_selection,
            } => {
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
            Message::CoreTick => return self.tick(),
            Message::UninstallLoaderForgeStart => {
                return Command::perform(
                    instance_mod_installer::forge::uninstall_w(
                        self.selected_instance.clone().unwrap(),
                    ),
                    Message::UninstallLoaderEnd,
                )
            }
            Message::UninstallLoaderOptiFineStart => {
                return Command::perform(
                    instance_mod_installer::optifine::uninstall_w(
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
                return Command::perform(
                    instance_mod_installer::fabric::uninstall_w(
                        self.selected_instance.clone().unwrap(),
                    ),
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
                    return self.go_to_main_menu(Some(message));
                }
                Err(err) => self.set_error(err),
            },
            Message::InstallForgeStart => {
                return self.install_forge();
            }
            Message::InstallForgeEnd(result) => match result {
                Ok(()) => {
                    let message = "Installed Forge".to_owned();
                    return self.go_to_main_menu(Some(message));
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
                    return Command::perform(
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
            Message::InstallModsDownloadComplete(result) => match result {
                Ok(id) => {
                    if let State::ModsDownload(menu) = &mut self.state {
                        menu.mods_download_in_progress.remove(&id);
                    }
                }
                Err(err) => self.set_error(err),
            },
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
                            receiver: None,
                            progress: 0.0,
                            progress_message: None,
                        });
                    }
                },
                Err(err) => {
                    err!("Could not check for updates: {err}");
                }
            },
            Message::UpdateDownloadStart => {
                if let State::UpdateFound(MenuLauncherUpdate {
                    url,
                    receiver,
                    progress_message,
                    ..
                }) = &mut self.state
                {
                    let (sender, update_receiver) = std::sync::mpsc::channel();
                    *receiver = Some(update_receiver);
                    *progress_message = Some("Starting Update".to_owned());

                    return Command::perform(
                        ql_instances::install_launcher_update_w(url.clone(), sender),
                        Message::UpdateDownloadEnd,
                    );
                }
            }
            Message::UpdateDownloadEnd(err) => {
                if let Err(err) = err {
                    self.set_error(err);
                } else {
                    return self.go_to_launch_screen(Some(
                        "Updated launcher! Close and reopen the launcher to see the new update"
                            .to_owned(),
                    ));
                }
            }
            Message::InstallModsSearchResult(search) => {
                if let State::ModsDownload(menu) = &mut self.state {
                    menu.is_loading_search = false;
                    match search {
                        Ok((search, time)) => {
                            if time > menu.latest_load {
                                menu.results = Some(search);
                                menu.latest_load = time;
                            }
                        }
                        Err(err) => self.set_error(err),
                    }
                }
            }
            Message::InstallModsOpen => match self.open_mods_screen() {
                Ok(command) => return command,
                Err(err) => self.set_error(err),
            },
            Message::InstallModsSearchInput(input) => {
                if let State::ModsDownload(menu) = &mut self.state {
                    menu.query = input;

                    return menu.search_modrinth(matches!(
                        &self.selected_instance,
                        Some(InstanceSelection::Server(_))
                    ));
                }
            }
            Message::InstallModsClick(i) => {
                if let State::ModsDownload(menu) = &mut self.state {
                    menu.opened_mod = Some(i);
                    if let Some(results) = &menu.results {
                        let hit = results.hits.get(i).unwrap();
                        if !menu.result_data.contains_key(&hit.project_id) {
                            let task = ProjectInfo::download_w(hit.project_id.clone());
                            return Command::perform(task, Message::InstallModsLoadData);
                        }
                    }
                }
            }
            Message::InstallModsBackToMainScreen => {
                if let State::ModsDownload(menu) = &mut self.state {
                    menu.opened_mod = None;
                }
            }
            Message::InstallModsLoadData(project_info) => match project_info {
                Ok(info) => {
                    if let State::ModsDownload(menu) = &mut self.state {
                        let id = info.id.clone();
                        menu.result_data.insert(id, *info);
                    }
                }
                Err(err) => self.set_error(err),
            },
            Message::InstallModsImageDownloaded(image) => match image {
                Ok(image) => {
                    if image.is_svg {
                        let handle = iced::widget::svg::Handle::from_memory(image.image);
                        self.images_svg.insert(image.url, handle);
                    } else {
                        self.images_bitmap
                            .insert(image.url, Handle::from_memory(image.image));
                    }
                }
                Err(err) => {
                    err!("Could not download image: {err}");
                }
            },
            Message::InstallModsDownload(index) => {
                if let Some(value) = self.mod_download(index) {
                    return value;
                }
            }
            Message::LauncherSettingsThemePicked(theme) => {
                info!("Setting theme {theme}");
                if let Some(config) = self.config.as_mut() {
                    config.theme = Some(theme.clone());
                }
                match theme.as_str() {
                    "Light" => self.theme = LauncherTheme::Light,
                    "Dark" => self.theme = LauncherTheme::Dark,
                    _ => err!("Invalid theme {theme}"),
                }
            }
            Message::LauncherSettingsOpen => {
                self.state = State::LauncherSettings;
            }
            Message::LauncherSettingsStylePicked(style) => {
                info!("Setting style {style}");
                if let Some(config) = self.config.as_mut() {
                    config.style = Some(style.clone());
                }
                match style.as_str() {
                    "Purple" => *self.style.lock().unwrap() = LauncherStyle::Purple,
                    "Brown" => *self.style.lock().unwrap() = LauncherStyle::Brown,
                    _ => err!("Invalid theme {style}"),
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
            Message::InstallOptifineScreenOpen => {
                self.state = State::InstallOptifine(MenuInstallOptifine { progress: None });
            }
            Message::InstallOptifineSelectInstallerStart => {
                return Command::perform(
                    rfd::AsyncFileDialog::new()
                        .add_filter("jar", &["jar"])
                        .set_title("Select OptiFine Installer")
                        .pick_file(),
                    Message::InstallOptifineSelectInstallerEnd,
                )
            }
            Message::InstallOptifineSelectInstallerEnd(handle) => {
                if let Some(handle) = handle {
                    let path = handle.path().to_owned();

                    let (p_sender, p_recv) = std::sync::mpsc::channel();
                    let (j_sender, j_recv) = std::sync::mpsc::channel();

                    self.state = State::InstallOptifine(MenuInstallOptifine {
                        progress: Some(OptifineInstallProgressData {
                            optifine_install_progress: p_recv,
                            optifine_install_num: 0.0,
                            java_install_progress: j_recv,
                            java_install_num: 0.0,
                            is_java_being_installed: false,
                            optifine_install_message: String::new(),
                            java_install_message: String::new(),
                        }),
                    });

                    return Command::perform(
                        // Note: OptiFine does not support servers
                        // so it's safe to assume we've selected an instance.
                        ql_mod_manager::instance_mod_installer::optifine::install_optifine_w(
                            self.selected_instance
                                .as_ref()
                                .unwrap()
                                .get_name()
                                .to_owned(),
                            path,
                            Some(p_sender),
                            Some(j_sender),
                        ),
                        Message::InstallOptifineEnd,
                    );
                }
            }
            Message::InstallOptifineEnd(result) => {
                if let Err(err) = result {
                    self.set_error(err);
                } else {
                    return self.go_to_launch_screen(Some("Installed OptiFine".to_owned()));
                }
            }
            Message::ManageModsUpdateCheckResult(updates) => {
                if let (Some(updates), State::EditMods(menu)) = (updates, &mut self.state) {
                    menu.available_updates =
                        updates.into_iter().map(|(a, b)| (a, b, true)).collect();
                }
            }
            Message::ManageModsUpdateCheckToggle(idx, t) => {
                if let State::EditMods(MenuEditMods {
                    available_updates, ..
                }) = &mut self.state
                {
                    if let Some((_, _, b)) = available_updates.get_mut(idx) {
                        *b = t;
                    }
                }
            }
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
                        progress_receiver: None,
                        progress_number: 0.0,
                    });
                } else {
                    let (sender, receiver) = std::sync::mpsc::channel();
                    self.state = State::ServerCreate(MenuServerCreate::Loading {
                        progress_receiver: receiver,
                        progress_number: 0.0,
                    });

                    return Command::perform(
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
                    progress_receiver,
                    ..
                }) = &mut self.state
                {
                    let (sender, receiver) = std::sync::mpsc::channel();
                    *progress_receiver = Some(receiver);
                    return Command::perform(
                        ql_servers::create_server_w(
                            name.clone(),
                            selected_version.clone(),
                            Some(sender),
                        ),
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
                        progress_receiver: None,
                        progress_number: 0.0,
                    });
                }
                Err(err) => self.set_error(err),
            },
            Message::ServerDeleteOpen => {
                self.state = State::ServerDelete;
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
                if let State::ServerManage(menu) = &mut self.state {
                    menu.java_install_recv = Some(receiver);
                }

                if self.server_processes.contains_key(&server) {
                    err!("Server is already running");
                } else {
                    return Command::perform(
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
                return Command::perform(
                    instance_mod_installer::paper::install_w(
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
                return Command::perform(
                    instance_mod_installer::paper::uninstall_w(
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
        }
        Command::none()
    }

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        const UPDATES_PER_SECOND: u64 = 12;

        iced::time::every(Duration::from_millis(1000 / UPDATES_PER_SECOND))
            .map(|_| Message::CoreTick)
    }

    fn view(&self) -> iced::Element<'_, Self::Message, Self::Theme, iced::Renderer> {
        match &self.state {
            State::Launch(menu) => menu.view(
                self.config.as_ref(),
                self.client_list.as_deref(),
                &self.client_processes,
                &self.client_logs,
                self.selected_instance.as_ref(),
            ),
            State::EditInstance(menu) => menu.view(self.selected_instance.as_ref().unwrap()),
            State::EditMods(menu) => menu.view(self.selected_instance.as_ref().unwrap()),
            State::Create(menu) => menu.view(),
            State::DeleteInstance => {
                menu_delete_instance_view(self.selected_instance.as_ref().unwrap())
            }
            State::Error { error } => widget::scrollable(
                widget::column!(
                    widget::text(format!("Error: {error}")),
                    widget::button("Back").on_press(Message::LaunchScreenOpen {
                        message: None,
                        clear_selection: true
                    }),
                    widget::button("Copy Error").on_press(Message::CoreErrorCopy),
                )
                .padding(10)
                .spacing(10),
            )
            .into(),
            State::InstallFabric(menu) => menu.view(self.selected_instance.as_ref().unwrap()),
            State::InstallForge(menu) => menu.view(),
            State::UpdateFound(menu) => menu.view(),
            State::InstallJava(menu) => menu.view(),
            State::ModsDownload(menu) => {
                menu.view(&self.images_bitmap, &self.images_svg, &self.images_to_load)
            }
            State::LauncherSettings => MenuLauncherSettings::view(self.config.as_ref()),
            State::RedownloadAssets(menu) => widget::column!(
                widget::text("Redownloading Assets").size(20),
                widget::progress_bar(0.0..=1.0, menu.num),
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
            ),
            State::ServerCreate(menu) => menu.view(),
            State::ServerDelete => {
                let selected_server = self
                    .selected_instance
                    .as_ref()
                    .unwrap()
                    .get_name()
                    .to_owned();
                widget::column!(
                    widget::text(format!("Delete server: {selected_server}?")).size(20),
                    "You will lose ALL of your data!",
                    button_with_icon(icon_manager::tick(), "Confirm")
                        .on_press(Message::ServerDeleteConfirm),
                    button_with_icon(icon_manager::back(), "Back").on_press(
                        Message::ServerManageOpen {
                            selected_server: Some(selected_server),
                            message: None
                        }
                    ),
                )
                .padding(10)
                .spacing(10)
                .into()
            }
            State::InstallPaper => widget::column!(widget::text("Installing Paper...").size(20))
                .padding(10)
                .spacing(10)
                .into(),
        }
    }

    fn theme(&self) -> Self::Theme {
        self.theme.clone()
    }

    fn scale_factor(&self) -> f64 {
        1.0
    }
}

fn load_window_icon() -> Command<Message> {
    let icon = iced::window::icon::from_file_data(LAUNCHER_ICON, Some(image::ImageFormat::Ico));
    match icon {
        Ok(icon) => iced::window::change_icon(iced::window::Id::MAIN, icon),
        Err(err) => {
            err!("Could not load icon: {err}");
            Command::none()
        }
    }
}

impl Launcher {
    fn mod_download(&mut self, index: usize) -> Option<Command<Message>> {
        let selected_instance = self.selected_instance.clone()?;
        let State::ModsDownload(menu) = &mut self.state else {
            return None;
        };
        let Some(results) = &menu.results else {
            err!("Couldn't download mod: Search results empty");
            return None;
        };
        let Some(hit) = results.hits.get(index) else {
            err!("Couldn't download mod: Not present in results");
            return None;
        };

        menu.mods_download_in_progress
            .insert(hit.project_id.clone());
        Some(Command::perform(
            ql_mod_manager::mod_manager::download_mod_w(hit.project_id.clone(), selected_instance),
            Message::InstallModsDownloadComplete,
        ))
    }

    fn set_game_crashed(&mut self, status: std::process::ExitStatus, name: &str) {
        if let State::Launch(MenuLaunch { message, .. }) = &mut self.state {
            let has_crashed = !status.success();
            if has_crashed {
                *message =
                    format!("Game Crashed with code: {status}\nCheck Logs for more information");
            }
            if let Some(log) = self.client_logs.get_mut(name) {
                log.has_crashed = has_crashed;
            }
        }
    }

    fn update_mod_index(&mut self) {
        if let State::EditMods(menu) = &mut self.state {
            match ModIndex::get(self.selected_instance.as_ref().unwrap())
                .map_err(|err| err.to_string())
            {
                Ok(idx) => menu.mods = idx,
                Err(err) => self.set_error(err),
            }
        }
    }

    fn update_mods(&mut self) -> Command<Message> {
        if let State::EditMods(menu) = &mut self.state {
            let updates = menu
                .available_updates
                .clone()
                .into_iter()
                .map(|(n, _, _)| n)
                .collect();
            let (sender, receiver) = std::sync::mpsc::channel();
            menu.mod_update_progress = Some(UpdateModsProgress {
                recv: receiver,
                num: 0.0,
                message: "Starting...".to_owned(),
            });
            Command::perform(
                ql_mod_manager::mod_manager::apply_updates_w(
                    self.selected_instance.clone().unwrap(),
                    updates,
                    Some(sender),
                ),
                |n| Message::ManageMods(ManageModsMessage::UpdateModsFinished(n)),
            )
        } else {
            Command::none()
        }
    }

    fn go_to_server_manage_menu(&mut self, message: Option<String>) -> Command<Message> {
        self.state = State::ServerManage(MenuServerManage {
            java_install_recv: None,
            message,
        });
        Command::perform(
            get_entries("servers".to_owned(), true),
            Message::CoreListLoaded,
        )
    }

    fn install_forge(&mut self) -> Command<Message> {
        let (f_sender, f_receiver) = std::sync::mpsc::channel();
        let (j_sender, j_receiver) = std::sync::mpsc::channel();

        let command = Command::perform(
            instance_mod_installer::forge::install_w(
                self.selected_instance.clone().unwrap(),
                Some(f_sender),
                Some(j_sender),
            ),
            Message::InstallForgeEnd,
        );

        self.state = State::InstallForge(MenuInstallForge {
            forge_progress_receiver: f_receiver,
            forge_progress_num: 0.0,
            java_progress_receiver: j_receiver,
            java_progress_num: 0.0,
            is_java_getting_installed: false,
            forge_message: "Installing Forge".to_owned(),
            java_message: None,
        });
        command
    }

    fn add_server_to_processes(
        &mut self,
        child: Arc<std::sync::Mutex<tokio::process::Child>>,
        is_classic_server: bool,
    ) -> Command<Message> {
        let Some(InstanceSelection::Server(selected_server)) = &self.selected_instance else {
            err!("Launched server but can't identify which one! This is a bug, please report it");
            return Command::none();
        };
        if let (Some(stdout), Some(stderr), Some(stdin)) = {
            let mut child = child.lock().unwrap();
            (child.stdout.take(), child.stderr.take(), child.stdin.take())
        } {
            let (sender, receiver) = std::sync::mpsc::channel();

            self.server_processes.insert(
                selected_server.clone(),
                ServerProcess {
                    child: child.clone(),
                    receiver: Some(receiver),
                    stdin: Some(stdin),
                    is_classic_server,
                    name: selected_server.clone(),
                    has_issued_stop_command: false,
                },
            );

            return Command::perform(
                ql_servers::read_logs_w(stdout, stderr, child, sender, selected_server.clone()),
                Message::ServerManageEndedLog,
            );
        }

        self.server_processes.insert(
            selected_server.clone(),
            ServerProcess {
                child: child.clone(),
                receiver: None,
                stdin: None,
                is_classic_server,
                name: "Unknown".to_owned(),
                has_issued_stop_command: false,
            },
        );
        Command::none()
    }

    fn go_to_main_menu(&mut self, message: Option<String>) -> Command<Message> {
        match self.selected_instance.as_ref().unwrap() {
            InstanceSelection::Instance(_) => self.go_to_launch_screen(message),
            InstanceSelection::Server(_) => self.go_to_server_manage_menu(message),
        }
    }
}

// async fn pick_file() -> Option<PathBuf> {
//     const MESSAGE: &str = if cfg!(windows) {
//         "Select the java.exe executable"
//     } else {
//         "Select the java executable"
//     };

//     rfd::AsyncFileDialog::new()
//         .set_title(MESSAGE)
//         .pick_file()
//         .await
//         .map(|n| n.path().to_owned())
// }

const WINDOW_HEIGHT: f32 = 450.0;
const WINDOW_WIDTH: f32 = 650.0;

fn main() {
    let args = std::env::args();
    let mut info = ArgumentInfo {
        headless: false,
        operation: None,
        is_used: false,
    };
    process_args(args, &mut info);

    if !info.is_used {
        info!("Welcome to QuantumLauncher! This terminal window just outputs some debug info. You can ignore it.");
    }

    if let Some(op) = info.operation {
        match op {
            ArgumentOperation::ListInstances => {
                match tokio::runtime::Runtime::new()
                    .unwrap()
                    .block_on(get_entries("instances".to_owned(), false))
                    .map_err(|err| err.to_string())
                {
                    Ok((instances, _)) => {
                        for instance in instances {
                            let launcher_dir = file_utils::get_launcher_dir().unwrap();
                            let instance_dir = launcher_dir.join("instances").join(&instance);

                            let json =
                                std::fs::read_to_string(instance_dir.join("details.json")).unwrap();
                            let json: VersionDetails = serde_json::from_str(&json).unwrap();

                            let config_json =
                                std::fs::read_to_string(instance_dir.join("config.json")).unwrap();
                            let config_json: InstanceConfigJson =
                                serde_json::from_str(&config_json).unwrap();

                            println!("{instance} : {} : {}", json.id, config_json.mod_type);
                        }
                    }
                    Err(err) => eprintln!("[cmd.error] {err}"),
                }
            }
        }
    }

    if info.headless {
        return;
    }
    info!("Starting up the launcher...");

    Launcher::run(Settings {
        window: iced::window::Settings {
            size: iced::Size {
                width: WINDOW_WIDTH,
                height: WINDOW_HEIGHT,
            },
            resizable: true,
            ..Default::default()
        },
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
    .unwrap();
}

struct ArgumentInfo {
    pub headless: bool,
    pub is_used: bool,
    pub operation: Option<ArgumentOperation>,
}

enum ArgumentOperation {
    ListInstances,
}

fn process_args(mut args: std::env::Args, info: &mut ArgumentInfo) -> Option<()> {
    let program = args.next()?;
    let mut first_argument = true;

    loop {
        let Some(command) = args.next() else {
            if first_argument {
                info!(
                    "You can run {} to see the possible command line arguments",
                    format!("{program} --help").yellow()
                );
            }
            return None;
        };
        info.is_used = true;
        match command.as_str() {
            "--help" => {
                println!(
                    r#"Usage: {}
    --help           : Print a list of valid command line flags
    --version        : Print the launcher version
    --command        : Run a command with the launcher in headless mode (command line)
    --list-instances : Print a list of instances (name, version and type (Vanilla/Fabric/Forge/...))
"#,
                    format!("{program} [FLAGS]").yellow(),
                );
            }
            "--version" => {
                println!(
                    "{}",
                    format!("QuantumLauncher v{LAUNCHER_VERSION_NAME} - made by Mrmayman").bold()
                );
            }
            "--command" => {
                info.headless = true;
            }
            "--list-instances" => info.operation = Some(ArgumentOperation::ListInstances),
            _ => {
                eprintln!(
                    "{} Unknown flag! Type {} to see all the command-line flags.",
                    "[error]".red(),
                    format!("{program} --help").yellow()
                );
            }
        }
        first_argument = false;
    }
}
