use std::collections::{HashMap, HashSet};

use iced::Command;
use ql_core::{err, info, InstanceSelection, JavaInstallProgress};
use ql_instances::{AssetRedownloadProgress, LogLine, ScrapeProgress, UpdateProgress};
use ql_mod_manager::{
    instance_mod_installer::{
        fabric::FabricInstallProgress, forge::ForgeInstallProgress,
        optifine::OptifineInstallProgress,
    },
    mod_manager::{ApplyUpdateProgress, ModConfig, ModIndex, Search},
};
use ql_servers::ServerCreateProgress;

use crate::launcher_state::{
    get_entries, InstanceLog, Launcher, MenuCreateInstance, MenuEditMods, MenuInstallFabric,
    MenuInstallForge, MenuInstallJava, MenuLaunch, MenuLauncherUpdate, MenuRedownloadAssets,
    MenuServerCreate, Message, ModListEntry, ServerProcess, State,
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
                    if let Ok(JavaInstallProgress::P1Started) = receiver.try_recv() {
                        info!("Installing Java");
                        self.state = State::InstallJava(MenuInstallJava {
                            num: 0.0,
                            recv: receiver,
                            message: "Starting...".to_owned(),
                        });
                        return Command::none();
                    }
                    *java_recv = Some(receiver);
                }

                if let Some(receiver) = asset_recv.take() {
                    if let Ok(AssetRedownloadProgress::P1Start) = receiver.try_recv() {
                        self.state = State::RedownloadAssets(MenuRedownloadAssets {
                            num: 0.0,
                            recv: receiver,
                            java_recv: java_recv.take(),
                        });
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
                menu.sorted_mods_list =
                    sort_dependencies(&menu.mods.mods, &menu.locally_installed_mods);

                let has_finished = menu.tick_mod_update_progress();
                if has_finished {
                    menu.mod_update_progress = None;
                }

                let instance_selection = self.selected_instance.clone().unwrap();
                return MenuEditMods::update_locally_installed_mods(&menu.mods, instance_selection);
            }
            State::Error { .. } | State::DeleteInstance => {}
            State::InstallFabric(menu) => menu.tick(),
            State::InstallForge(menu) => menu.tick(),
            State::UpdateFound(menu) => menu.tick(),
            State::InstallJava(menu) => {
                let finished_install = menu.tick();
                if finished_install {
                    let message = "Installed Java".to_owned();
                    match &self.selected_instance {
                        Some(InstanceSelection::Instance(_)) | None => {
                            self.state = State::Launch(MenuLaunch::with_message(message));
                        }
                        Some(InstanceSelection::Server(_)) => {
                            self.go_to_server_manage_menu(Some(message))
                        }
                    }
                    if let Ok(list) = get_entries("instances") {
                        self.instances = Some(list);
                    } else {
                        err!("Failed to reload instances list.");
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
                                Message::InstallModsImageDownloaded,
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
            State::RedownloadAssets(menu) => {
                let finished = menu.tick();
                if finished {
                    let message = "Redownloaded Assets".to_owned();
                    let java_recv = menu.java_recv.take();
                    self.state = State::Launch(MenuLaunch {
                        message,
                        java_recv,
                        asset_recv: None,
                    });
                    if let Ok(list) = get_entries("instances") {
                        self.instances = Some(list);
                    } else {
                        err!("Failed to reload instances list.");
                    }
                }
            }
            State::InstallOptifine(menu) => {
                if let Some(progress) = &mut menu.progress {
                    while let Ok(message) = progress.optifine_install_progress.try_recv() {
                        match message {
                            OptifineInstallProgress::P1Start => {
                                progress.optifine_install_num = 0.0;
                                "Starting...".clone_into(&mut progress.optifine_install_message);
                            }
                            OptifineInstallProgress::P2CompilingHook => {
                                progress.optifine_install_num = 1.0;
                                "Compiling hook..."
                                    .clone_into(&mut progress.optifine_install_message);
                            }
                            OptifineInstallProgress::P3RunningHook => {
                                progress.optifine_install_num = 2.0;
                                "Running hook..."
                                    .clone_into(&mut progress.optifine_install_message);
                            }
                            OptifineInstallProgress::P4DownloadingLibraries { done, total } => {
                                progress.optifine_install_num = 2.0 + (done as f32 / total as f32);
                                progress.optifine_install_message =
                                    format!("Downloading libraries ({done}/{total})");
                            }
                            OptifineInstallProgress::P5Done => {
                                progress.optifine_install_num = 3.0;
                                "Done!".clone_into(&mut progress.optifine_install_message);
                            }
                        }
                    }

                    while let Ok(message) = progress.java_install_progress.try_recv() {
                        match message {
                            JavaInstallProgress::P1Started => {
                                progress.java_install_num = 0.0;
                                "Starting...".clone_into(&mut progress.java_install_message);
                                progress.is_java_being_installed = true;
                            }
                            JavaInstallProgress::P2 { done, out_of, name } => {
                                progress.java_install_num = done as f32 / out_of as f32;
                                progress.java_install_message =
                                    format!("Downloading ({done}/{out_of}): {name}");
                            }
                            JavaInstallProgress::P3Done => {
                                progress.java_install_num = 1.0;
                                "Done!".clone_into(&mut progress.java_install_message);
                                progress.is_java_being_installed = false;
                            }
                        }
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
                    self.state = State::InstallJava(MenuInstallJava {
                        num: 0.0,
                        recv: menu.java_install_recv.take().unwrap(),
                        message: String::new(),
                    })
                }

                self.tick_server_processes_and_logs();
            }
            State::ServerCreate(menu) => match menu {
                MenuServerCreate::Loading {
                    progress_receiver,
                    progress_number,
                } => {
                    while let Ok(progress) = progress_receiver.try_recv() {
                        if let ScrapeProgress::ScrapedFile = progress {
                            *progress_number += 1.0;
                        }
                        if *progress_number > 15.0 {
                            err!("More than 15 indexes scraped: {progress_number}");
                            *progress_number = 15.0;
                        }
                    }
                }
                MenuServerCreate::Loaded {
                    progress_receiver,
                    progress_number,
                    ..
                } => {
                    while let Some(progress) =
                        progress_receiver.as_ref().and_then(|n| n.try_recv().ok())
                    {
                        *progress_number = match progress {
                            ServerCreateProgress::P1DownloadingManifest => 0.0,
                            ServerCreateProgress::P2DownloadingVersionJson => 1.0,
                            ServerCreateProgress::P3DownloadingServerJar => 2.0,
                        };
                    }
                }
            },
            State::ServerDelete { .. } => {}
        }

        let mut commands = Vec::new();
        {
            let mut images_to_load = self.images_to_load.lock().unwrap();
            for url in images_to_load.iter() {
                if !self.images_downloads_in_progress.contains(url) {
                    self.images_downloads_in_progress.insert(url.to_owned());
                    commands.push(Command::perform(
                        Search::download_image(url.to_owned(), false),
                        Message::InstallModsImageDownloaded,
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

impl MenuLauncherUpdate {
    fn tick(&mut self) {
        while let Some(Ok(message)) = self
            .receiver
            .as_ref()
            .map(std::sync::mpsc::Receiver::try_recv)
        {
            match message {
                UpdateProgress::P1Start => {}
                UpdateProgress::P2Backup => {
                    self.progress = 1.0;
                    self.progress_message = Some("Backing up current version".to_owned());
                }
                UpdateProgress::P3Download => {
                    self.progress = 2.0;
                    self.progress_message = Some("Downloading new version".to_owned());
                }
                UpdateProgress::P4Extract => {
                    self.progress = 3.0;
                    self.progress_message = Some("Extracting new version".to_owned());
                }
            }
        }
    }
}

impl MenuInstallForge {
    fn tick(&mut self) {
        while let Ok(message) = self.forge_progress_receiver.try_recv() {
            self.forge_progress_num = match message {
                ForgeInstallProgress::P1Start => 0.0,
                ForgeInstallProgress::P2DownloadingJson => 1.0,
                ForgeInstallProgress::P3DownloadingInstaller => 2.0,
                ForgeInstallProgress::P4RunningInstaller => 3.0,
                ForgeInstallProgress::P5DownloadingLibrary { num, out_of } => {
                    3.0 + (num as f32 / out_of as f32)
                }
                ForgeInstallProgress::P6Done => 4.0,
            };

            self.forge_message = match message {
                ForgeInstallProgress::P1Start => "Installing forge...".to_owned(),
                ForgeInstallProgress::P2DownloadingJson => "Downloading JSON".to_owned(),
                ForgeInstallProgress::P3DownloadingInstaller => "Downloading installer".to_owned(),
                ForgeInstallProgress::P4RunningInstaller => "Running Installer".to_owned(),
                ForgeInstallProgress::P5DownloadingLibrary { num, out_of } => {
                    format!("Downloading Library ({num}/{out_of})")
                }
                ForgeInstallProgress::P6Done => "Done!".to_owned(),
            };
        }
        while let Ok(message) = self.java_progress_receiver.try_recv() {
            match message {
                JavaInstallProgress::P1Started => {
                    self.is_java_getting_installed = true;
                    self.java_progress_num = 0.0;
                    self.java_message = Some("Started...".to_owned());
                }
                JavaInstallProgress::P2 {
                    done: progress,
                    out_of,
                    name,
                } => {
                    self.java_progress_num = progress as f32 / out_of as f32;
                    self.java_message = Some(format!("Downloading ({progress}/{out_of}): {name}"));
                }
                JavaInstallProgress::P3Done => {
                    self.is_java_getting_installed = false;
                    self.java_message = None;
                }
            }
        }
    }
}

impl MenuInstallFabric {
    fn tick(&mut self) {
        if let Self::Loaded {
            progress_receiver: Some(receiver),
            progress_num,
            progress_message,
            ..
        } = self
        {
            while let Ok(progress) = receiver.try_recv() {
                *progress_num = match progress {
                    FabricInstallProgress::P1Start => 0.0,
                    FabricInstallProgress::P2Library {
                        done,
                        out_of,
                        message,
                    } => {
                        *progress_message = message;
                        done as f32 / out_of as f32
                    }
                    FabricInstallProgress::P3Done => 1.0,
                }
            }
        }
    }
}

impl MenuInstallJava {
    /// Returns true if Java installation has finished.
    pub fn tick(&mut self) -> bool {
        while let Ok(message) = self.recv.try_recv() {
            match message {
                JavaInstallProgress::P1Started => {
                    self.num = 0.0;
                    "Starting up (2/2)".clone_into(&mut self.message);
                }
                JavaInstallProgress::P2 {
                    done: progress,
                    out_of,
                    name,
                } => {
                    self.num = (progress as f32) / (out_of as f32);
                    self.message = format!("Downloading ({progress}/{out_of}): {name}");
                }
                JavaInstallProgress::P3Done => {
                    return true;
                }
            }
        }
        false
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
    pub fn tick_mod_update_progress(&mut self) -> bool {
        if let Some(progress) = &mut self.mod_update_progress {
            while let Ok(message) = progress.recv.try_recv() {
                match message {
                    ApplyUpdateProgress::P1DeleteMods => {
                        progress.num = 0.0;
                        "Deleting old versions".clone_into(&mut progress.message);
                    }
                    ApplyUpdateProgress::P2DownloadMod { done, out_of } => {
                        progress.num = 0.2 + (done as f32 / out_of as f32) * 0.8;
                        progress.message = format!("Downloading mods ({done}/{out_of})");
                    }
                    ApplyUpdateProgress::P3Done => return true,
                }
            }
        }
        false
    }
}

impl MenuCreateInstance {
    pub fn tick(&mut self) {
        match self {
            MenuCreateInstance::Loading {
                progress_receiver,
                progress_number,
            } => {
                while let Ok(progress) = progress_receiver.try_recv() {
                    if let ScrapeProgress::ScrapedFile = progress {
                        *progress_number += 1.0;
                    }
                    if *progress_number > 21.0 {
                        err!("More than 20 indexes scraped: {progress_number}");
                        *progress_number = 21.0;
                    }
                }
            }
            MenuCreateInstance::Loaded {
                progress_receiver: Some(receiver),
                progress_number,
                progress_text,
                ..
            } => {
                while let Ok(progress) = receiver.try_recv() {
                    if let Some(progress_text) = progress_text {
                        *progress_text = progress.to_string();
                    }
                    if let Some(progress_num) = progress_number {
                        *progress_num = progress.into();
                    }
                }
            }
            _ => {}
        }
    }
}
