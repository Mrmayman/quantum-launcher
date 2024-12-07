use iced::Command;
use ql_mod_manager::instance_mod_installer;

use crate::launcher_state::{
    CreateInstanceMessage, InstallFabricMessage, Launcher, MenuInstallFabric, Message, State,
};

impl Launcher {
    pub fn update_install_fabric(&mut self, message: InstallFabricMessage) -> Command<Message> {
        match message {
            InstallFabricMessage::End(result) => match result {
                Ok(()) => self.go_to_launch_screen_with_message("Installed Fabric".to_owned()),
                Err(err) => self.set_error(err),
            },
            InstallFabricMessage::VersionSelected(selection) => {
                if let State::InstallFabric(menu) = &mut self.state {
                    menu.fabric_version = Some(selection);
                }
            }
            InstallFabricMessage::VersionsLoaded(result) => match result {
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
            InstallFabricMessage::ButtonClicked => {
                if let State::InstallFabric(menu) = &mut self.state {
                    let (sender, receiver) = std::sync::mpsc::channel();
                    menu.progress_receiver = Some(receiver);

                    return Command::perform(
                        instance_mod_installer::fabric::install_wrapped(
                            menu.fabric_version.clone().unwrap(),
                            self.selected_instance.clone().unwrap(),
                            Some(sender),
                        ),
                        |m| Message::InstallFabric(InstallFabricMessage::End(m)),
                    );
                }
            }
            InstallFabricMessage::ScreenOpen => {
                self.state = State::InstallFabric(MenuInstallFabric {
                    fabric_version: None,
                    fabric_versions: Vec::new(),
                    progress_receiver: None,
                    progress_num: 0.0,
                    progress_message: String::new(),
                });

                return Command::perform(
                    instance_mod_installer::fabric::get_list_of_versions(),
                    |m| Message::InstallFabric(InstallFabricMessage::VersionsLoaded(m)),
                );
            }
        }
        Command::none()
    }

    pub fn update_create_instance(&mut self, message: CreateInstanceMessage) -> Command<Message> {
        match message {
            CreateInstanceMessage::ScreenOpen => return self.go_to_create_screen(),
            CreateInstanceMessage::VersionsLoaded(result) => {
                self.create_instance_finish_loading_versions_list(result);
            }
            CreateInstanceMessage::VersionSelected(selected_version) => {
                self.select_created_instance_version(selected_version)
            }
            CreateInstanceMessage::NameInput(name) => self.update_created_instance_name(name),
            CreateInstanceMessage::Start => return self.create_instance(),
            CreateInstanceMessage::End(result) => match result {
                Ok(()) => match Launcher::new(Some("Created New Instance".to_owned())) {
                    Ok(launcher) => *self = launcher,
                    Err(err) => self.set_error(err.to_string()),
                },
                Err(n) => self.state = State::Error { error: n },
            },
            CreateInstanceMessage::ChangeAssetToggle(t) => {
                if let State::Create(menu) = &mut self.state {
                    menu.download_assets = t;
                }
            }
        }
        Command::none()
    }
}
