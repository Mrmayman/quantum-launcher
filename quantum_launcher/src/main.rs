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

use arguments::ArgumentInfo;
use iced::{executor, widget, Application, Command, Settings};
use launcher_state::{
    get_entries, Launcher, MenuEditPresets, MenuEditPresetsInner, MenuLauncherSettings,
    MenuLauncherUpdate, MenuServerCreate, Message, ModListEntry, ProgressBar, SelectedState,
    ServerProcess, State,
};

use menu_renderer::{button_with_icon, changelog::changelog_0_3_1, menu_delete_instance_view};
use message_handler::open_file_explorer;
use ql_core::{err, info, GenericProgress, InstanceSelection, SelectedMod};
use ql_instances::UpdateCheckInfo;
use ql_mod_manager::{loaders, mod_manager::Loader};
use stylesheet::styles::{LauncherStyle, LauncherTheme};
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

        let get_entries_command = Command::perform(
            get_entries("instances".to_owned(), false),
            Message::CoreListLoaded,
        );

        (
            Launcher::load_new(None).unwrap_or_else(Launcher::with_error),
            Command::batch(vec![
                load_icon_command,
                check_for_updates_command,
                get_entries_command,
            ]),
        )
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
                    loaders::forge::uninstall_w(self.selected_instance.clone().unwrap()),
                    Message::UninstallLoaderEnd,
                )
            }
            Message::UninstallLoaderOptiFineStart => {
                return Command::perform(
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
                return Command::perform(
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
                return Command::perform(
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
            Message::EditPresetsOpen => return self.go_to_edit_presets_menu(),
            Message::EditPresetsToggleCheckbox((name, id), enable) => {
                if let State::ManagePresets(MenuEditPresets {
                    inner:
                        MenuEditPresetsInner::Build {
                            selected_mods,
                            selected_state,
                            ..
                        },
                    ..
                }) = &mut self.state
                {
                    if enable {
                        selected_mods.insert(SelectedMod::Downloaded { name, id });
                    } else {
                        selected_mods.remove(&SelectedMod::Downloaded { name, id });
                    }
                    *selected_state = SelectedState::Some;
                }
            }
            Message::EditPresetsToggleCheckboxLocal(file_name, enable) => {
                if let State::ManagePresets(MenuEditPresets {
                    inner:
                        MenuEditPresetsInner::Build {
                            selected_mods,
                            selected_state,
                            ..
                        },
                    ..
                }) = &mut self.state
                {
                    if enable {
                        selected_mods.insert(SelectedMod::Local { file_name });
                    } else {
                        selected_mods.remove(&SelectedMod::Local { file_name });
                    }
                    *selected_state = SelectedState::Some;
                }
            }
            Message::EditPresetsSelectAll => {
                if let State::ManagePresets(MenuEditPresets {
                    inner:
                        MenuEditPresetsInner::Build {
                            selected_mods,
                            selected_state,
                            mods,
                            ..
                        },
                    ..
                }) = &mut self.state
                {
                    match selected_state {
                        SelectedState::All => {
                            selected_mods.clear();
                            *selected_state = SelectedState::None;
                        }
                        SelectedState::Some | SelectedState::None => {
                            *selected_mods = mods
                                .iter()
                                .filter_map(|mod_info| {
                                    mod_info.is_manually_installed().then_some(match mod_info {
                                        ModListEntry::Downloaded { id, config } => {
                                            SelectedMod::Downloaded {
                                                name: config.name.clone(),
                                                id: id.clone(),
                                            }
                                        }
                                        ModListEntry::Local { file_name } => SelectedMod::Local {
                                            file_name: file_name.clone(),
                                        },
                                    })
                                })
                                .collect();
                            *selected_state = SelectedState::All;
                        }
                    }
                }
            }
            Message::EditPresetsBuildYourOwn => {
                if let State::ManagePresets(MenuEditPresets {
                    inner:
                        MenuEditPresetsInner::Build {
                            selected_mods,
                            is_building,
                            ..
                        },
                    ..
                }) = &mut self.state
                {
                    *is_building = true;
                    return Command::perform(
                        ql_mod_manager::PresetJson::generate_w(
                            self.selected_instance.clone().unwrap(),
                            selected_mods.clone(),
                        ),
                        Message::EditPresetsBuildYourOwnEnd,
                    );
                }
            }
            Message::EditPresetsBuildYourOwnEnd(result) => match result {
                Ok(preset) => {
                    if let Some(path) = rfd::FileDialog::new().save_file() {
                        if let Err(err) = std::fs::write(path, preset) {
                            self.set_error(err);
                        } else {
                            match self.go_to_edit_mods_menu() {
                                Ok(n) => return n,
                                Err(err) => self.set_error(err),
                            }
                        }
                    }
                }
                Err(err) => self.set_error(err),
            },
            Message::CoreOpenChangeLog => {
                self.state = State::ChangeLog;
            }
            Message::EditPresetsLoad => return self.load_preset(),
            Message::EditPresetsLoadComplete(result) => {
                if let Err(err) = result {
                    self.set_error(err);
                } else {
                    match self.go_to_edit_mods_menu() {
                        Ok(cmd) => return cmd,
                        Err(err) => self.set_error(err),
                    }
                }
            }
            Message::EditPresetsRecommendedModCheck(result) => {
                if let State::ManagePresets(MenuEditPresets {
                    inner: MenuEditPresetsInner::Recommended { mods, error, .. },
                    ..
                }) = &mut self.state
                {
                    match result {
                        Ok(n) => {
                            *mods = Some(n.into_iter().map(|n| (true, n)).collect());
                        }
                        Err(err) => *error = Some(err),
                    }
                }
            }
            Message::EditPresetsRecommendedToggle(idx, toggle) => {
                if let State::ManagePresets(MenuEditPresets {
                    inner:
                        MenuEditPresetsInner::Recommended {
                            mods: Some(mods), ..
                        },
                    ..
                }) = &mut self.state
                {
                    if let Some((t, _)) = mods.get_mut(idx) {
                        *t = toggle;
                    }
                }
            }
            Message::EditPresetsRecommendedDownload => {
                if let State::ManagePresets(MenuEditPresets {
                    inner:
                        MenuEditPresetsInner::Recommended {
                            mods: Some(mods), ..
                        },
                    progress,
                    ..
                }) = &mut self.state
                {
                    let (sender, receiver) = std::sync::mpsc::channel();

                    *progress = Some(ProgressBar {
                        num: 0.0,
                        message: None,
                        receiver,
                        progress: GenericProgress::default(),
                    });

                    return Command::perform(
                        ql_mod_manager::mod_manager::download_mods_w(
                            mods.iter()
                                .filter(|n| n.0)
                                .map(|n| n.1.id.to_owned())
                                .collect(),
                            self.selected_instance.clone().unwrap(),
                            sender,
                        ),
                        Message::EditPresetsRecommendedDownloadEnd,
                    );
                }
            }
            Message::EditPresetsRecommendedDownloadEnd(result) => {
                if let Err(err) = result {
                    self.set_error(err);
                } else {
                    match self.go_to_edit_mods_menu_without_update_check() {
                        Ok(n) => return n,
                        Err(err) => self.set_error(err),
                    }
                }
            }
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
            State::InstallJava(bar) => {
                widget::column!(widget::text("Downloading Java").size(20), bar.view())
                    .padding(10)
                    .spacing(10)
                    .into()
            }
            State::ModsDownload(menu) => {
                menu.view(&self.images_bitmap, &self.images_svg, &self.images_to_load)
            }
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
            State::ManagePresets(menu) => menu.view(),
            State::ChangeLog => widget::scrollable(
                widget::column!(
                    button_with_icon(icon_manager::back(), "Back").on_press(
                        Message::LaunchScreenOpen {
                            message: None,
                            clear_selection: true
                        }
                    ),
                    changelog_0_3_1()
                )
                .padding(10)
                .spacing(10),
            )
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
    let mut args = std::env::args();
    let mut info = ArgumentInfo {
        headless: false,
        program: None,
    };
    arguments::process_args(&mut args, &mut info);

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
