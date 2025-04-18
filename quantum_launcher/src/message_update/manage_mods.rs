use std::path::PathBuf;

use iced::Task;
use ql_core::{err_no_log, InstanceSelection, IntoIoError, IntoStringError, ModId, SelectedMod};
use ql_mod_manager::mod_manager::ModIndex;

use crate::launcher_state::{
    Launcher, ManageModsMessage, MenuEditMods, Message, SelectedState, State,
};

impl Launcher {
    pub fn update_manage_mods(&mut self, msg: ManageModsMessage) -> Task<Message> {
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
                    let mods_dir = self.get_selected_dot_minecraft_dir().unwrap().join("mods");
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
                            Task::perform(n, |n| {
                                Message::ManageMods(ManageModsMessage::LocalDeleteFinished(n))
                            })
                        });
                    let delete_local_command = Task::batch(file_paths);

                    return Task::batch([command, delete_local_command]);
                }
            }
            ManageModsMessage::DeleteFinished(result) => match result {
                Ok(_) => {
                    if let State::EditMods(menu) = &mut self.state {
                        menu.selected_mods.clear();
                    }
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
                if let State::EditMods(menu) = &mut self.state {
                    let (ids_downloaded, ids_local) = menu.get_kinds_of_ids();
                    let instance_name = self.selected_instance.clone().unwrap();

                    menu.selected_mods.retain(|n| {
                        if let SelectedMod::Local { file_name } = n {
                            !ids_local.contains(file_name)
                        } else {
                            true
                        }
                    });

                    menu.selected_mods
                        .extend(ids_local.iter().map(|n| SelectedMod::Local {
                            file_name: ql_mod_manager::mod_manager::flip_filename(n),
                        }));

                    let toggle_downloaded = Task::perform(
                        ql_mod_manager::mod_manager::toggle_mods(
                            ids_downloaded,
                            instance_name.clone(),
                        ),
                        |n| Message::ManageMods(ManageModsMessage::ToggleFinished(n.strerr())),
                    );
                    let toggle_local = Task::perform(
                        ql_mod_manager::mod_manager::toggle_mods_local(
                            ids_local,
                            instance_name.clone(),
                        ),
                        |n| Message::ManageMods(ManageModsMessage::ToggleFinished(n.strerr())),
                    )
                    .chain(MenuEditMods::update_locally_installed_mods(
                        &menu.mods,
                        &instance_name,
                        &self.dir,
                    ));

                    return Task::batch([toggle_downloaded, toggle_local]);
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
                    return Task::perform(
                        ql_mod_manager::mod_manager::check_for_updates(
                            self.selected_instance.clone().unwrap(),
                        ),
                        |n| Message::ManageMods(ManageModsMessage::UpdateCheckResult(n.strerr())),
                    );
                }
            }
            ManageModsMessage::UpdateCheckResult(updates) => {
                let updates = match updates {
                    Ok(n) => n,
                    Err(err) => {
                        err_no_log!("Could not check for updates: {err}");
                        return Task::none();
                    }
                };

                if let State::EditMods(menu) = &mut self.state {
                    menu.available_updates =
                        updates.into_iter().map(|(a, b)| (a, b, true)).collect();
                }
            }
            ManageModsMessage::UpdateCheckToggle(idx, t) => {
                if let State::EditMods(MenuEditMods {
                    available_updates, ..
                }) = &mut self.state
                {
                    if let Some((_, _, b)) = available_updates.get_mut(idx) {
                        *b = t;
                    }
                }
            }
            ManageModsMessage::SelectAll => {
                if let State::EditMods(menu) = &mut self.state {
                    match menu.selected_state {
                        SelectedState::All => {
                            menu.selected_mods.clear();
                            menu.selected_state = SelectedState::None;
                        }
                        SelectedState::Some | SelectedState::None => {
                            menu.selected_mods = menu
                                .mods
                                .mods
                                .iter()
                                .filter_map(|(id, mod_info)| {
                                    mod_info
                                        .manually_installed
                                        .then_some(SelectedMod::Downloaded {
                                            name: mod_info.name.clone(),
                                            id: ModId::from_index_str(id),
                                        })
                                })
                                .chain(menu.locally_installed_mods.iter().map(|n| {
                                    SelectedMod::Local {
                                        file_name: n.clone(),
                                    }
                                }))
                                .collect();
                            menu.selected_state = SelectedState::All;
                        }
                    }
                }
            }
        }
        Task::none()
    }

    fn get_delete_mods_command(
        selected_instance: InstanceSelection,
        menu: &crate::launcher_state::MenuEditMods,
    ) -> Task<Message> {
        let ids: Vec<ModId> = menu
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

        Task::perform(
            async move {
                let ids = ids;
                let selected_instance = selected_instance;
                ql_mod_manager::mod_manager::delete_mods(&ids, &selected_instance)
                    .await
                    .map(|()| ids)
            },
            |n| Message::ManageMods(ManageModsMessage::DeleteFinished(n.strerr())),
        )
    }

    fn update_mod_index(&mut self) {
        if let State::EditMods(menu) = &mut self.state {
            match ModIndex::get_s(self.selected_instance.as_ref().unwrap()).strerr() {
                Ok(idx) => menu.mods = idx,
                Err(err) => self.set_error(err),
            }
        }
    }
}

async fn delete_file_wrapper(path: PathBuf) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    tokio::fs::remove_file(&path).await.path(path).strerr()
}
