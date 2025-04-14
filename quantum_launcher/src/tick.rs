use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

use chrono::Datelike;
use iced::Task;
use ql_core::{
    err, json::InstanceConfigJson, InstanceSelection, IntoIoError, IntoStringError, JsonFileError,
    ModId,
};
use ql_mod_manager::mod_manager::{ModConfig, ModIndex};

use crate::launcher_state::{
    EditInstanceMessage, ImageState, InstallModsMessage, InstanceLog, LaunchTabId, Launcher,
    MenuCreateInstance, MenuEditMods, MenuEditPresetsInner, MenuInstallFabric, MenuLaunch,
    MenuModsDownload, MenuServerCreate, Message, ModListEntry, ProgressBar, ServerProcess, State,
};

impl Launcher {
    pub fn tick(&mut self) -> Task<Message> {
        match &mut self.state {
            State::Launch(MenuLaunch {
                asset_recv,
                edit_instance,
                tab,
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

                if let (Some(edit), LaunchTabId::Edit) = (&edit_instance, tab) {
                    let config = edit.config.clone();
                    self.tick_edit_instance(config, &mut commands);
                }
                self.tick_client_processes_and_logs();
                self.tick_server_processes_and_logs();

                let launcher_config = self.config.clone();
                commands.push(Task::perform(
                    async move { launcher_config.save().await.strerr() },
                    Message::CoreTickConfigSaved,
                ));
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
                    return self.go_to_main_menu_with_message(Some("Installed Java"));
                }
            }
            State::ModsDownload(menu) => {
                return menu.tick(self.selected_instance.clone().unwrap(), &mut self.images)
            }
            State::LauncherSettings => {
                let launcher_config = self.config.clone();
                return Task::perform(
                    async move { launcher_config.save().await.strerr() },
                    Message::CoreTickConfigSaved,
                );
            }
            State::RedownloadAssets { progress } => {
                progress.tick();
                if progress.progress.has_finished {
                    return self.go_to_launch_screen(Some("Redownloaded Assets"));
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
            // State::ServerManage(_) => {
            //     if self.java_recv.as_mut().is_some_and(ProgressBar::tick) {
            //         self.state = State::InstallJava;
            //         return Task::none();
            //     }
            //     self.tick_server_processes_and_logs();
            // }
            State::ServerCreate(menu) => menu.tick(),
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

    fn tick_edit_instance(&self, config: InstanceConfigJson, commands: &mut Vec<Task<Message>>) {
        let Some(instance) = self.selected_instance.clone() else {
            return;
        };
        let dir = self.dir.clone();
        let cmd = Task::perform(Launcher::save_config(instance, config, dir), |n| {
            Message::EditInstance(EditInstanceMessage::ConfigSaved(n.strerr()))
        });
        commands.push(cmd);
    }

    pub fn get_imgs_to_load(&mut self) -> Vec<Task<Message>> {
        let mut commands = Vec::new();

        let mut images_to_load = self.images.to_load.lock().unwrap();

        for url in images_to_load.iter() {
            if !self.images.downloads_in_progress.contains(url) {
                self.images.downloads_in_progress.insert(url.to_owned());
                commands.push(Task::perform(
                    ql_mod_manager::mod_manager::download_image(url.to_owned(), false),
                    |n| Message::InstallMods(InstallModsMessage::ImageDownloaded(n)),
                ));
            }
        }

        images_to_load.clear();
        commands
    }

    fn tick_client_processes_and_logs(&mut self) {
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
            let mut log_start = vec![
                format!(
                    "Starting Minecraft Server ({})",
                    Self::get_current_date_formatted()
                ),
                format!("OS: {}\n", ql_instances::OS_NAME),
            ];

            if let Some(log) = server_logs.get_mut(name) {
                if log.log.is_empty() {
                    log.log = log_start;
                }
                log.log.push(message);
            } else {
                log_start.push(message);

                server_logs.insert(
                    name.to_owned(),
                    InstanceLog {
                        log: log_start,
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
            let message = message.to_string();

            let mut log_start = vec![
                format!(
                    "Launching Minecraft ({})",
                    Self::get_current_date_formatted()
                ),
                format!("OS: {}\n", ql_instances::OS_NAME),
            ];

            if !logs.contains_key(name) {
                log_start.push(message);

                logs.insert(
                    name.to_owned(),
                    InstanceLog {
                        log: log_start,
                        has_crashed: false,
                        command: String::new(),
                    },
                );
            } else if let Some(log) = logs.get_mut(name) {
                if log.log.is_empty() {
                    log.log = log_start;
                }
                log.log.push(message);
            }
        }
    }

    fn get_current_date_formatted() -> String {
        // Get the current date and time in UTC
        let now = chrono::Local::now();

        // Extract the day, month, and year
        let day = now.day();
        let month = now.format("%B").to_string(); // Full month name (e.g., "September")
        let year = now.year();

        // Return the formatted string
        format!("{day} {month} {year}")
    }

    async fn save_config(
        instance: InstanceSelection,
        config: InstanceConfigJson,
        dir: PathBuf,
    ) -> Result<(), JsonFileError> {
        let mut config = config.clone();
        if config.enable_logger.is_none() {
            config.enable_logger = Some(true);
        }
        let config_path = instance.get_instance_path(&dir).join("config.json");

        let config_json = serde_json::to_string(&config)?;
        tokio::fs::write(&config_path, config_json)
            .await
            .path(config_path)?;
        Ok(())
    }
}

impl MenuModsDownload {
    pub fn tick(
        &mut self,
        selected_instance: InstanceSelection,
        images: &mut ImageState,
    ) -> Task<Message> {
        let index_cmd = Task::perform(
            async move { ModIndex::get(&selected_instance).await },
            |n| Message::InstallMods(InstallModsMessage::IndexUpdated(n.strerr())),
        );

        if let Some(results) = &self.results {
            let mut commands = vec![index_cmd];
            for result in &results.mods {
                if commands.len() > 64 {
                    break;
                }
                if !images.downloads_in_progress.contains(&result.title)
                    && !result.icon_url.is_empty()
                {
                    images.downloads_in_progress.insert(result.title.clone());
                    commands.push(Task::perform(
                        ql_mod_manager::mod_manager::download_image(result.icon_url.clone(), true),
                        |n| Message::InstallMods(InstallModsMessage::ImageDownloaded(n)),
                    ));
                }
            }

            Task::batch(commands)
        } else {
            index_cmd
        }
    }
}

impl MenuServerCreate {
    pub fn tick(&mut self) {
        match self {
            MenuServerCreate::LoadingList {
                progress_receiver,
                progress_number,
            } => {
                while let Ok(()) = progress_receiver.try_recv() {
                    *progress_number += 1.0;
                    if *progress_number > 18.0 {
                        err!("More than 18 indexes scraped: {progress_number}");
                        *progress_number = 18.0;
                    }
                }
            }
            MenuServerCreate::Loaded { .. } => {}
            MenuServerCreate::Downloading { progress } => {
                progress.tick();
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
            id: ModId::from_index_str(k),
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
                receiver, number, ..
            } => {
                while let Ok(()) = receiver.try_recv() {
                    *number += 1.0;
                    if *number > 26.0 {
                        err!("More than 26 indexes scraped: {}", *number);
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
