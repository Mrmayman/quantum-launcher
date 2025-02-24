use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
    path::Path,
};

use iced::Task;
use ql_core::{err, InstanceSelection};
use ql_instances::LogLine;
use ql_mod_manager::mod_manager::{ModConfig, ModIndex, Search};

use crate::launcher_state::{
    get_entries, EditInstanceMessage, InstallModsMessage, InstanceLog, LaunchTabId, Launcher,
    MenuCreateInstance, MenuEditMods, MenuEditPresetsInner, MenuInstallFabric, MenuLaunch,
    MenuServerCreate, Message, ModListEntry, ProgressBar, ServerProcess, State,
};

impl Launcher {
    pub fn tick(&mut self) -> Task<Message> {
        match &mut self.state {
            State::Launch(MenuLaunch {
                asset_recv,
                edit_instance,
                ..
            }) => {
                if let Some(receiver) = &mut self.java_recv {
                    if receiver.tick() {
                        self.state = State::InstallJava;
                        return Task::none();
                    }
                }

                if let Some(receiver) = asset_recv.take() {
                    if receiver.try_recv().is_ok() {
                        self.state = State::RedownloadAssets {
                            progress: ProgressBar::with_recv(receiver),
                        };
                        return Task::none();
                    }
                    *asset_recv = Some(receiver);
                }

                let mut commands = Vec::new();

                if let Some(edit) = edit_instance.as_ref() {
                    let cmd = Task::perform(
                        Launcher::save_config_w(
                            self.selected_instance.clone().unwrap(),
                            edit.config.clone(),
                            self.dir.clone(),
                        ),
                        |n| Message::EditInstance(EditInstanceMessage::ConfigSaved(n)),
                    );
                    commands.push(cmd);
                }

                self.tick_processes_and_logs();

                if let Some(config) = self.config.clone() {
                    commands.push(Task::perform(config.save_w(), Message::CoreTickConfigSaved));
                }
                return Task::batch(commands);
            }
            State::Create(menu) => menu.tick(),
            State::EditMods(menu) => {
                let instance_selection = self.selected_instance.as_ref().unwrap();
                let update_locally_installed_mods = menu.tick(instance_selection, &self.dir);
                return update_locally_installed_mods;
            }
            State::InstallFabric(menu) => {
                if let MenuInstallFabric::Loaded {
                    progress: Some(progress),
                    ..
                } = menu
                {
                    progress.tick();
                }
            }
            State::InstallForge(menu) => {
                menu.forge_progress.tick();
                if menu.java_progress.tick() {
                    menu.is_java_getting_installed = true;
                }
            }
            State::UpdateFound(menu) => {
                if let Some(progress) = &mut menu.progress {
                    progress.tick();
                }
            }
            State::InstallJava => {
                let has_finished = if let Some(progress) = &mut self.java_recv {
                    progress.tick();
                    progress.progress.has_finished
                } else {
                    true
                };
                if has_finished {
                    self.java_recv = None;
                    let message = "Installed Java".to_owned();
                    match &self.selected_instance {
                        Some(InstanceSelection::Instance(_)) | None => {
                            return self.go_to_launch_screen(Some(message));
                        }
                        Some(InstanceSelection::Server(_)) => {
                            return self.go_to_server_manage_menu(Some(message));
                        }
                    }
                }
            }
            State::ModsDownload(menu) => {
                let index_cmd = Task::perform(
                    ModIndex::get_w(self.selected_instance.clone().unwrap()),
                    |n| Message::InstallMods(InstallModsMessage::IndexUpdated(n)),
                );

                if let Some(results) = &menu.results {
                    let mut commands = vec![index_cmd];
                    for result in &results.hits {
                        if commands.len() > 64 {
                            break;
                        }
                        if !self.images.downloads_in_progress.contains(&result.title)
                            && !result.icon_url.is_empty()
                        {
                            self.images
                                .downloads_in_progress
                                .insert(result.title.clone());
                            commands.push(Task::perform(
                                Search::download_image(result.icon_url.clone(), true),
                                |n| Message::InstallMods(InstallModsMessage::ImageDownloaded(n)),
                            ));
                        }
                    }

                    return Task::batch(commands);
                }
                return index_cmd;
            }
            State::LauncherSettings => {
                if let Some(config) = self.config.clone() {
                    return Task::perform(config.save_w(), Message::CoreTickConfigSaved);
                }
            }
            State::RedownloadAssets { progress } => {
                progress.tick();
                if progress.progress.has_finished {
                    let message = "Redownloaded Assets".to_owned();
                    self.state = State::Launch(MenuLaunch {
                        message,
                        asset_recv: None,
                        tab: LaunchTabId::default(),
                        edit_instance: None,
                        login_progress: None,
                        sidebar_width: 200,
                        sidebar_dragging: false,
                    });
                    return Task::perform(
                        get_entries("instances".to_owned(), false),
                        Message::CoreListLoaded,
                    );
                }
            }
            State::InstallOptifine(menu) => {
                if let Some(optifine_progress) = &mut menu.optifine_install_progress {
                    optifine_progress.tick();
                }
                if let Some(java_progress) = &mut menu.java_install_progress {
                    if java_progress.tick() {
                        menu.is_java_being_installed = true;
                    }
                }
            }
            State::ServerManage(_) => {
                if self.java_recv.as_mut().map(|n| n.tick()).unwrap_or(false) {
                    self.state = State::InstallJava;
                    return Task::none();
                }

                self.tick_server_processes_and_logs();
            }
            State::ServerCreate(menu) => match menu {
                MenuServerCreate::LoadingList {
                    progress_receiver,
                    progress_number,
                } => {
                    while let Ok(()) = progress_receiver.try_recv() {
                        *progress_number += 1.0;
                        if *progress_number > 16.0 {
                            err!("More than 16 indexes scraped: {progress_number}");
                            *progress_number = 16.0;
                        }
                    }
                }
                MenuServerCreate::Loaded { .. } => {}
                MenuServerCreate::Downloading { progress } => {
                    progress.tick();
                }
            },
            State::ManagePresets(menu) => {
                if let Some(progress) = &mut menu.progress {
                    progress.tick();
                }

                if let MenuEditPresetsInner::Recommended { progress, .. } = &mut menu.inner {
                    progress.tick();
                }
            }
            State::AccountLoginProgress(progress) => {
                progress.tick();
            }
            // These menus don't require background ticking
            State::Error { .. }
            | State::ConfirmAction { .. }
            | State::ChangeLog
            | State::Welcome
            | State::AccountLogin { .. }
            | State::GenericMessage(_)
            | State::InstallPaper => {}
        }

        Task::none()
    }

