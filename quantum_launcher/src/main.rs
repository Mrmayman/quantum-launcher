use std::time::Duration;

use iced::{executor, widget, Application, Command, Settings, Subscription};
use launcher_state::{Launcher, MenuInstallFabric, MenuLaunch, Message, State};
use message_handler::{format_memory, open_file_explorer};
use quantum_launcher_backend::{error::LauncherError, instance_mod_installer};
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
            match Launcher::new() {
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
            Message::LaunchEnd(result) => self.finish_launching(result),
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
                Ok(_) => match Launcher::new() {
                    Ok(launcher) => *self = launcher,
                    Err(err) => self.set_error(err.to_string()),
                },
                Err(n) => self.state = State::Error { error: n },
            },
            Message::CreateInstanceProgressUpdate => self.update_instance_creation_progress_bar(),
            Message::DeleteInstanceMenu => self.confirm_instance_deletion(),
            Message::DeleteInstance => self.delete_selected_instance(),
            Message::LaunchScreenOpen => self.go_to_launch_screen(),
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
            Message::EditInstanceSave => {
                if let State::EditInstance(menu_edit_instance) = &self.state {
                    if let Err(err) = Launcher::save_config(
                        &menu_edit_instance.selected_instance,
                        &menu_edit_instance.config,
                    ) {
                        self.set_error(err.to_string())
                    }
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
                if let State::InstallFabric(menu) = &self.state {
                    return Command::perform(
                        instance_mod_installer::fabric::install_wrapped(
                            menu.fabric_version.clone().unwrap(),
                            menu.selected_instance.to_owned(),
                        ),
                        Message::InstallFabricEnd,
                    );
                }
            }
            Message::InstallFabricEnd(result) => match result {
                Ok(_) => self.go_to_launch_screen(),
                Err(err) => self.set_error(err),
            },
            Message::OpenDir(dir) => match dir.to_str() {
                Some(dir) => open_file_explorer(dir),
                None => self.set_error(LauncherError::PathBufToString(dir).to_string()),
            },
            Message::LaunchJavaInstallProgressUpdate => {
                if let State::Launch(MenuLaunch {
                    java_install_progress,
                    ..
                }) = &mut self.state
                {
                    let install_finished = receive_java_install_progress(java_install_progress);

                    if install_finished {
                        *java_install_progress = None;
                    }
                }
            }
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
        }
        Command::none()
    }

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        const UPDATES_PER_SECOND: u64 = 15;

        if let State::Create(menu) = &self.state {
            if menu.progress_receiver.is_none() {
                return Subscription::none();
            }
            return iced::time::every(Duration::from_millis(1000 / UPDATES_PER_SECOND))
                .map(|_| Message::CreateInstanceProgressUpdate);
        }

        if let State::Launch(MenuLaunch {
            java_install_progress: Some(_),
            ..
        }) = &self.state
        {
            return iced::time::every(Duration::from_millis(1000 / UPDATES_PER_SECOND))
                .map(|_| Message::LaunchJavaInstallProgressUpdate);
        }

        if let State::EditInstance(_) = &self.state {
            return iced::time::every(Duration::from_millis(1000 / UPDATES_PER_SECOND))
                .map(|_| Message::EditInstanceSave);
        }

        Subscription::none()
    }

    fn view(&self) -> iced::Element<'_, Self::Message, Self::Theme, iced::Renderer> {
        match &self.state {
            State::Launch(menu) => menu.view(self.config.as_ref(), self.instances.as_deref()),
            State::EditInstance(menu) => menu.view(),
            State::EditMods(menu) => menu.view(),
            State::Create(menu) => menu.view(),
            State::DeleteInstance(menu) => menu.view(),
            State::Error { error } => widget::column!(
                widget::text(format!("Error: {}", error)),
                widget::button("Back").on_press(Message::LaunchScreenOpen),
                widget::button("Copy Error").on_press(Message::ErrorCopy),
            )
            .into(),
            State::InstallFabric(menu) => menu.view(),
        }
    }
}

fn receive_java_install_progress(
    java_install_progress: &mut Option<launcher_state::JavaInstallProgress>,
) -> bool {
    let Some(java_install_progress) = java_install_progress else {
        return true;
    };

    match java_install_progress.recv.try_recv() {
        Ok(message) => match message {
            quantum_launcher_backend::JavaInstallMessage::P1Started => {
                java_install_progress.num = 0.0;
                java_install_progress.message = "Starting up (2/2)".to_owned();
            }
            quantum_launcher_backend::JavaInstallMessage::P2 {
                progress,
                out_of,
                name,
            } => {
                java_install_progress.num = (progress as f32) / (out_of as f32);
                java_install_progress.message =
                    format!("Downloading ({progress}/{out_of}): {name}");
            }
            quantum_launcher_backend::JavaInstallMessage::P3Done => {
                java_install_progress.num = 1.0;
                java_install_progress.message = "Done!".to_owned();
                return true;
            }
        },
        Err(_) => {}
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
    const WINDOW_HEIGHT: f32 = 550.0;
    const WINDOW_WIDTH: f32 = 220.0;

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
