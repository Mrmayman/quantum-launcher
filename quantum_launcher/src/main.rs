use std::time::Duration;

use colored::Colorize;
use iced::{executor, widget, Application, Command, Settings};
use launcher_state::{
    Launcher, MenuDeleteInstance, MenuInstallFabric, MenuInstallForge, MenuLaunch,
    MenuLauncherUpdate, Message, State,
};

use message_handler::{format_memory, open_file_explorer};
use ql_instances::{error::LauncherError, info, UpdateCheckInfo, LAUNCHER_VERSION_NAME};
use ql_mod_manager::instance_mod_installer;
use stylesheet::styles::LauncherTheme;

mod config;
mod icon_manager;
mod launcher_state;
mod menu_renderer;
mod message_handler;
mod stylesheet;
mod tick;

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
            Command::perform(
                ql_instances::check_for_updates_wrapped(),
                Message::UpdateCheckResult,
            ),
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
            Message::DeleteInstanceMenu => {
                self.state = State::DeleteInstance(MenuDeleteInstance {})
            }
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
                if let Err(err) = self.go_to_edit_mods_menu() {
                    self.set_error(err.to_string())
                }
            }
            Message::InstallFabricScreenOpen => {
                self.state = State::InstallFabric(MenuInstallFabric {
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
                            self.selected_instance.to_owned().unwrap(),
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
            Message::Tick => return self.tick(),
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
                                self.selected_instance.to_owned().unwrap(),
                            ),
                            Message::UninstallLoaderEnd,
                        );
                    }
                    if menu.config.mod_type == "Forge" {
                        return Command::perform(
                            instance_mod_installer::forge::uninstall_wrapped(
                                self.selected_instance.to_owned().unwrap(),
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
                let (f_sender, f_receiver) = std::sync::mpsc::channel();
                let (j_sender, j_receiver) = std::sync::mpsc::channel();

                let command = Command::perform(
                    instance_mod_installer::forge::install_wrapped(
                        self.selected_instance.to_owned().unwrap(),
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
                Ok(_) => self.go_to_launch_screen_with_message("Installed Forge".to_owned()),
                Err(err) => self.set_error(err),
            },
            Message::LaunchEndedLog(result) => {
                match result {
                    Ok(status) => {
                        info!("Game exited with status: {status}");
                        if !status.success() {
                            if let State::Launch(MenuLaunch { message, .. }) = &mut self.state {
                                *message = format!("Game Crashed with code: {status}\nCheck Logs for more information");
                            }
                        }
                    }
                    Err(err) => self.set_error(err),
                }
            }
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
            Message::LaunchKillEnd(result) => {
                if let Err(err) = result {
                    self.set_error(err)
                }
            }
            Message::LaunchCopyLog => {
                if let Some(log) = self.logs.get(self.selected_instance.as_ref().unwrap()) {
                    return iced::clipboard::write(log.to_owned());
                }
            }
            Message::UpdateCheckResult(update_check_info) => match update_check_info {
                Ok(info) => match info {
                    UpdateCheckInfo::UpToDate => {
                        info!("Launcher is latest version. No new updates")
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
                    eprintln!("[error] Could not check for updates: {err}")
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
                self.selected_instance.as_ref(),
            ),
            State::EditInstance(menu) => menu.view(self.selected_instance.as_ref().unwrap()),
            State::EditMods(menu) => menu.view(self.selected_instance.as_ref().unwrap()),
            State::Create(menu) => menu.view(),
            State::DeleteInstance(menu) => menu.view(self.selected_instance.as_ref().unwrap()),
            State::Error { error } => widget::scrollable(
                widget::column!(
                    widget::text(format!("Error: {}", error)),
                    widget::button("Back").on_press(Message::LaunchScreenOpen(None)),
                    widget::button("Copy Error").on_press(Message::ErrorCopy),
                )
                .padding(10)
                .spacing(10),
            )
            .into(),
            State::InstallFabric(menu) => menu.view(self.selected_instance.as_ref().unwrap()),
            State::InstallForge(menu) => menu.view(),
            State::UpdateFound(menu) => menu.view(),
            State::InstallJava(menu) => menu.view(),
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

fn main() {
    let args = std::env::args();
    process_args(args);

    const WINDOW_HEIGHT: f32 = 450.0;
    const WINDOW_WIDTH: f32 = 650.0;

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

fn process_args(mut args: std::env::Args) -> Option<()> {
    let program = args.next()?;
    loop {
        let command = args.next()?;
        match command.as_str() {
            "--help" => {
                println!(
                    r#"Usage: {}
    --help    : Print a list of valid command line flags
    --version : Print the launcher version
"#,
                    format!("{program} [FLAGS]").yellow()
                )
            }
            "--version" => {
                println!("QuantumLauncher v{LAUNCHER_VERSION_NAME} - made by Mrmayman")
            }
            _ => {
                eprintln!(
                    "{} Unknown flag! Type {} to see all the command-line flags.",
                    "[error]".red(),
                    format!("{program} --help").yellow()
                )
            }
        }
    }
}
