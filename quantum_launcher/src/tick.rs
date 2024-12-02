use std::collections::HashMap;

use iced::Command;
use ql_instances::{
    err, info, AssetRedownloadProgress, JavaInstallProgress, LogEvent, LogLine, UpdateProgress,
};
use ql_mod_manager::{
    instance_mod_installer::{fabric::FabricInstallProgress, forge::ForgeInstallProgress},
    mod_manager::{ModConfig, ModIndex, Search},
};

use crate::launcher_state::{
    reload_instances, InstanceLog, Launcher, MenuInstallJava, MenuLaunch, MenuLauncherUpdate,
    MenuRedownloadAssets, Message, State,
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
                        info!("Started install of Java");
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

                let mut killed_processes = Vec::new();
                for (name, process) in &self.processes {
                    if let Ok(Some(_)) = process.child.lock().unwrap().try_wait() {
                        // Game process has exited.
                        killed_processes.push(name.to_owned());
                    } else {
                        Launcher::read_game_logs(&mut self.logs, process, name);
                    }
                }
                for name in killed_processes {
                    self.processes.remove(&name);
                }

                if let Some(config) = self.config.clone() {
                    return Command::perform(config.save_wrapped(), Message::TickConfigSaved);
                }
            }
            State::EditInstance(menu) => {
                if let Err(err) =
                    Launcher::save_config(self.selected_instance.as_ref().unwrap(), &menu.config)
                {
                    self.set_error(err.to_string());
                }
            }
            State::Create(menu) => Launcher::update_instance_creation_progress_bar(menu),
            State::EditMods(menu) => {
                menu.sorted_dependencies = sort_dependencies(&menu.mods.mods);
            }
            State::Error { .. } => {}
            State::DeleteInstance(_) => {}
            State::InstallFabric(menu) => {
                if let Some(receiver) = &menu.progress_receiver {
                    while let Ok(progress) = receiver.try_recv() {
                        menu.progress_num = match progress {
                            FabricInstallProgress::P1Start => 0.0,
                            FabricInstallProgress::P2Library { done, out_of } => {
                                done as f32 / out_of as f32
                            }
                            FabricInstallProgress::P3Done => 1.0,
                        }
                    }
                }
            }
            State::InstallForge(menu) => {
                while let Ok(message) = menu.forge_progress_receiver.try_recv() {
                    menu.forge_progress_num = match message {
                        ForgeInstallProgress::P1Start => 0.0,
                        ForgeInstallProgress::P2DownloadingJson => 1.0,
                        ForgeInstallProgress::P3DownloadingInstaller => 2.0,
                        ForgeInstallProgress::P4RunningInstaller => 3.0,
                        ForgeInstallProgress::P5DownloadingLibrary { num, out_of } => {
                            3.0 + (num as f32 / out_of as f32)
                        }
                        ForgeInstallProgress::P6Done => 4.0,
                    };

                    menu.forge_message = match message {
                        ForgeInstallProgress::P1Start => "Installing forge...".to_owned(),
                        ForgeInstallProgress::P2DownloadingJson => "Downloading JSON".to_owned(),
                        ForgeInstallProgress::P3DownloadingInstaller => {
                            "Downloading installer".to_owned()
                        }
                        ForgeInstallProgress::P4RunningInstaller => "Running Installer".to_owned(),
                        ForgeInstallProgress::P5DownloadingLibrary { num, out_of } => {
                            format!("Downloading Library ({num}/{out_of})")
                        }
                        ForgeInstallProgress::P6Done => "Done!".to_owned(),
                    };
                }

                while let Ok(message) = menu.java_progress_receiver.try_recv() {
                    match message {
                        JavaInstallProgress::P1Started => {
                            menu.is_java_getting_installed = true;
                            menu.java_progress_num = 0.0;
                            menu.java_message = Some("Started...".to_owned());
                        }
                        JavaInstallProgress::P2 {
                            progress,
                            out_of,
                            name,
                        } => {
                            menu.java_progress_num = progress as f32 / out_of as f32;
                            menu.java_message =
                                Some(format!("Downloading ({progress}/{out_of}): {name}"));
                        }
                        JavaInstallProgress::P3Done => {
                            menu.is_java_getting_installed = false;
                            menu.java_message = None;
                        }
                    }
                }
            }
            State::UpdateFound(MenuLauncherUpdate {
                receiver,
                progress,
                progress_message,
                ..
            }) => {
                while let Some(Ok(message)) =
                    receiver.as_ref().map(std::sync::mpsc::Receiver::try_recv)
                {
                    match message {
                        UpdateProgress::P1Start => {}
                        UpdateProgress::P2Backup => {
                            *progress = 1.0;
                            *progress_message = Some("Backing up current version".to_owned());
                        }
                        UpdateProgress::P3Download => {
                            *progress = 2.0;
                            *progress_message = Some("Downloading new version".to_owned());
                        }
                        UpdateProgress::P4Extract => {
                            *progress = 3.0;
                            *progress_message = Some("Extracting new version".to_owned());
                        }
                    }
                }
            }
            State::InstallJava(menu) => {
                let finished_install = menu.tick();
                if finished_install {
                    let message = "Installed Java".to_owned();
                    self.state = State::Launch(MenuLaunch::with_message(message));
                    if let Ok(list) = reload_instances() {
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

                        if commands.len() > 10 {
                            break;
                        }
                    }

                    if !commands.is_empty() {
                        return Command::batch(commands);
                    }
                }
            }
            State::LauncherSettings => {
                if let Some(config) = self.config.clone() {
                    return Command::perform(config.save_wrapped(), Message::TickConfigSaved);
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
                    if let Ok(list) = reload_instances() {
                        self.instances = Some(list);
                    } else {
                        err!("Failed to reload instances list.");
                    }
                }
            }
            State::InstallOptifine(_) => {}
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

    fn read_game_logs(
        logs: &mut HashMap<String, InstanceLog>,
        process: &crate::launcher_state::GameProcess,
        name: &String,
    ) {
        while let Ok(message) = process.receiver.try_recv() {
            let message = match message {
                LogLine::Info(LogEvent {
                    logger,
                    timestamp,
                    level,
                    thread,
                    message,
                }) => {
                    let date = get_date(&timestamp).unwrap_or(timestamp);
                    format!("[{date}:{thread}.{logger}] [{level}] {}\n", message.content)
                }
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

fn get_date(timestamp: &str) -> Option<String> {
    let time: i64 = timestamp.parse().ok()?;
    let seconds = time / 1000;
    let milliseconds = time % 1000;
    let nanoseconds = milliseconds * 1_000_000;
    let datetime = chrono::DateTime::from_timestamp(seconds, nanoseconds as u32)?;
    let datetime = datetime.with_timezone(&chrono::Local);
    Some(datetime.format("%H:%M:%S").to_string())
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
                    progress,
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

pub fn sort_dependencies(map: &HashMap<String, ModConfig>) -> Vec<(String, ModConfig)> {
    let mut entries: Vec<(String, ModConfig)> =
        map.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
    entries.sort_by(|(_, val1), (_, val2)| {
        // First, sort based on the custom condition
        let cond1 = val1.manually_installed;
        let cond2 = val2.manually_installed;

        match (cond1, cond2) {
            // If both are true or both are false, fall back to alphabetical sorting
            (true, true) | (false, false) => val1.name.cmp(&val2.name),
            // If only cond1 is true, it should come first (higher priority)
            (true, false) => std::cmp::Ordering::Less,
            // If only cond2 is true, it should come first
            (false, true) => std::cmp::Ordering::Greater,
        }
    });

    entries
}
