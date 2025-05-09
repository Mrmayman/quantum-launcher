use std::collections::HashSet;

use iced::Task;
use ql_core::{
    err, json::VersionDetails, InstanceSelection, IntoStringError, Loader, ModId, SelectedMod,
};
use ql_mod_manager::store::{RecommendedMod, RECOMMENDED_MODS};

use crate::launcher_state::{
    EditPresetsMessage, Launcher, MenuEditPresets, MenuEditPresetsInner, Message, ProgressBar,
    SelectedState, State, PRESET_INNER_BUILD, PRESET_INNER_RECOMMENDED,
};

macro_rules! iflet_manage_preset {
    ($self:ident, $variant:ident, $($field:ident),+, { $($code:tt)* }) => {
        if let State::ManagePresets(MenuEditPresets {
            inner: MenuEditPresetsInner::$variant { $($field,)* .. },
            ..
        }) = &mut $self.state
        {
            $($code)*
        }
    };
}

impl Launcher {
    pub fn update_edit_presets(
        &mut self,
        message: EditPresetsMessage,
    ) -> Result<Task<Message>, String> {
        match message {
            EditPresetsMessage::Open => return Ok(self.go_to_edit_presets_menu()),
            EditPresetsMessage::TabChange(tab) => {
                if let Some(value) = self.preset_change_tab(&tab) {
                    return Ok(value);
                }
            }
            EditPresetsMessage::ToggleCheckbox((name, id), enable) => {
                iflet_manage_preset!(self, Build, selected_mods, selected_state, {
                    if enable {
                        selected_mods.insert(SelectedMod::Downloaded { name, id });
                    } else {
                        selected_mods.remove(&SelectedMod::Downloaded { name, id });
                    }
                    *selected_state = SelectedState::Some;
                });
            }
            EditPresetsMessage::ToggleCheckboxLocal(file_name, enable) => {
                iflet_manage_preset!(self, Build, selected_mods, selected_state, {
                    if enable {
                        selected_mods.insert(SelectedMod::Local { file_name });
                    } else {
                        selected_mods.remove(&SelectedMod::Local { file_name });
                    }
                    *selected_state = SelectedState::Some;
                });
            }
            EditPresetsMessage::SelectAll => {
                self.preset_select_all();
            }
            EditPresetsMessage::BuildYourOwn => {
                iflet_manage_preset!(self, Build, selected_mods, is_building, {
                    *is_building = true;
                    let selected_instance = self.selected_instance.clone().unwrap();
                    let selected_mods = selected_mods.clone();
                    return Ok(Task::perform(
                        ql_mod_manager::PresetJson::generate(selected_instance, selected_mods),
                        |n| Message::EditPresets(EditPresetsMessage::BuildYourOwnEnd(n.strerr())),
                    ));
                });
            }
            EditPresetsMessage::BuildYourOwnEnd(result) => match self.build_end(result) {
                Ok(task) => return Ok(task),
                Err(err) => self.set_error(err),
            },
            EditPresetsMessage::Load => return Ok(self.load_preset()),
            EditPresetsMessage::LoadComplete(result) => {
                return result.and_then(|()| self.go_to_edit_mods_menu().strerr());
            }
            EditPresetsMessage::RecommendedModCheck(result) => {
                if let State::ManagePresets(MenuEditPresets {
                    inner: MenuEditPresetsInner::Recommended { error, .. },
                    recommended_mods,
                    ..
                }) = &mut self.state
                {
                    match result {
                        Ok(n) => {
                            *recommended_mods = Some(n.into_iter().map(|n| (true, n)).collect());
                        }
                        Err(err) => *error = Some(err),
                    }
                }
            }
            EditPresetsMessage::RecommendedToggle(idx, toggle) => {
                if let State::ManagePresets(MenuEditPresets {
                    recommended_mods: Some(recommended_mods),
                    ..
                }) = &mut self.state
                {
                    if let Some((t, _)) = recommended_mods.get_mut(idx) {
                        *t = toggle;
                    }
                }
            }
            EditPresetsMessage::RecommendedDownload => {
                return Ok(self.preset_download_recommended())
            }
            EditPresetsMessage::RecommendedDownloadEnd(result) => {
                result?;
                return self.go_to_edit_mods_menu_without_update_check().strerr();
            }
        }
        Ok(Task::none())
    }

    fn preset_download_recommended(&mut self) -> Task<Message> {
        if let State::ManagePresets(MenuEditPresets {
            recommended_mods: Some(recommended_mods),
            progress,
            ..
        }) = &mut self.state
        {
            let (sender, receiver) = std::sync::mpsc::channel();
            *progress = Some(ProgressBar::with_recv(receiver));

            let ids: Vec<ModId> = recommended_mods
                .iter()
                .filter(|n| n.0)
                .map(|n| ModId::from_pair(n.1.id, n.1.backend))
                .collect();

            let instance = self.selected_instance.clone().unwrap();

            Task::perform(
                ql_mod_manager::store::download_mods_bulk(ids, instance, Some(sender)),
                |n| Message::EditPresets(EditPresetsMessage::RecommendedDownloadEnd(n.strerr())),
            )
        } else {
            Task::none()
        }
    }

