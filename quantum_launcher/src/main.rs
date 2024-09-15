use std::time::Duration;

use iced::{executor, widget, Application, Command, Settings};
use launcher_state::{Launcher, MenuInstallFabric, MenuInstallForge, MenuLaunch, Message, State};
use message_handler::{format_memory, open_file_explorer};
use ql_instances::error::LauncherError;
use ql_instances::JavaInstallProgress;
use ql_mod_manager::instance_mod_installer;
use ql_mod_manager::instance_mod_installer::fabric::FabricInstallProgress;
use ql_mod_manager::instance_mod_installer::forge::ForgeInstallProgress;
use stylesheet::styles::LauncherTheme;

mod config;
mod icon_manager;
mod launcher_state;
mod menu_renderer;
mod message_handler;
mod stylesheet;

impl Application for Launcher {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = LauncherTheme;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, iced::Command<Self::Message>) {
        (
            match Launcher::new(None) {
                Ok(launcher) => launcher,
                Err(error) => Launcher::with_error(error.to_string()),
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        "Quantum Launcher".to_owned()
    }

    fn update(&mut self, message: Self::Message) -> iced::Command<Self::Message> {
        match message {
            Message::LaunchInstanceSelected(selected_instance) => {
                self.select_launch_instance(selected_instance)
            }
            Message::LaunchUsernameSet(username) => self.set_username(username),
            Message::LaunchStart => return self.launch_game(),
            Message::LaunchEnd(result) => {
                return self.finish_launching(result);
            }
            Message::CreateInstanceScreenOpen => return self.go_to_create_screen(),
            Message::CreateInstanceVersionsLoaded(result) => {
                self.create_instance_finish_loading_versions_list(result)
            }
            Message::CreateInstanceVersionSelected(selected_version) => {
                self.select_created_instance_version(selected_version)
            }
            Message::CreateInstanceNameInput(name) => self.update_created_instance_name(name),
            Message::CreateInstanceStart => return self.create_instance(),
            Message::CreateInstanceEnd(result) => match result {
                Ok(_) => match Launcher::new(Some("Created New Instance".to_owned())) {
                    Ok(launcher) => *self = launcher,
                    Err(err) => self.set_error(err.to_string()),
                },
                Err(n) => self.state = State::Error { error: n },
            },
            Message::DeleteInstanceMenu => self.confirm_instance_deletion(),
            Message::DeleteInstance => self.delete_selected_instance(),
            Message::LaunchScreenOpen(message) => {
                if let Some(message) = message {
                    self.go_to_launch_screen_with_message(message);
                } else {
                    self.go_to_launch_screen()
                }
            }
            Message::EditInstance => {
                self.edit_instance_wrapped();
            }
            Message::EditInstanceJavaOverride(n) => {
                if let State::EditInstance(menu_edit_instance) = &mut self.state {
                    menu_edit_instance.config.java_override = Some(n);
                }
            }
            Message::EditInstanceMemoryChanged(new_slider_value) => {
                if let State::EditInstance(menu_edit_instance) = &mut self.state {
                    menu_edit_instance.slider_value = new_slider_value;
                    menu_edit_instance.config.ram_in_mb = 2f32.powf(new_slider_value) as usize;
                    menu_edit_instance.slider_text =
                        format_memory(menu_edit_instance.config.ram_in_mb);
                }
            }
            Message::ManageModsScreenOpen => {
                if let State::Launch(menu_launch) = &self.state {
                    if let Err(err) =
                        self.go_to_edit_mods_menu(menu_launch.selected_instance.clone().unwrap())
                    {
                        self.set_error(err.to_string())
                    }
                }
            }
            Message::InstallFabricScreenOpen => {
                if let State::EditMods(menu) = &self.state {
                    self.state = State::InstallFabric(MenuInstallFabric {
                        selected_instance: menu.selected_instance.clone(),
                        fabric_version: None,
                        fabric_versions: Vec::new(),
                        progress_receiver: None,
                        progress_num: 0.0,
                    });

                    return Command::perform(
                        instance_mod_installer::fabric::get_list_of_versions(),
                        Message::InstallFabricVersionsLoaded,
                    );
                }
            }
            Message::InstallFabricVersionsLoaded(result) => match result {
                Ok(list_of_versions) => {
                    if let State::InstallFabric(menu) = &mut self.state {
                        menu.fabric_versions = list_of_versions
                            .iter()
                            .map(|ver| ver.version.clone())
                            .collect();
                    }
                }
                Err(err) => self.set_error(err),
            },
            Message::InstallFabricVersionSelected(selection) => {
                if let State::InstallFabric(menu) = &mut self.state {
                    menu.fabric_version = Some(selection);
                }
            }
            Message::InstallFabricClicked => {
                if let State::InstallFabric(menu) = &mut self.state {
                    let (sender, receiver) = std::sync::mpsc::channel();
                    menu.progress_receiver = Some(receiver);

                    return Command::perform(
                        instance_mod_installer::fabric::install_wrapped(
                            menu.fabric_version.clone().unwrap(),
                            menu.selected_instance.to_owned(),
                            Some(sender),
                        ),
                        Message::InstallFabricEnd,
                    );
                }
            }
            Message::InstallFabricEnd(result) => match result {
                Ok(_) => self.go_to_launch_screen_with_message("Installed Fabric".to_owned()),
                Err(err) => self.set_error(err),
            },
            Message::OpenDir(dir) => match dir.to_str() {
                Some(dir) => open_file_explorer(dir),
                None => self.set_error(LauncherError::PathBufToString(dir).to_string()),
            },
            Message::CreateInstanceChangeAssetToggle(toggle) => {
                if let State::Create(menu) = &mut self.state {
                    menu.download_assets = toggle;
                }
            }
            Message::ErrorCopy => {
                if let State::Error { error } = &self.state {
                    return iced::clipboard::write(format!("QuantumLauncher Error: {error}"));
                }
            }
            Message::Tick => match &mut self.state {
                State::Launch(MenuLaunch {
                    java_install_progress,
                    ..
                }) => {
                    let install_finished = receive_java_install_progress(java_install_progress);
                    if install_finished {
                        *java_install_progress = None;
                    }

                    let mut killed_processes = Vec::new();
                    for (name, process) in self.processes.iter() {
                        if let Ok(Some(_)) = process.child.lock().unwrap().try_wait() {
                            // Game process has exited.
                            killed_processes.push(name.to_owned())
                        } else {
                            if let Ok(message) = process.receiver.try_recv() {
                                if !self.logs.contains_key(name) {
                                    self.logs.insert(name.to_owned(), message);
                                } else {
                                    if let Some(log) = self.logs.get_mut(name) {
                                        log.push_str(&message);
                                    }
                                }
                            }
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
                            ForgeInstallProgress::P2DownloadingJson => {
                                "Downloading JSON".to_owned()
                            }
                            ForgeInstallProgress::P3DownloadingInstaller => {
                                "Downloading installer".to_owned()
                            }
                            ForgeInstallProgress::P4RunningInstaller => {
                                "Running Installer".to_owned()
                            }
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
            },
            Message::TickConfigSaved(result) => {
                if let Err(err) = result {
                    self.set_error(err)
                }
            }
            Message::UninstallLoaderStart => {
                if let State::EditMods(menu) = &self.state {
                    if menu.config.mod_type == "Fabric" {
                        return Command::perform(
                            instance_mod_installer::fabric::uninstall_wrapped(
                                menu.selected_instance.to_owned(),
                            ),
                            Message::UninstallLoaderEnd,
                        );
                    }
                    if menu.config.mod_type == "Forge" {
                        return Command::perform(
                            instance_mod_installer::forge::uninstall_wrapped(
                                menu.selected_instance.to_owned(),
                            ),
                            Message::UninstallLoaderEnd,
                        );
                    }
                }
            }
            Message::UninstallLoaderEnd(result) => {
                if let Err(err) = result {
                    self.set_error(err)
                } else {
                    self.go_to_launch_screen_with_message("Uninstalled Fabric".to_owned());
                }
            }
            Message::InstallForgeStart => {
                if let State::EditMods(menu) = &self.state {
                    let (f_sender, f_receiver) = std::sync::mpsc::channel();
                    let (j_sender, j_receiver) = std::sync::mpsc::channel();

                    let command = Command::perform(
                        instance_mod_installer::forge::install_wrapped(
                            menu.selected_instance.to_owned(),
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
            }
            Message::InstallForgeEnd(result) => match result {
                Ok(_) => self.go_to_launch_screen_with_message("Installed Forge".to_owned()),
                Err(err) => self.set_error(err),
            },
            Message::LaunchEndedLog(result) => match result {
                Ok(status) => {
                    println!("[info] Game exited with status: {status}")
                }
                Err(err) => self.set_error(err),
            },
            Message::LaunchKill => {
                if let State::Launch(MenuLaunch {
                    selected_instance: Some(selected_instance),
                    ..
                }) = &self.state
                {
                    if let Some(process) = self.processes.remove(selected_instance) {
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
            }
            Message::LaunchKillEnd(result) => {
                if let Err(err) = result {
                    self.set_error(err)
                }
            }
        }
        Command::none()
    }

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        const UPDATES_PER_SECOND: u64 = 12;

        iced::time::every(Duration::from_millis(1000 / UPDATES_PER_SECOND)).map(|_| Message::Tick)
    }

    fn view(&self) -> iced::Element<'_, Self::Message, Self::Theme, iced::Renderer> {
        match &self.state {
            State::Launch(menu) => menu.view(
                self.config.as_ref(),
                self.instances.as_deref(),
                &self.processes,
                &self.logs,
            ),
            State::EditInstance(menu) => menu.view(),
            State::EditMods(menu) => menu.view(),
            State::Create(menu) => menu.view(),
            State::DeleteInstance(menu) => menu.view(),
            State::Error { error } => widget::column!(
                widget::text(format!("Error: {}", error)),
                widget::button("Back").on_press(Message::LaunchScreenOpen(None)),
                widget::button("Copy Error").on_press(Message::ErrorCopy),
            )
            .into(),
            State::InstallFabric(menu) => menu.view(),
            State::InstallForge(menu) => menu.view(),
        }
    }
}

fn receive_java_install_progress(
    java_install_progress: &mut Option<launcher_state::JavaInstallProgressData>,
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

fn main() {
    const WINDOW_HEIGHT: f32 = 450.0;
    const WINDOW_WIDTH: f32 = 400.0;

    // let rt = tokio::runtime::Runtime::new().unwrap();
    // rt.block_on(ql_mod_manager::instance_mod_installer::forge::install(
    //     "1.20.1 fresh test",
    // ))
    // .unwrap();

    // return;

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
        ],
        default_font: iced::Font::with_name("Inter"),
        ..Default::default()
    })
    .unwrap();
}
