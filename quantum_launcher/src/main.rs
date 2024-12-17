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

use std::{path::PathBuf, time::Duration};

use colored::Colorize;
use iced::{
    executor,
    widget::{self, image::Handle},
    Application, Command, Settings,
};
use launcher_state::{
    reload_instances, Launcher, MenuEditMods, MenuInstallForge, MenuInstallOptifine, MenuLaunch,
    MenuLauncherSettings, MenuLauncherUpdate, Message, OptifineInstallProgressData, SelectedMod,
    SelectedState, State, UpdateModsProgress,
};

use menu_renderer::menu_delete_instance_view;
use message_handler::open_file_explorer;
use ql_instances::{
    err,
    error::IoError,
    file_utils, info,
    json_structs::{json_instance_config::InstanceConfigJson, json_version::VersionDetails},
    UpdateCheckInfo, LAUNCHER_VERSION_NAME,
};
use ql_mod_manager::{
    instance_mod_installer,
    mod_manager::{ModIndex, ProjectInfo},
};
use stylesheet::styles::{LauncherStyle, LauncherTheme};

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
            ql_instances::check_for_updates_wrapped(),
            Message::UpdateCheckResult,
        );

        let command = if let Some(command) = load_icon_command {
            Command::batch(vec![command, check_for_updates_command])
        } else {
            check_for_updates_command
        };

        (
            match Launcher::new(None) {
                Ok(launcher) => launcher,
                Err(error) => Launcher::with_error(&error.to_string()),
            },
            command,
        )
    }

    fn title(&self) -> String {
        "Quantum Launcher".to_owned()
    }

    fn update(&mut self, message: Self::Message) -> iced::Command<Self::Message> {
        match message {
            Message::LaunchInstanceSelected(selected_instance) => {
                self.select_launch_instance(selected_instance);
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
            Message::DeleteInstance => self.delete_selected_instance(),
            Message::LaunchScreenOpen(message) => {
                if let Some(message) = message {
                    self.go_to_launch_screen_with_message(message);
                } else {
                    self.go_to_launch_screen();
                }
            }
            Message::EditInstance(message) => return self.update_edit_instance(message),
            Message::ManageModsScreenOpen => match self.go_to_edit_mods_menu() {
                Ok(command) => return command,
                Err(err) => self.set_error(err.to_string()),
            },
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
                    instance_mod_installer::forge::uninstall_wrapped(
                        self.selected_instance.clone().unwrap(),
                    ),
                    Message::UninstallLoaderEnd,
                );
            }
            Message::UninstallLoaderOptiFineStart => {
                return Command::perform(
                    instance_mod_installer::optifine::uninstall_wrapped(
                        self.selected_instance.clone().unwrap(),
                    ),
                    Message::UninstallLoaderEnd,
                );
            }
            Message::UninstallLoaderFabricStart => {
                return Command::perform(
                    instance_mod_installer::fabric::uninstall_wrapped(
                        self.selected_instance.clone().unwrap(),
                    ),
                    Message::UninstallLoaderEnd,
                );
            }
            Message::UninstallLoaderEnd(result) => match result {
                Ok(loader) => {
                    self.go_to_launch_screen_with_message(format!("Uninstalled {loader}"))
                }
                Err(err) => self.set_error(err),
            },
            Message::InstallForgeStart => {
                let (f_sender, f_receiver) = std::sync::mpsc::channel();
                let (j_sender, j_receiver) = std::sync::mpsc::channel();

                let command = Command::perform(
                    instance_mod_installer::forge::install_wrapped(
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

                return command;
            }
            Message::InstallForgeEnd(result) => match result {
                Ok(()) => self.go_to_launch_screen_with_message("Installed Forge".to_owned()),
                Err(err) => self.set_error(err),
            },
            Message::LaunchEndedLog(result) => match result {
                Ok(status) => {
                    info!("Game exited with status: {status}");
                    self.set_game_crashed(status);
                }
                Err(err) => self.set_error(err),
            },
            Message::LaunchKill => {
                if let Some(process) = self
                    .processes
                    .remove(self.selected_instance.as_ref().unwrap())
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
                if let Some(log) = self.logs.get(self.selected_instance.as_ref().unwrap()) {
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
                        ql_instances::install_update_wrapped(url.clone(), sender),
                        Message::UpdateDownloadEnd,
                    );
                }
            }
            Message::UpdateDownloadEnd(err) => {
                if let Err(err) = err {
                    self.set_error(err);
                } else {
                    self.go_to_launch_screen_with_message(
                        "Updated launcher! Close and reopen the launcher to see the new update"
                            .to_owned(),
                    );
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
                        Err(err) => self.set_error(err.to_string()),
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

                    return menu.search_modrinth();
                }
            }
            Message::InstallModsClick(i) => {
                if let State::ModsDownload(menu) = &mut self.state {
                    menu.opened_mod = Some(i);
                    if let Some(results) = &menu.results {
                        let hit = results.hits.get(i).unwrap();
                        if !menu.result_data.contains_key(&hit.project_id) {
                            let task = ProjectInfo::download_wrapped(hit.project_id.clone());
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
                Ok((name, path)) => {
                    self.images.insert(name, Handle::from_memory(path));
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
            Message::ManageModsToggleCheckboxLocal(name, enable) => {
                if let State::EditMods(menu) = &mut self.state {
                    if enable {
                        menu.selected_mods
                            .insert(SelectedMod::Local { file_name: name });
                        menu.selected_state = SelectedState::Some;
                    } else {
                        menu.selected_mods
                            .remove(&SelectedMod::Local { file_name: name });
                        menu.selected_state = if menu.selected_mods.is_empty() {
                            SelectedState::None
                        } else {
                            SelectedState::Some
                        };
                    }
                }
            }
            Message::ManageModsToggleCheckbox((name, id), enable) => {
                if let State::EditMods(menu) = &mut self.state {
                    if enable {
                        menu.selected_mods
                            .insert(SelectedMod::Downloaded { name, id });
                        menu.selected_state = SelectedState::Some;
                    } else {
                        menu.selected_mods
                            .remove(&SelectedMod::Downloaded { name, id });
                        menu.selected_state = if menu.selected_mods.is_empty() {
                            SelectedState::None
                        } else {
                            SelectedState::Some
                        };
                    }
                }
            }
            Message::ManageModsDeleteSelected => {
                if let State::EditMods(menu) = &self.state {
                    let ids = menu
                        .selected_mods
                        .iter()
                        .filter_map(|s_mod| {
                            if let SelectedMod::Downloaded { id, .. } = s_mod {
                                Some(id.clone())
                            } else {
                                None
                            }
                        })
                        .collect();

                    let command = Command::perform(
                        ql_mod_manager::mod_manager::delete_mods_wrapped(
                            ids,
                            self.selected_instance.clone().unwrap(),
                        ),
                        Message::ManageModsDeleteFinished,
                    );

                    let mods_dir = file_utils::get_launcher_dir()
                        .unwrap()
                        .join("instances")
                        .join(self.selected_instance.as_ref().unwrap())
                        .join(".minecraft/mods");
                    let file_paths = menu
                        .selected_mods
                        .iter()
                        .filter_map(|s_mod| {
                            if let SelectedMod::Local { file_name } = s_mod {
                                Some(file_name.clone())
                            } else {
                                None
                            }
                        })
                        .map(|n| mods_dir.join(n))
                        .map(delete_file_wrapper)
                        .map(|n| Command::perform(n, Message::ManageModsLocalDeleteFinished));
                    let delete_local_command = Command::batch(file_paths);

                    return Command::batch(vec![command, delete_local_command]);
                }
            }
            Message::ManageModsLocalDeleteFinished(result) => {
                if let Err(err) = result {
                    self.set_error(err);
                }
            }
            Message::ManageModsLocalIndexLoaded(hash_set) => {
                if let State::EditMods(menu) = &mut self.state {
                    menu.locally_installed_mods = hash_set;
                }
            }
            Message::ManageModsDeleteFinished(result) => match result {
                Ok(_) => {
                    self.update_mod_index();
                }
                Err(err) => self.set_error(err),
            },
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
            Message::ManageModsToggleSelected => {
                if let State::EditMods(menu) = &self.state {
                    let ids = menu
                        .selected_mods
                        .iter()
                        .filter_map(|s_mod| {
                            if let SelectedMod::Downloaded { id, .. } = s_mod {
                                Some(id.clone())
                            } else {
                                None
                            }
                        })
                        .collect();
                    return Command::perform(
                        ql_mod_manager::mod_manager::toggle_mods_wrapped(
                            ids,
                            self.selected_instance.clone().unwrap(),
                        ),
                        Message::ManageModsToggleFinished,
                    );
                }
            }
            Message::ManageModsToggleFinished(err) => {
                if let Err(err) = err {
                    self.set_error(err);
                } else {
                    self.update_mod_index();
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
                        ql_mod_manager::instance_mod_installer::optifine::install_optifine_wrapped(
                            self.selected_instance.clone().unwrap(),
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
                    self.go_to_launch_screen_with_message("Installed OptiFine".to_owned());
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
            Message::ManageModsUpdateMods => return self.update_mods(),
            Message::ManageModsUpdateModsFinished(result) => {
                if let Err(err) = result {
                    self.set_error(err);
                } else {
                    self.update_mod_index();
                    if let State::EditMods(menu) = &mut self.state {
                        menu.available_updates.clear();
                    }
                    return Command::perform(
                        ql_mod_manager::mod_manager::check_for_updates(
                            self.selected_instance.clone().unwrap(),
                        ),
                        Message::ManageModsUpdateCheckResult,
                    );
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
                self.instances.as_deref(),
                &self.processes,
                &self.logs,
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
                    widget::button("Back").on_press(Message::LaunchScreenOpen(None)),
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
            State::ModsDownload(menu) => menu.view(&self.images, &self.images_to_load),
            State::LauncherSettings => MenuLauncherSettings::view(self.config.as_ref()),
            State::RedownloadAssets(menu) => widget::column!(
                widget::text("Redownloading Assets").size(20),
                widget::progress_bar(0.0..=1.0, menu.num),
            )
            .padding(10)
            .spacing(10)
            .into(),
            State::InstallOptifine(menu) => menu.view(),
        }
    }

    fn theme(&self) -> Self::Theme {
        self.theme.clone()
    }

    fn scale_factor(&self) -> f64 {
        1.0
    }
}

async fn delete_file_wrapper(path: PathBuf) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    tokio::fs::remove_file(&path).await.map_err(|error| {
        IoError::Io {
            error,
            path: path.to_owned(),
        }
        .to_string()
    })
}

fn load_window_icon() -> Option<Command<Message>> {
    let icon = iced::window::icon::from_file_data(LAUNCHER_ICON, Some(image::ImageFormat::Ico));
    match icon {
        Ok(icon) => Some(iced::window::change_icon(iced::window::Id::MAIN, icon)),
        Err(err) => {
            err!("Could not load icon: {err}");
            None
        }
    }
}

impl Launcher {
    fn mod_download(&mut self, index: usize) -> Option<Command<Message>> {
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
        let selected_instance = self.selected_instance.as_ref()?;

        menu.mods_download_in_progress
            .insert(hit.project_id.clone());
        Some(Command::perform(
            ql_mod_manager::mod_manager::download_mod_wrapped(
                hit.project_id.clone(),
                selected_instance.to_owned(),
            ),
            Message::InstallModsDownloadComplete,
        ))
    }

    fn set_game_crashed(&mut self, status: std::process::ExitStatus) {
        if let State::Launch(MenuLaunch { message, .. }) = &mut self.state {
            let has_crashed = !status.success();
            if has_crashed {
                *message =
                    format!("Game Crashed with code: {status}\nCheck Logs for more information");
            }
            if let Some(log) = self.logs.get_mut(self.selected_instance.as_ref().unwrap()) {
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
                ql_mod_manager::mod_manager::apply_updates_wrapped(
                    self.selected_instance.clone().unwrap(),
                    updates,
                    Some(sender),
                ),
                Message::ManageModsUpdateModsFinished,
            )
        } else {
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
                match reload_instances().map_err(|err| err.to_string()) {
                    Ok(instances) => {
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
            include_bytes!("../../assets/Inter-Regular.ttf")
                .as_slice()
                .into(),
            include_bytes!("../../assets/launcher-icons.ttf")
                .as_slice()
                .into(),
            include_bytes!("../../assets/JetBrainsMono-Regular.ttf")
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