    fn preset_change_tab(&mut self, tab: &str) -> Option<Task<Message>> {
        if let State::ManagePresets(MenuEditPresets {
            inner,
            config,
            sorted_mods_list,
            recommended_mods,
            ..
        }) = &mut self.state
        {
            let selected_mods = sorted_mods_list
                .iter()
                .filter_map(|n| n.is_manually_installed().then_some(n.id()))
                .collect::<HashSet<_>>();

            match tab {
                PRESET_INNER_BUILD => {
                    *inner = MenuEditPresetsInner::Build {
                        selected_mods,
                        selected_state: SelectedState::All,
                        is_building: false,
                    };
                }
                PRESET_INNER_RECOMMENDED => {
                    if let Some(task) = Self::presets_switch_to_recommended(
                        self.selected_instance.as_ref().unwrap(),
                        inner,
                        config,
                        recommended_mods,
                    ) {
                        return Some(task);
                    }
                }
                _ => {
                    err!("Invalid mod preset tab: {tab}");
                }
            }
        }
        None
    }

    fn preset_select_all(&mut self) {
        if let State::ManagePresets(MenuEditPresets {
            inner:
                MenuEditPresetsInner::Build {
                    selected_mods,
                    selected_state,
                    ..
                },
            sorted_mods_list,
            ..
        }) = &mut self.state
        {
            match selected_state {
                SelectedState::All => {
                    selected_mods.clear();
                    *selected_state = SelectedState::None;
                }
                SelectedState::Some | SelectedState::None => {
                    *selected_mods = sorted_mods_list
                        .iter()
                        .filter_map(|mod_info| {
                            mod_info.is_manually_installed().then_some(mod_info.id())
                        })
                        .collect();
                    *selected_state = SelectedState::All;
                }
            }
        }
    }

    fn presets_switch_to_recommended(
        selected_instance: &InstanceSelection,
        inner: &mut MenuEditPresetsInner,
        config: &mut ql_core::json::InstanceConfigJson,
        recommended_mods: &mut Option<Vec<(bool, RecommendedMod)>>,
    ) -> Option<Task<Message>> {
        let mod_type = config.mod_type.clone();
        let (sender, receiver) = std::sync::mpsc::channel();
        *inner = MenuEditPresetsInner::Recommended {
            progress: ProgressBar::with_recv(receiver),
            error: None,
        };
        if recommended_mods.is_some() {
            return None;
        }
        let json = VersionDetails::load_s(&selected_instance.get_instance_path())?;
        let loader = Loader::try_from(mod_type.as_str()).ok()?;
        let version = json.id.clone();
        let ids = RECOMMENDED_MODS.to_owned();

        Some(Task::perform(
            RecommendedMod::get_compatible_mods(ids, version, loader, sender),
            |n| Message::EditPresets(EditPresetsMessage::RecommendedModCheck(n.strerr())),
        ))
    }

    fn go_to_edit_presets_menu(&mut self) -> Task<Message> {
        let State::EditMods(menu) = &self.state else {
            return Task::none();
        };

        let selected_mods = menu
            .sorted_mods_list
            .iter()
            .filter_map(|n| n.is_manually_installed().then_some(n.id()))
            .collect::<HashSet<_>>();

        let is_empty = menu.sorted_mods_list.is_empty();

        let mod_type = menu.config.mod_type.clone();

        let (sender, receiver) = std::sync::mpsc::channel();

        self.state = State::ManagePresets(MenuEditPresets {
            inner: if is_empty {
                MenuEditPresetsInner::Recommended {
                    progress: ProgressBar::with_recv(receiver),
                    error: None,
                }
            } else {
                MenuEditPresetsInner::Build {
                    selected_mods,
                    selected_state: SelectedState::All,
                    is_building: false,
                }
            },
            recommended_mods: None,
            progress: None,
            config: menu.config.clone(),
            sorted_mods_list: menu.sorted_mods_list.clone(),
            drag_and_drop_hovered: false,
        });

        if !is_empty {
            return Task::none();
        }

        let Some(json) = VersionDetails::load_s(&self.get_selected_instance_dir().unwrap()) else {
            return Task::none();
        };

        let Ok(loader) = Loader::try_from(mod_type.as_str()) else {
            return Task::none();
        };

        let version = json.id.clone();
        let ids = RECOMMENDED_MODS.to_owned();
        Task::perform(
            RecommendedMod::get_compatible_mods(ids, version, loader, sender),
            |n| Message::EditPresets(EditPresetsMessage::RecommendedModCheck(n.strerr())),
        )
    }

    fn load_preset(&mut self) -> Task<Message> {
        let Some(file) = rfd::FileDialog::new()
            .add_filter("QuantumLauncher Mod Preset", &["qmp"])
            .set_title("Select Mod Preset to Load")
            .pick_file()
        else {
            return Task::none();
        };

        self.load_qmp_from_path(&file)
    }

    fn build_end(&mut self, preset: Result<Vec<u8>, String>) -> Result<Task<Message>, String> {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("QuantumLauncher Preset", &["qmp"])
            .set_file_name("my_preset.qmp")
            .set_title("Save your QuantumLauncher Preset")
            .save_file()
        {
            std::fs::write(path, preset?).strerr()?;
            self.go_to_edit_mods_menu().strerr()
        } else {
            Ok(Task::none())
        }
    }
}
