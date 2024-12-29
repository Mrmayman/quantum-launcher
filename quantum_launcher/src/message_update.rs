use std::path::PathBuf;

use iced::Command;
use ql_core::{file_utils, InstanceSelection, IntoIoError};
use ql_mod_manager::instance_mod_installer;

use crate::{
    launcher_state::{
        CreateInstanceMessage, EditInstanceMessage, InstallFabricMessage, Launcher,
        ManageModsMessage, MenuCreateInstance, MenuInstallFabric, Message, SelectedMod,
        SelectedState, State,
    },
    message_handler::format_memory,
};

impl Launcher {
    pub fn update_install_fabric(&mut self, message: InstallFabricMessage) -> Command<Message> {
        match message {
            InstallFabricMessage::End(result) => match result {
                Ok(()) => {
                    let message = "Installed Fabric".to_owned();
                    return self.go_to_main_menu(Some(message));
                }
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
                            *menu = MenuInstallFabric::Unsupported(menu.is_quilt());
                        } else {
                            *menu = MenuInstallFabric::Loaded {
                                is_quilt: menu.is_quilt(),
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
                    is_quilt,
                    ..
                }) = &mut self.state
                {
                    let (sender, receiver) = std::sync::mpsc::channel();
                    *progress_receiver = Some(receiver);
                    let loader_version = fabric_version.clone().unwrap();

                    return Command::perform(
                        instance_mod_installer::fabric::install_w(
                            loader_version,
                            self.selected_instance.clone().unwrap(),
                            Some(sender),
                            *is_quilt,
                        ),
                        |m| Message::InstallFabric(InstallFabricMessage::End(m)),
                    );
                }
            }
            InstallFabricMessage::ScreenOpen { is_quilt } => {
                self.state = State::InstallFabric(MenuInstallFabric::Loading(is_quilt));

                return Command::perform(
                    instance_mod_installer::fabric::get_list_of_versions_w(
                        self.selected_instance.clone().unwrap(),
                        is_quilt,
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
                    return self.go_to_launch_screen(Some("Created Instance".to_owned()));
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
            EditInstanceMessage::MenuOpen => self.edit_instance_w(),
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
                self.e_java_arg_add();
            }
            EditInstanceMessage::JavaArgEdit(msg, idx) => {
                self.e_java_arg_edit(msg, idx);
            }
            EditInstanceMessage::JavaArgDelete(idx) => {
                self.e_java_arg_delete(idx);
            }
            EditInstanceMessage::GameArgsAdd => {
                self.e_game_arg_add();
            }
            EditInstanceMessage::GameArgEdit(msg, idx) => {
                self.e_game_arg_edit(msg, idx);
            }
            EditInstanceMessage::GameArgDelete(idx) => {
                self.e_game_arg_delete(idx);
            }
            EditInstanceMessage::JavaArgShiftUp(idx) => {
                self.e_java_arg_shift_up(idx);
            }
            EditInstanceMessage::JavaArgShiftDown(idx) => {
                self.e_java_arg_shift_down(idx);
            }
            EditInstanceMessage::GameArgShiftUp(idx) => {
                self.e_game_arg_shift_up(idx);
            }
            EditInstanceMessage::GameArgShiftDown(idx) => {
                self.e_game_arg_shift_down(idx);
            }
        }
        Command::none()
    }

    fn e_java_arg_add(&mut self) {
        if let State::EditInstance(menu) = &mut self.state {
            menu.config
                .java_args
                .get_or_insert_with(Vec::new)
                .push(String::new());
        }
    }

    fn e_java_arg_edit(&mut self, msg: String, idx: usize) {
        let State::EditInstance(menu) = &mut self.state else {
            return;
        };
        let Some(args) = menu.config.java_args.as_mut() else {
            return;
        };
        add_to_arguments_list(msg, args, idx);
    }

    fn e_java_arg_delete(&mut self, idx: usize) {
        if let State::EditInstance(menu) = &mut self.state {
            if let Some(args) = &mut menu.config.java_args {
                args.remove(idx);
            }
        }
    }

    fn e_game_arg_add(&mut self) {
        if let State::EditInstance(menu) = &mut self.state {
            menu.config
                .game_args
                .get_or_insert_with(Vec::new)
                .push(String::new());
        }
    }

    fn e_game_arg_edit(&mut self, msg: String, idx: usize) {
        let State::EditInstance(menu) = &mut self.state else {
            return;
        };
        let Some(args) = &mut menu.config.game_args else {
            return;
        };
        add_to_arguments_list(msg, args, idx);
    }

    fn e_game_arg_delete(&mut self, idx: usize) {
        if let State::EditInstance(menu) = &mut self.state {
            if let Some(args) = &mut menu.config.game_args {
                args.remove(idx);
            }
        }
    }

    fn e_java_arg_shift_up(&mut self, idx: usize) {
        let State::EditInstance(menu) = &mut self.state else {
            return;
        };
        let Some(args) = &mut menu.config.java_args else {
            return;
        };
        if idx > 0 {
            args.swap(idx, idx - 1);
        }
    }

    fn e_java_arg_shift_down(&mut self, idx: usize) {
        let State::EditInstance(menu) = &mut self.state else {
            return;
        };
        let Some(args) = &mut menu.config.java_args else {
            return;
        };
        if idx + 1 < args.len() {
            args.swap(idx, idx + 1);
        }
    }

    fn e_game_arg_shift_up(&mut self, idx: usize) {
        let State::EditInstance(menu) = &mut self.state else {
            return;
        };
        let Some(args) = &mut menu.config.game_args else {
            return;
        };
        if idx > 0 {
            args.swap(idx, idx - 1);
        }
    }

    fn e_game_arg_shift_down(&mut self, idx: usize) {
        let State::EditInstance(menu) = &mut self.state else {
            return;
        };
        let Some(args) = &mut menu.config.game_args else {
            return;
        };
        if idx + 1 < args.len() {
            args.swap(idx, idx + 1);
        }
    }

    pub fn update_manage_mods(&mut self, msg: ManageModsMessage) -> Command<Message> {
        match msg {
            ManageModsMessage::ScreenOpen => match self.go_to_edit_mods_menu() {
                Ok(command) => return command,
                Err(err) => self.set_error(err),
            },
            ManageModsMessage::ToggleCheckbox((name, id), enable) => {
                if let State::EditMods(menu) = &mut self.state {
                    if enable {
                        menu.selected_mods
                            .insert(SelectedMod::Downloaded { name, id });
                        menu.selected_state = SelectedState::Some;
                    } else {
                        menu.selected_mods
                            .remove(&SelectedMod::Downloaded { name, id });
                        menu.selected_state = if menu.selected_mods.is_empty() {
                            SelectedState::None
                        } else {
                            SelectedState::Some
                        };
                    }
                }
            }
            ManageModsMessage::ToggleCheckboxLocal(name, enable) => {
                if let State::EditMods(menu) = &mut self.state {
                    if enable {
                        menu.selected_mods
                            .insert(SelectedMod::Local { file_name: name });
                        menu.selected_state = SelectedState::Some;
                    } else {
                        menu.selected_mods
                            .remove(&SelectedMod::Local { file_name: name });
                        menu.selected_state = if menu.selected_mods.is_empty() {
                            SelectedState::None
                        } else {
                            SelectedState::Some
                        };
                    }
                }
            }
            ManageModsMessage::DeleteSelected => {
                if let State::EditMods(menu) = &self.state {
                    let command = Self::get_delete_mods_command(
                        self.selected_instance.clone().unwrap(),
                        menu,
                    );
                    let mods_dir =
                        file_utils::get_dot_minecraft_dir(self.selected_instance.as_ref().unwrap())
                            .unwrap()
                            .join("mods");
                    let file_paths = menu
                        .selected_mods
                        .iter()
                        .filter_map(|s_mod| {
                            if let SelectedMod::Local { file_name } = s_mod {
                                Some(file_name.clone())
                            } else {
                                None
                            }
                        })
                        .map(|n| mods_dir.join(n))
                        .map(delete_file_wrapper)
                        .map(|n| {
                            Command::perform(n, |n| {
                                Message::ManageMods(ManageModsMessage::LocalDeleteFinished(n))
                            })
                        });
                    let delete_local_command = Command::batch(file_paths);

                    return Command::batch(vec![command, delete_local_command]);
                }
            }
            ManageModsMessage::DeleteFinished(result) => match result {
                Ok(_) => {
                    self.update_mod_index();
                }
                Err(err) => self.set_error(err),
            },
            ManageModsMessage::LocalDeleteFinished(result) => {
                if let Err(err) = result {
                    self.set_error(err);
                }
            }
            ManageModsMessage::LocalIndexLoaded(hash_set) => {
                if let State::EditMods(menu) = &mut self.state {
                    menu.locally_installed_mods = hash_set;
                }
            }
            ManageModsMessage::ToggleSelected => {
                if let State::EditMods(menu) = &self.state {
                    let ids = menu
                        .selected_mods
                        .iter()
                        .filter_map(|s_mod| {
                            if let SelectedMod::Downloaded { id, .. } = s_mod {
                                Some(id.clone())
                            } else {
                                None
                            }
                        })
                        .collect();
                    return Command::perform(
                        ql_mod_manager::mod_manager::toggle_mods_w(
                            ids,
                            self.selected_instance.clone().unwrap(),
                        ),
                        |n| Message::ManageMods(ManageModsMessage::ToggleFinished(n)),
                    );
                }
            }
            ManageModsMessage::ToggleFinished(err) => {
                if let Err(err) = err {
                    self.set_error(err);
                } else {
                    self.update_mod_index();
                }
            }
            ManageModsMessage::UpdateMods => return self.update_mods(),
            ManageModsMessage::UpdateModsFinished(result) => {
                if let Err(err) = result {
                    self.set_error(err);
                } else {
                    self.update_mod_index();
                    if let State::EditMods(menu) = &mut self.state {
                        menu.available_updates.clear();
                    }
                    return Command::perform(
                        ql_mod_manager::mod_manager::check_for_updates(
                            self.selected_instance.clone().unwrap(),
                        ),
                        Message::ManageModsUpdateCheckResult,
                    );
                }
            }
        }
        Command::none()
    }

    fn get_delete_mods_command(
        selected_instance: InstanceSelection,
        menu: &crate::launcher_state::MenuEditMods,
    ) -> Command<Message> {
        let ids = menu
            .selected_mods
            .iter()
            .filter_map(|s_mod| {
                if let SelectedMod::Downloaded { id, .. } = s_mod {
                    Some(id.clone())
                } else {
                    None
                }
            })
            .collect();

        Command::perform(
            ql_mod_manager::mod_manager::delete_mods_w(ids, selected_instance),
            |n| Message::ManageMods(ManageModsMessage::DeleteFinished(n)),
        )
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

async fn delete_file_wrapper(path: PathBuf) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    tokio::fs::remove_file(&path)
        .await
        .path(path)
        .map_err(|n| n.to_string())
}
