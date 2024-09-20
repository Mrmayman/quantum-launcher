use std::collections::HashMap;

use iced::Command;
use ql_instances::{JavaInstallProgress, LogEvent, LogLine, UpdateProgress};
use ql_mod_manager::instance_mod_installer::{
    fabric::FabricInstallProgress, forge::ForgeInstallProgress,
};

use crate::launcher_state::{
    JavaInstallProgressData, Launcher, MenuLaunch, MenuLauncherUpdate, Message, State,
};

impl Launcher {
    pub fn tick(&mut self) -> Option<Command<Message>> {
        match &mut self.state {
            State::Launch(MenuLaunch {
                java_install_progress,
                ..
            }) => {
                check_java_install_progress(java_install_progress);

                let mut killed_processes = Vec::new();
                for (name, process) in self.processes.iter() {
                    if let Ok(Some(_)) = process.child.lock().unwrap().try_wait() {
                        // Game process has exited.
                        killed_processes.push(name.to_owned())
                    } else {
                        Launcher::read_game_logs(&mut self.logs, process, name);
                    }
                }
                for name in killed_processes {
                    self.processes.remove(&name);
                }

                if let Some(config) = self.config.clone() {
                    return Some(Command::perform(
                        config.save_wrapped(),
                        Message::TickConfigSaved,
                    ));
                }
            }
            State::EditInstance(menu) => {
                if let Err(err) = Launcher::save_config(&menu.selected_instance, &menu.config) {
                    self.set_error(err.to_string())
                }
            }
            State::Create(menu) => Launcher::update_instance_creation_progress_bar(menu),
            State::EditMods(_) => {}
            State::Error { .. } => {}
            State::DeleteInstance(_) => {}
            State::InstallFabric(menu) => {
                if let Some(receiver) = &menu.progress_receiver {
                    if let Ok(progress) = receiver.try_recv() {
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
                if let Ok(message) = menu.forge_progress_receiver.try_recv() {
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

                if let Ok(message) = menu.java_progress_receiver.try_recv() {
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
                if let Some(Ok(message)) = receiver.as_ref().map(|n| n.try_recv()) {
                    match message {
                        UpdateProgress::P1Start => {}
                        UpdateProgress::P2Backup => {
                            *progress = 1.0;
                            *progress_message = Some("Backing up current version".to_owned())
                        }
                        UpdateProgress::P3Download => {
                            *progress = 2.0;
                            *progress_message = Some("Downloading new version".to_owned())
                        }
                        UpdateProgress::P4Extract => {
                            *progress = 3.0;
                            *progress_message = Some("Extracting new version".to_owned())
                        }
                    }
                }
            }
        }
        None
    }

    fn read_game_logs(
        logs: &mut HashMap<String, String>,
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
                logs.insert(name.to_owned(), message);
            } else if let Some(log) = logs.get_mut(name) {
                log.push_str(&message);
            }
        }
    }
}

fn get_date(timestamp: &str) -> Option<String> {
    let time: i64 = timestamp.parse().ok()?;
    let seconds = time / 1000;
    let milliseconds = time % 1000;
    let nanoseconds = milliseconds * 1000000;
    let datetime = chrono::DateTime::from_timestamp(seconds, nanoseconds as u32)?;
    Some(datetime.format("%Y-%m-%d %H:%M:%S").to_string())
}

fn check_java_install_progress(java_install_progress: &mut Option<JavaInstallProgressData>) {
    let install_finished = receive_java_install_progress(java_install_progress);
    if install_finished {
        *java_install_progress = None;
    }
}

fn receive_java_install_progress(
    java_install_progress: &mut Option<JavaInstallProgressData>,
) -> bool {
    let Some(java_install_progress) = java_install_progress else {
        return true;
    };

    if let Ok(message) = java_install_progress.recv.try_recv() {
        match message {
            JavaInstallProgress::P1Started => {
                java_install_progress.num = 0.0;
                java_install_progress.message = "Starting up (2/2)".to_owned();
            }
            JavaInstallProgress::P2 {
                progress,
                out_of,
                name,
            } => {
                java_install_progress.num = (progress as f32) / (out_of as f32);
                java_install_progress.message =
                    format!("Downloading ({progress}/{out_of}): {name}");
            }
            JavaInstallProgress::P3Done => {
                java_install_progress.num = 1.0;
                java_install_progress.message = "Done!".to_owned();
                return true;
            }
        }
    }
    false
}