    pub fn get_imgs_to_load(&mut self) -> Vec<Task<Message>> {
        let mut commands = Vec::new();

        let mut images_to_load = self.images.to_load.lock().unwrap();

        for url in images_to_load.iter() {
            if !self.images.downloads_in_progress.contains(url) {
                self.images.downloads_in_progress.insert(url.to_owned());
                commands.push(Task::perform(
                    Search::download_image(url.to_owned(), false),
                    |n| Message::InstallMods(InstallModsMessage::ImageDownloaded(n)),
                ));
            }
        }

        images_to_load.clear();
        commands
    }

    fn tick_processes_and_logs(&mut self) {
        let mut killed_processes = Vec::new();
        for (name, process) in &self.client_processes {
            if let Ok(Some(_)) = process.child.lock().unwrap().try_wait() {
                // Game process has exited.
                killed_processes.push(name.to_owned());
            } else {
                Launcher::read_game_logs(&mut self.client_logs, process, name);
            }
        }
        for name in killed_processes {
            self.client_processes.remove(&name);
        }
    }

    fn tick_server_processes_and_logs(&mut self) {
        let mut killed_processes = Vec::new();
        for (name, process) in &self.server_processes {
            if let Ok(Some(_)) = process.child.lock().unwrap().try_wait() {
                // Game process has exited.
                killed_processes.push(name.to_owned());
            } else {
                Self::tick_server_logs(process, name, &mut self.server_logs);
            }
        }
        for name in killed_processes {
            self.server_processes.remove(&name);
        }
    }

