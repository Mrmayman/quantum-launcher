use iced::Command;
use ql_core::SelectedMod;

use crate::launcher_state::{
    EditPresetsMessage, Launcher, MenuEditPresets, MenuEditPresetsInner, Message, ProgressBar,
    SelectedState, State,
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
    pub fn update_edit_presets(&mut self, message: EditPresetsMessage) -> Command<Message> {
        match message {
            EditPresetsMessage::Open => return self.go_to_edit_presets_menu(),
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
                iflet_manage_preset!(self, Build, selected_mods, selected_state, mods, {
                    match selected_state {
                        SelectedState::All => {
                            selected_mods.clear();
                            *selected_state = SelectedState::None;
                        }
                        SelectedState::Some | SelectedState::None => {
                            *selected_mods = mods
                                .iter()
                                .filter_map(|mod_info| {
                                    mod_info.is_manually_installed().then_some(mod_info.id())
                                })
                                .collect();
                            *selected_state = SelectedState::All;
                        }
                    }
                });
            }
            EditPresetsMessage::BuildYourOwn => {
                iflet_manage_preset!(self, Build, selected_mods, is_building, {
                    *is_building = true;
                    return Command::perform(
                        ql_mod_manager::PresetJson::generate_w(
                            self.selected_instance.clone().unwrap(),
                            selected_mods.clone(),
                        ),
                        |n| Message::EditPresets(EditPresetsMessage::BuildYourOwnEnd(n)),
                    );
                });
            }
            EditPresetsMessage::BuildYourOwnEnd(result) => match result {
                Ok(preset) => {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("QuantumLauncher Preset", &["qmp"])
                        .set_file_name("my_preset.qmp")
                        .set_title("Save your QuantumLauncher Preset")
                        .save_file()
                    {
                        if let Err(err) = std::fs::write(path, preset) {
                            self.set_error(err);
                        } else {
                            match self.go_to_edit_mods_menu() {
                                Ok(n) => return n,
                                Err(err) => self.set_error(err),
                            }
                        }
                    }
                }
                Err(err) => self.set_error(err),
            },
            EditPresetsMessage::Load => return self.load_preset(),
            EditPresetsMessage::LoadComplete(result) => {
                if let Err(err) = result {
                    self.set_error(err);
                } else {
                    match self.go_to_edit_mods_menu() {
                        Ok(cmd) => return cmd,
                        Err(err) => self.set_error(err),
                    }
                }
            }
            EditPresetsMessage::RecommendedModCheck(result) => {
                iflet_manage_preset!(self, Recommended, mods, error, {
                    match result {
                        Ok(n) => {
                            *mods = Some(n.into_iter().map(|n| (true, n)).collect());
                        }
                        Err(err) => *error = Some(err),
                    }
                });
            }
            EditPresetsMessage::RecommendedToggle(idx, toggle) => {
                if let State::ManagePresets(MenuEditPresets {
                    inner:
                        MenuEditPresetsInner::Recommended {
                            mods: Some(mods), ..
                        },
                    ..
                }) = &mut self.state
                {
                    if let Some((t, _)) = mods.get_mut(idx) {
                        *t = toggle;
                    }
                }
            }
            EditPresetsMessage::RecommendedDownload => {
                if let State::ManagePresets(MenuEditPresets {
                    inner:
                        MenuEditPresetsInner::Recommended {
                            mods: Some(mods), ..
                        },
                    progress,
                    ..
                }) = &mut self.state
                {
                    let (sender, receiver) = std::sync::mpsc::channel();

                    *progress = Some(ProgressBar::with_recv(receiver));

                    return Command::perform(
                        ql_mod_manager::mod_manager::download_mods_w(
                            mods.iter()
                                .filter(|n| n.0)
                                .map(|n| n.1.id.to_owned())
                                .collect(),
                            self.selected_instance.clone().unwrap(),
                            sender,
                        ),
                        |n| Message::EditPresets(EditPresetsMessage::RecommendedDownloadEnd(n)),
                    );
                }
            }
            EditPresetsMessage::RecommendedDownloadEnd(result) => {
                if let Err(err) = result {
                    self.set_error(err);
                } else {
                    match self.go_to_edit_mods_menu_without_update_check() {
                        Ok(n) => return n,
                        Err(err) => self.set_error(err),
                    }
                }
            }
        }
        Command::none()
    }
}
