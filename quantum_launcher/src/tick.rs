use std::collections::{HashMap, HashSet};

use iced::Command;
use ql_core::{err, info, GenericProgress, InstanceSelection};
use ql_instances::LogLine;
use ql_mod_manager::mod_manager::{ModConfig, ModIndex, Search};

use crate::launcher_state::{
    get_entries, InstallModsMessage, InstanceLog, Launcher, MenuCreateInstance, MenuEditMods,
    MenuEditPresetsInner, MenuInstallFabric, MenuLaunch, MenuServerCreate, Message, ModListEntry,
    ProgressBar, ServerProcess, State,
};

impl Launcher {
    pub fn tick(&mut self) -> Command<Message> {
        match &mut self.state {
            State::Launch(MenuLaunch {
                java_recv,
                asset_recv,
                ..
            }) => {
                if let Some(receiver) = java_recv.take() {
                    if let Ok(GenericProgress { done: 0, .. }) = receiver.try_recv() {
                        info!("Installing Java");
                        self.state = State::InstallJava(ProgressBar::with_recv_and_msg(
                            receiver,
                            "Starting".to_owned(),
                        ));
                        return Command::none();
                    }
                    *java_recv = Some(receiver);
                }

                if let Some(receiver) = asset_recv.take() {
                    if receiver.try_recv().is_ok() {
                        self.state = State::RedownloadAssets {
                            progress: ProgressBar::with_recv(receiver),
                            java_recv: java_recv.take(),
                        };
                        return Command::none();
                    }
                    *asset_recv = Some(receiver);
                }

                self.tick_processes_and_logs();

                if let Some(config) = self.config.clone() {
                    return Command::perform(config.save_w(), Message::CoreTickConfigSaved);
                }
            }
            State::EditInstance(menu) => {
                if let Err(err) =
                    Launcher::save_config(self.selected_instance.as_ref().unwrap(), &menu.config)
                {
                    self.set_error(err);
                }
            }
            State::Create(menu) => menu.tick(),
            State::EditMods(menu) => {
                let instance_selection = self.selected_instance.clone().unwrap();
                let update_locally_installed_mods = menu.tick(instance_selection);
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
            State::InstallJava(menu) => {
                menu.tick();
                if menu.progress.has_finished {
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
                match ModIndex::get(self.selected_instance.as_ref().unwrap()) {
                    Ok(index) => menu.mod_index = index,
                    Err(err) => err!("Can't load mod index: {err}"),
                }

                if let Some(results) = &menu.results {
                    let mut commands = Vec::new();
                    for result in &results.hits {
                        if commands.len() > 64 {
                            break;
                        }
                        if !self.images_downloads_in_progress.contains(&result.title)
                            && !result.icon_url.is_empty()
                        {
                            self.images_downloads_in_progress
                                .insert(result.title.clone());
                            commands.push(Command::perform(
                                Search::download_image(result.icon_url.clone(), true),
                                |n| Message::InstallMods(InstallModsMessage::ImageDownloaded(n)),
                            ));
                        }
                    }

                    if !commands.is_empty() {
                        return Command::batch(commands);
                    }
                }
            }
            State::LauncherSettings => {
                if let Some(config) = self.config.clone() {
                    return Command::perform(config.save_w(), Message::CoreTickConfigSaved);
                }
            }
            State::RedownloadAssets {
                progress,
                java_recv,
            } => {
                progress.tick();
                if progress.progress.has_finished {
                    let message = "Redownloaded Assets".to_owned();
                    let java_recv = java_recv.take();
                    self.state = State::Launch(MenuLaunch {
                        message,
                        java_recv,
                        asset_recv: None,
                    });
                    return Command::perform(
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
            State::ServerManage(menu) => {
                if menu
                    .java_install_recv
                    .as_ref()
                    .and_then(|n| n.try_recv().ok())
                    .is_some()
                {
                    self.state = State::InstallJava(ProgressBar::with_recv(
                        menu.java_install_recv.take().unwrap(),
                    ));
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
                        if *progress_number > 15.0 {
                            err!("More than 15 indexes scraped: {progress_number}");
                            *progress_number = 15.0;
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
            State::Error { .. }
            | State::ConfirmAction { .. }
            | State::ChangeLog
            | State::InstallPaper => {}
        }

        let mut commands = Vec::new();
        {
            let mut images_to_load = self.images_to_load.lock().unwrap();
            for url in images_to_load.iter() {
                if !self.images_downloads_in_progress.contains(url) {
                    self.images_downloads_in_progress.insert(url.to_owned());
                    commands.push(Command::perform(
                        Search::download_image(url.to_owned(), false),
                        |n| Message::InstallMods(InstallModsMessage::ImageDownloaded(n)),
                    ));
                }
            }
            images_to_load.clear();
        }

        if commands.is_empty() {
            Command::none()
        } else {
            Command::batch(commands)
        }
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
        ) => config.name.cmp(&config2.name),
        (ModListEntry::Downloaded { .. }, ModListEntry::Local { .. }) => std::cmp::Ordering::Less,
        (ModListEntry::Local { .. }, ModListEntry::Downloaded { .. }) => {
            std::cmp::Ordering::Greater
        }
        (
            ModListEntry::Local { file_name, .. },
            ModListEntry::Local {
                file_name: file_name2,
                ..
            },
        ) => file_name.cmp(file_name2),
    });

    entries
}

impl MenuEditMods {
    fn tick(&mut self, instance_selection: InstanceSelection) -> Command<Message> {
        self.sorted_mods_list = sort_dependencies(&self.mods.mods, &self.locally_installed_mods);

        if let Some(progress) = &mut self.mod_update_progress {
            progress.tick();
            if progress.progress.has_finished {
                self.mod_update_progress = None;
            }
        }

        MenuEditMods::update_locally_installed_mods(&self.mods, instance_selection)
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
                    if *progress_number > 21.0 {
                        err!("More than 20 indexes scraped: {}", *progress_number - 1.0);
                        *progress_number = 21.0;
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
