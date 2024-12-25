use iced::Command;
use ql_core::InstanceSelection;
use ql_mod_manager::instance_mod_installer;

use crate::{
    launcher_state::{
        CreateInstanceMessage, EditInstanceMessage, InstallFabricMessage, Launcher,
        MenuCreateInstance, MenuInstallFabric, Message, State,
    },
    message_handler::format_memory,
};

impl Launcher {
    pub fn update_install_fabric(&mut self, message: InstallFabricMessage) -> Command<Message> {
        match message {
            InstallFabricMessage::End(result) => match result {
                Ok(()) => self.go_to_launch_screen_with_message("Installed Fabric".to_owned()),
                Err(err) => self.set_error(err),
            },
            InstallFabricMessage::VersionSelected(selection) => {
                if let State::InstallFabric(MenuInstallFabric::Loaded { fabric_version, .. }) =
                    &mut self.state
                {
                    *fabric_version = Some(selection);
                }
            }
            InstallFabricMessage::VersionsLoaded(result) => match result {
                Ok(list_of_versions) => {
                    if let State::InstallFabric(menu) = &mut self.state {
                        if list_of_versions.is_empty() {
                            *menu = MenuInstallFabric::Unsupported;
                        } else {
                            *menu = MenuInstallFabric::Loaded {
                                fabric_version: None,
                                fabric_versions: list_of_versions
                                    .iter()
                                    .map(|ver| ver.loader.version.clone())
                                    .collect(),
                                progress_receiver: None,
                                progress_num: 0.0,
                                progress_message: String::new(),
                            };
                        }
                    }
                }
                Err(err) => self.set_error(err),
            },
            InstallFabricMessage::ButtonClicked => {
                if let State::InstallFabric(MenuInstallFabric::Loaded {
                    fabric_version,
                    progress_receiver,
                    ..
                }) = &mut self.state
                {
                    let (sender, receiver) = std::sync::mpsc::channel();
                    *progress_receiver = Some(receiver);

                    match self.selected_instance.as_ref().unwrap() {
                        InstanceSelection::Instance(n) => {
                            return Command::perform(
                                instance_mod_installer::fabric::install_client_wrapped(
                                    fabric_version.clone().unwrap(),
                                    n.clone(),
                                    Some(sender),
                                ),
                                |m| Message::InstallFabric(InstallFabricMessage::End(m)),
                            );
                        }
                        InstanceSelection::Server(_) => todo!(),
                    }
                }
            }
            InstallFabricMessage::ScreenOpen => {
                self.state = State::InstallFabric(MenuInstallFabric::Loading);

                return Command::perform(
                    instance_mod_installer::fabric::get_list_of_versions_wrapped(
                        self.selected_instance.clone().unwrap(),
                    ),
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
                self.select_created_instance_version(selected_version);
            }
            CreateInstanceMessage::NameInput(name) => self.update_created_instance_name(name),
            CreateInstanceMessage::Start => return self.create_instance(),
            CreateInstanceMessage::End(result) => match result {
                Ok(instance) => {
                    self.selected_instance = Some(InstanceSelection::Instance(instance));
                    self.go_to_launch_screen_with_message("Created Instance".to_owned())
                }
                Err(n) => self.state = State::Error { error: n },
            },
            CreateInstanceMessage::ChangeAssetToggle(t) => {
                if let State::Create(MenuCreateInstance::Loaded {
                    download_assets, ..
                }) = &mut self.state
                {
                    *download_assets = t;
                }
            }
        }
        Command::none()
    }

    pub fn update_edit_instance(&mut self, message: EditInstanceMessage) -> Command<Message> {
        match message {
            EditInstanceMessage::MenuOpen => self.edit_instance_wrapped(),
            EditInstanceMessage::JavaOverride(n) => {
                if let State::EditInstance(menu) = &mut self.state {
                    menu.config.java_override = Some(n);
                }
            }
            EditInstanceMessage::MemoryChanged(new_slider_value) => {
                if let State::EditInstance(menu) = &mut self.state {
                    menu.slider_value = new_slider_value;
                    menu.config.ram_in_mb = 2f32.powf(new_slider_value) as usize;
                    menu.slider_text = format_memory(menu.config.ram_in_mb);
                }
            }
            EditInstanceMessage::LoggingToggle(t) => {
                if let State::EditInstance(menu) = &mut self.state {
                    menu.config.enable_logger = Some(t);
                }
            }
            EditInstanceMessage::JavaArgsAdd => {
                if let State::EditInstance(menu) = &mut self.state {
                    menu.config
                        .java_args
                        .get_or_insert_with(Vec::new)
                        .push(String::new());
                }
            }
            EditInstanceMessage::JavaArgEdit(msg, idx) => {
                let State::EditInstance(menu) = &mut self.state else {
                    return Command::none();
                };
                let Some(args) = menu.config.java_args.as_mut() else {
                    return Command::none();
                };
                add_to_arguments_list(msg, args, idx);
            }
            EditInstanceMessage::JavaArgDelete(idx) => {
                if let State::EditInstance(menu) = &mut self.state {
                    if let Some(args) = &mut menu.config.java_args {
                        args.remove(idx);
                    }
                }
            }
            EditInstanceMessage::GameArgsAdd => {
                if let State::EditInstance(menu) = &mut self.state {
                    menu.config
                        .game_args
                        .get_or_insert_with(Vec::new)
                        .push(String::new());
                }
            }
            EditInstanceMessage::GameArgEdit(msg, idx) => {
                let State::EditInstance(menu) = &mut self.state else {
                    return Command::none();
                };
                let Some(args) = &mut menu.config.game_args else {
                    return Command::none();
                };
                add_to_arguments_list(msg, args, idx);
            }
            EditInstanceMessage::GameArgDelete(idx) => {
                if let State::EditInstance(menu) = &mut self.state {
                    if let Some(args) = &mut menu.config.game_args {
                        args.remove(idx);
                    }
                }
            }
            EditInstanceMessage::JavaArgShiftUp(idx) => {
                let State::EditInstance(menu) = &mut self.state else {
                    return Command::none();
                };
                let Some(args) = &mut menu.config.java_args else {
                    return Command::none();
                };
                if idx > 0 {
                    args.swap(idx, idx - 1);
                }
            }
            EditInstanceMessage::JavaArgShiftDown(idx) => {
                let State::EditInstance(menu) = &mut self.state else {
                    return Command::none();
                };
                let Some(args) = &mut menu.config.java_args else {
                    return Command::none();
                };
                if idx + 1 < args.len() {
                    args.swap(idx, idx + 1);
                }
            }
            EditInstanceMessage::GameArgShiftUp(idx) => {
                let State::EditInstance(menu) = &mut self.state else {
                    return Command::none();
                };
                let Some(args) = &mut menu.config.game_args else {
                    return Command::none();
                };
                if idx > 0 {
                    args.swap(idx, idx - 1);
                }
            }
            EditInstanceMessage::GameArgShiftDown(idx) => {
                let State::EditInstance(menu) = &mut self.state else {
                    return Command::none();
                };
                let Some(args) = &mut menu.config.game_args else {
                    return Command::none();
                };
                if idx + 1 < args.len() {
                    args.swap(idx, idx + 1);
                }
            }
        }
        Command::none()
    }
}

fn add_to_arguments_list(msg: String, args: &mut Vec<String>, mut idx: usize) {
    if msg.contains(' ') {
        args.remove(idx);
        for s in msg.split(' ').filter(|n| !n.is_empty()) {
            args.insert(idx, s.to_owned());
            idx += 1;
        }
    } else if let Some(arg) = args.get_mut(idx) {
        *arg = msg;
    }
}