    fn tick_server_logs(
        process: &ServerProcess,
        name: &String,
        server_logs: &mut HashMap<String, InstanceLog>,
    ) {
        while let Some(message) = process.receiver.as_ref().and_then(|n| n.try_recv().ok()) {
            if let Some(log) = server_logs.get_mut(name) {
                if log.log.is_empty() {
                    log.log.push_str(&format!(
                        "Starting Minecraft Server ({})\nOS: {}\n\n",
                        Self::get_current_date_formatted(),
                        ql_instances::OS_NAME
                    ));
                }
                log.log.push_str(&message);
            } else {
                server_logs.insert(
                    name.to_owned(),
                    InstanceLog {
                        log: format!(
                            "Starting Minecraft Server ({})\nOS: {}\n\n{}",
                            Self::get_current_date_formatted(),
                            ql_instances::OS_NAME,
                            message
                        ),
                        has_crashed: false,
                        command: String::new(),
                    },
                );
            }
        }
    }

    fn read_game_logs(
        logs: &mut HashMap<String, InstanceLog>,
        process: &crate::launcher_state::ClientProcess,
        name: &String,
    ) {
        let Some(receiver) = process.receiver.as_ref() else {
            return;
        };
        while let Ok(message) = receiver.try_recv() {
            let message = match message {
                LogLine::Info(event) => event.to_string(),
                LogLine::Error(error) => format!("! {error}"),
                LogLine::Message(message) => message,
            };

            if !logs.contains_key(name) {
                logs.insert(
                    name.to_owned(),
                    InstanceLog {
                        log: format!(
                            "Launching Minecraft ({})\nOS: {}\n\n{}",
                            Self::get_current_date_formatted(),
                            ql_instances::OS_NAME,
                            message
                        ),
                        has_crashed: false,
                        command: String::new(),
                    },
                );
            } else if let Some(log) = logs.get_mut(name) {
                if log.log.is_empty() {
                    log.log.push_str(&format!(
                        "Launching Minecraft ({})\nOS: {}\n\n",
                        Self::get_current_date_formatted(),
                        ql_instances::OS_NAME
                    ));
                }
                log.log.push_str(&message);
            }
        }
    }
}

pub fn sort_dependencies(
    downloaded_mods: &HashMap<String, ModConfig>,
    locally_installed_mods: &HashSet<String>,
) -> Vec<ModListEntry> {
    let mut entries: Vec<ModListEntry> = downloaded_mods
        .iter()
        .map(|(k, v)| ModListEntry::Downloaded {
            id: k.clone(),
            config: Box::new(v.clone()),
        })
        .chain(locally_installed_mods.iter().map(|n| ModListEntry::Local {
            file_name: n.clone(),
        }))
        .collect();
    entries.sort_by(|val1, val2| match (val1, val2) {
        (
            ModListEntry::Downloaded { config, .. },
            ModListEntry::Downloaded {
                config: config2, ..
            },
        ) => match (config.manually_installed, config2.manually_installed) {
            (true, true) | (false, false) => config.name.cmp(&config2.name),
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
        },
        (ModListEntry::Downloaded { config, .. }, ModListEntry::Local { .. }) => {
            if config.manually_installed {
                Ordering::Less
            } else {
                Ordering::Greater
            }
        }
        (ModListEntry::Local { .. }, ModListEntry::Downloaded { config, .. }) => {
            if config.manually_installed {
                Ordering::Greater
            } else {
                Ordering::Less
            }
        }
        (
            ModListEntry::Local { file_name },
            ModListEntry::Local {
                file_name: file_name2,
            },
        ) => file_name.cmp(file_name2),
    });

    entries
}

impl MenuEditMods {
    fn tick(&mut self, instance_selection: &InstanceSelection, dir: &Path) -> Task<Message> {
        self.sorted_mods_list = sort_dependencies(&self.mods.mods, &self.locally_installed_mods);

        if let Some(progress) = &mut self.mod_update_progress {
            progress.tick();
            if progress.progress.has_finished {
                self.mod_update_progress = None;
            }
        }

        MenuEditMods::update_locally_installed_mods(&self.mods, instance_selection, dir)
    }
}

impl MenuCreateInstance {
    pub fn tick(&mut self) {
        match self {
            MenuCreateInstance::Loading {
                progress_receiver,
                progress_number,
            } => {
                while let Ok(()) = progress_receiver.try_recv() {
                    *progress_number += 1.0;
                    if *progress_number > 24.0 {
                        err!("More than 24 indexes scraped: {}", *progress_number);
                    }
                }
            }
            MenuCreateInstance::Loaded { progress, .. } => {
                if let Some(progress) = progress {
                    progress.tick();
                }
            }
        }
    }
}
