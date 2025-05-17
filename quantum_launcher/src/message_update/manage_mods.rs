use std::{collections::HashSet, path::PathBuf};

use iced::Task;
use ql_core::{
    err, err_no_log, jarmod::JarMods, InstanceSelection, IntoIoError, IntoStringError, ModId,
    SelectedMod,
};
use ql_mod_manager::store::ModIndex;

use crate::launcher_state::{
    Launcher, ManageJarModsMessage, ManageModsMessage, MenuCurseforgeManualDownload,
    MenuEditJarMods, MenuEditMods, Message, ProgressBar, SelectedState, State,
};

impl Launcher {
    pub fn update_manage_mods(&mut self, msg: ManageModsMessage) -> Task<Message> {
        match msg {
            ManageModsMessage::ScreenOpen => match self.go_to_edit_mods_menu() {
                Ok(command) => return command,
                Err(err) => self.set_error(err),
            },
            ManageModsMessage::ScreenOpenWithoutUpdate => {
                match self.go_to_edit_mods_menu_without_update_check() {
                    Ok(command) => return command,
                    Err(err) => self.set_error(err),
                }
            }

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
            ManageModsMessage::AddFile => {
                if let Some(paths) = rfd::FileDialog::new()
                    .add_filter("Mod/Modpack", &["jar", "zip", "mrpack", "qmp"])
                    .set_title("Add Mod, Modpack or Preset")
                    .pick_files()
                {
                    let (sender, receiver) = std::sync::mpsc::channel();

                    self.state = State::ImportModpack(ProgressBar::with_recv(receiver));

                    return Task::perform(
                        ql_mod_manager::add_files(
                            self.selected_instance.clone().unwrap(),
                            paths,
                            Some(sender),
                        ),
                        |n| Message::ManageMods(ManageModsMessage::AddFileDone(n.strerr())),
                    );
                }
            }
            ManageModsMessage::AddFileDone(n) => match n {
                Ok(unsupported) => {
                    if !unsupported.is_empty() {
                        self.state =
                            State::CurseforgeManualDownload(MenuCurseforgeManualDownload {
                                unsupported,
                            });
                    } else if let Err(err) = self.go_to_edit_mods_menu_without_update_check() {
                        self.set_error(err);
                    }
                }
                Err(err) => self.set_error(err),
            },
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
                            file_name: ql_mod_manager::store::flip_filename(n),
                        }));

                    let toggle_downloaded = Task::perform(
                        ql_mod_manager::store::toggle_mods(ids_downloaded, instance_name.clone()),
                        |n| Message::ManageMods(ManageModsMessage::ToggleFinished(n.strerr())),
                    );
                    let toggle_local = Task::perform(
                        ql_mod_manager::store::toggle_mods_local(ids_local, instance_name.clone()),
                        |n| Message::ManageMods(ManageModsMessage::ToggleFinished(n.strerr())),
                    )
                    .chain(MenuEditMods::update_locally_installed_mods(
                        &menu.mods,
                        &instance_name,
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
                        ql_mod_manager::store::check_for_updates(
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
                    menu.update_check_handle = None;
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
            ql_mod_manager::store::delete_mods(ids, selected_instance),
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

    pub fn update_manage_jar_mods(&mut self, msg: ManageJarModsMessage) -> Task<Message> {
        match msg {
            ManageJarModsMessage::Open => {
                self.manage_jarmods_open();
            }
            ManageJarModsMessage::AddFile => {
                self.manage_jarmods_add_file_from_picker();
            }
            ManageJarModsMessage::ToggleCheckbox(name, enable) => {
                self.manage_jarmods_toggle_checkbox(name, enable);
            }
            ManageJarModsMessage::DeleteSelected => {
                self.manage_jarmods_delete_selected();
            }
            ManageJarModsMessage::ToggleSelected => {
                self.manage_jarmods_toggle_selected();
            }
            ManageJarModsMessage::SelectAll => {
                self.manage_jarmods_select_all();
            }
            ManageJarModsMessage::AutosaveFinished((res, jarmods)) => {
                if let Err(err) = res {
                    self.set_error(format!("While autosaving jarmods index: {err}"));
                } else if let State::EditJarMods(menu) = &mut self.state {
                    let set_a: HashSet<&str> = menu
                        .jarmods
                        .mods
                        .iter()
                        .map(|m| m.filename.as_str())
                        .collect();
                    let set_b: HashSet<&str> =
                        jarmods.mods.iter().map(|m| m.filename.as_str()).collect();

                    if set_a == set_b {
                        menu.jarmods = jarmods;
                    }
                    menu.free_for_autosave = true;
                }
            }

            ManageJarModsMessage::MoveUp | ManageJarModsMessage::MoveDown => {
                self.manage_jarmods_move_up_or_down(&msg);
            }
        }
        Task::none()
    }

    fn manage_jarmods_move_up_or_down(&mut self, msg: &ManageJarModsMessage) {
        if let State::EditJarMods(menu) = &mut self.state {
            let mut selected: Vec<usize> = menu
                .selected_mods
                .iter()
                .filter_map(|selected_name| {
                    menu.jarmods
                        .mods
                        .iter()
                        .enumerate()
                        .find_map(|(i, n)| (n.filename == *selected_name).then_some(i))
                })
                .collect();
            selected.sort_unstable();
            if let ManageJarModsMessage::MoveDown = msg {
                selected.reverse();
            }

            for i in selected {
                if i < menu.jarmods.mods.len() {
                    match msg {
                        ManageJarModsMessage::MoveUp => {
                            if i > 0 {
                                let removed = menu.jarmods.mods.remove(i);
                                menu.jarmods.mods.insert(i - 1, removed);
                            }
                        }
                        ManageJarModsMessage::MoveDown => {
                            if i + 1 < menu.jarmods.mods.len() {
                                let removed = menu.jarmods.mods.remove(i);
                                menu.jarmods.mods.insert(i + 1, removed);
                            }
                        }
                        _ => {}
                    }
                } else {
                    err!(
                        "Out of bounds in jarmods move up/down: !({i} < len:{})",
                        menu.jarmods.mods.len()
                    );
                }
            }
        }
    }

    fn manage_jarmods_select_all(&mut self) {
        if let State::EditJarMods(menu) = &mut self.state {
            match menu.selected_state {
                SelectedState::All => {
                    menu.selected_mods.clear();
                    menu.selected_state = SelectedState::None;
                }
                SelectedState::Some | SelectedState::None => {
                    menu.selected_mods = menu
                        .jarmods
                        .mods
                        .iter()
                        .map(|mod_info| mod_info.filename.clone())
                        .collect();
                    menu.selected_state = SelectedState::All;
                }
            }
        }
    }

    fn manage_jarmods_toggle_selected(&mut self) {
        if let State::EditJarMods(menu) = &mut self.state {
            for selected in &menu.selected_mods {
                if let Some(jarmod) = menu
                    .jarmods
                    .mods
                    .iter_mut()
                    .find(|n| n.filename == *selected)
                {
                    jarmod.enabled = !jarmod.enabled;
                }
            }
        }
    }

    fn manage_jarmods_delete_selected(&mut self) {
        if let State::EditJarMods(menu) = &mut self.state {
            let jarmods_path = self
                .selected_instance
                .as_ref()
                .unwrap()
                .get_instance_path()
                .join("jarmods");

            for selected in &menu.selected_mods {
                let path = jarmods_path.join(selected);
                if path.is_file() {
                    _ = std::fs::remove_file(&path);
                }
            }

            menu.selected_mods.clear();
        }
    }

    fn manage_jarmods_toggle_checkbox(&mut self, name: String, enable: bool) {
        if let State::EditJarMods(menu) = &mut self.state {
            if enable {
                menu.selected_mods.insert(name);
                menu.selected_state = SelectedState::Some;
            } else {
                menu.selected_mods.remove(&name);
                menu.selected_state = if menu.selected_mods.is_empty() {
                    SelectedState::None
                } else {
                    SelectedState::Some
                };
            }
        }
    }

    fn manage_jarmods_add_file_from_picker(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("jar/zip", &["jar", "zip"])
            .set_title("Pick a Jar Mod Patch (.jar/.zip)")
            .pick_file()
        {
            if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                let dest = self
                    .selected_instance
                    .as_ref()
                    .unwrap()
                    .get_instance_path()
                    .join("jarmods")
                    .join(filename);
                if let Err(err) = std::fs::copy(&path, dest) {
                    self.set_error(format!("While picking jar mod to be added: {err}"));
                }
            }
        }
    }

    fn manage_jarmods_open(&mut self) {
        let jarmods = match JarMods::get_s(self.selected_instance.as_ref().unwrap()) {
            Ok(n) => n,
            Err(err) => {
                self.set_error(format!("While opening jar mods screen: {err}"));
                return;
            }
        };
        self.state = State::EditJarMods(MenuEditJarMods {
            jarmods,
            selected_state: SelectedState::None,
            selected_mods: HashSet::new(),
            drag_and_drop_hovered: false,
            free_for_autosave: true,
        });
    }
}

async fn delete_file_wrapper(path: PathBuf) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    tokio::fs::remove_file(&path).await.path(path).strerr()
}
