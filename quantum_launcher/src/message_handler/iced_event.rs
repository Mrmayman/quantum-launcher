use std::ffi::OsStr;

use iced::{
    keyboard::{key::Named, Key},
    Task,
};
use ql_core::{err, info, info_no_log, jarmod::JarMod, InstanceSelection};

use crate::state::{
    Launcher, MenuCreateInstance, MenuEditMods, MenuInstallFabric, MenuInstallOptifine, MenuLaunch,
    MenuLauncherUpdate, MenuServerCreate, Message, State,
};

use super::{SIDEBAR_DRAG_LEEWAY, SIDEBAR_LIMIT_LEFT, SIDEBAR_LIMIT_RIGHT};

impl Launcher {
    pub fn iced_event(&mut self, event: iced::Event, status: iced::event::Status) -> Task<Message> {
        if let State::Launch(MenuLaunch { sidebar_width, .. }) = &mut self.state {
            self.config.sidebar_width = Some(u32::from(*sidebar_width));

            if self.window_size.0 > f32::from(SIDEBAR_LIMIT_RIGHT)
                && *sidebar_width > self.window_size.0 as u16 - SIDEBAR_LIMIT_RIGHT
            {
                *sidebar_width = self.window_size.0 as u16 - SIDEBAR_LIMIT_RIGHT;
            }

            if self.window_size.0 > SIDEBAR_LIMIT_LEFT && *sidebar_width < SIDEBAR_LIMIT_LEFT as u16
            {
                *sidebar_width = SIDEBAR_LIMIT_LEFT as u16;
            }
        }

        match event {
            iced::Event::Window(event) => match event {
                iced::window::Event::CloseRequested => {
                    info_no_log!("Shutting down launcher (1)");
                    std::process::exit(0);
                }
                iced::window::Event::Closed => {
                    info!("Shutting down launcher (2)");
                }
                iced::window::Event::Resized(size) => {
                    self.window_size = (size.width, size.height);
                }
                iced::window::Event::FileHovered(_) => {
                    self.set_drag_and_drop_hover(true);
                }
                iced::window::Event::FilesHoveredLeft => {
                    self.set_drag_and_drop_hover(false);
                }
                iced::window::Event::FileDropped(path) => {
                    self.set_drag_and_drop_hover(false);

                    if let (Some(extension), Some(filename)) = (
                        path.extension().map(OsStr::to_ascii_lowercase),
                        path.file_name().and_then(OsStr::to_str),
                    ) {
                        if let State::EditMods(_) = &self.state {
                            if extension == "jar" || extension == "disabled" {
                                self.load_jar_from_path(&path, filename);
                            } else if extension == "qmp" {
                                return self.load_qmp_from_path(&path);
                            } else if extension == "zip" || extension == "mrpack" {
                                return self.load_modpack_from_path(path);
                            }
                        } else if let State::ManagePresets(_) = &self.state {
                            if extension == "qmp" {
                                return self.load_qmp_from_path(&path);
                            } else if extension == "zip" || extension == "mrpack" {
                                return self.load_modpack_from_path(path);
                            }
                        } else if let State::EditJarMods(menu) = &mut self.state {
                            if extension == "jar" || extension == "zip" {
                                let selected_instance = self.selected_instance.as_ref().unwrap();
                                let new_path = selected_instance
                                    .get_instance_path()
                                    .join("jarmods")
                                    .join(filename);
                                if path != new_path {
                                    if let Err(err) = std::fs::copy(&path, &new_path) {
                                        err!("Couldn't drag and drop mod file in: {err}");
                                    } else if !menu
                                        .jarmods
                                        .mods
                                        .iter()
                                        .any(|n| n.filename == filename)
                                    {
                                        menu.jarmods.mods.push(JarMod {
                                            filename: filename.to_owned(),
                                            enabled: true,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
                iced::window::Event::RedrawRequested(_)
                | iced::window::Event::Moved { .. }
                | iced::window::Event::Opened { .. }
                | iced::window::Event::Focused
                | iced::window::Event::Unfocused => {}
            },
            iced::Event::Keyboard(event) => match event {
                iced::keyboard::Event::KeyPressed {
                    key,
                    // location,
                    modifiers,
                    ..
                } => {
                    if let iced::event::Status::Ignored = status {
                        if let Key::Named(Named::Escape) = key {
                            return self.key_escape_back();
                        }
                        if let Key::Named(Named::ArrowUp) = key {
                            return self.key_change_selected_instance(false);
                        } else if let Key::Named(Named::ArrowDown) = key {
                            return self.key_change_selected_instance(true);
                        } else if let Key::Named(Named::Enter) = key {
                            if modifiers.command() {
                                return self.launch_start();
                            }
                        } else if let Key::Named(Named::Backspace) = key {
                            match self.selected_instance.clone() {
                                Some(InstanceSelection::Instance(_)) => {
                                    return self.kill_selected_instance();
                                }
                                Some(InstanceSelection::Server(server)) => {
                                    self.kill_selected_server(&server);
                                }
                                None => {}
                            }
                        } else if let Key::Character(ch) = &key {
                            let instances_not_running = self.client_processes.is_empty()
                                && self.server_processes.is_empty();

                            if ch == "q" && modifiers.command() && instances_not_running {
                                info_no_log!("CTRL-Q pressed, closing launcher...");
                                std::process::exit(1);
                            }
                        }

                        self.keys_pressed.insert(key);
                    } else {
                        // FUTURE
                    }
                }
                iced::keyboard::Event::KeyReleased { key, .. } => {
                    self.keys_pressed.remove(&key);
                }
                iced::keyboard::Event::ModifiersChanged(_) => {}
            },
            iced::Event::Mouse(mouse) => match mouse {
                iced::mouse::Event::CursorMoved { position } => {
                    self.mouse_pos.0 = position.x;
                    self.mouse_pos.1 = position.y;

                    if let State::Launch(MenuLaunch {
                        sidebar_width,
                        sidebar_dragging: true,
                        ..
                    }) = &mut self.state
                    {
                        if self.mouse_pos.0 < SIDEBAR_LIMIT_LEFT {
                            *sidebar_width = SIDEBAR_LIMIT_LEFT as u16;
                        } else if (self.mouse_pos.0 + f32::from(SIDEBAR_LIMIT_RIGHT)
                            > self.window_size.0)
                            && self.window_size.0 as u16 > SIDEBAR_LIMIT_RIGHT
                        {
                            *sidebar_width = self.window_size.0 as u16 - SIDEBAR_LIMIT_RIGHT;
                        } else {
                            *sidebar_width = self.mouse_pos.0 as u16;
                        }
                    }
                }
                iced::mouse::Event::ButtonPressed(button) => {
                    if let (State::Launch(menu), iced::mouse::Button::Left) =
                        (&mut self.state, button)
                    {
                        let difference = self.mouse_pos.0 - f32::from(menu.sidebar_width);
                        if difference > 0.0 && difference < SIDEBAR_DRAG_LEEWAY {
                            menu.sidebar_dragging = true;
                        }
                    }
                }
                iced::mouse::Event::ButtonReleased(button) => {
                    if let (State::Launch(menu), iced::mouse::Button::Left) =
                        (&mut self.state, button)
                    {
                        menu.sidebar_dragging = false;
                    }
                }
                iced::mouse::Event::WheelScrolled { delta } => {
                    if let iced::event::Status::Ignored = status {
                        if self
                            .keys_pressed
                            .contains(&iced::keyboard::Key::Named(Named::Control))
                        {
                            match delta {
                                iced::mouse::ScrollDelta::Lines { y, .. }
                                | iced::mouse::ScrollDelta::Pixels { y, .. } => {
                                    let new_scale =
                                        self.config.ui_scale.unwrap_or(1.0) + (f64::from(y) / 5.0);
                                    let new_scale = new_scale.clamp(0.5, 2.0);
                                    self.config.ui_scale = Some(new_scale);
                                    if let State::LauncherSettings(menu) = &mut self.state {
                                        menu.temp_scale = new_scale;
                                    }
                                }
                            }
                        }
                    }
                }
                iced::mouse::Event::CursorEntered | iced::mouse::Event::CursorLeft => {}
            },
            iced::Event::Touch(_) => {}
        }
        Task::none()
    }

    fn key_escape_back(&mut self) -> Task<Message> {
        let mut should_return_to_main_screen = false;
        let mut should_return_to_mods_screen = false;
        let mut should_return_to_download_screen = false;

        match &self.state {
            State::ChangeLog
            | State::EditMods(MenuEditMods {
                mod_update_progress: None,
                ..
            })
            | State::Create(
                MenuCreateInstance::Loading { .. }
                | MenuCreateInstance::Loaded { progress: None, .. },
            )
            | State::ServerCreate(
                MenuServerCreate::LoadingList | MenuServerCreate::Loaded { .. },
            )
            | State::Error { .. }
            | State::UpdateFound(MenuLauncherUpdate { progress: None, .. })
            | State::LauncherSettings(_)
            | State::AccountLogin { .. }
            | State::Welcome(_) => {
                should_return_to_main_screen = true;
            }
            State::ConfirmAction { no, .. } => return self.update(no.clone()),

            State::InstallOptifine(MenuInstallOptifine {
                optifine_install_progress: None,
                java_install_progress: None,
                ..
            })
            | State::InstallFabric(MenuInstallFabric::Loaded { progress: None, .. })
            | State::EditJarMods(_) => {
                should_return_to_mods_screen = true;
            }
            State::ModsDownload(menu) if menu.opened_mod.is_some() => {
                should_return_to_download_screen = true;
            }
            State::ModsDownload(menu) if menu.mods_download_in_progress.is_empty() => {
                should_return_to_mods_screen = true;
            }
            State::ExportInstance(_) => {
                // TODO
            }
            State::InstallPaper
            | State::InstallForge(_)
            | State::InstallJava
            | State::InstallOptifine(_)
            | State::UpdateFound(_)
            | State::InstallFabric(_)
            | State::EditMods(_)
            | State::Create(_)
            | State::ManagePresets(_)
            | State::ModsDownload(_)
            | State::ServerCreate(_)
            | State::GenericMessage(_)
            | State::AccountLoginProgress(_)
            | State::ImportModpack(_)
            | State::CurseforgeManualDownload(_)
            | State::Launch(_) => {}
        }

        if should_return_to_main_screen {
            return self.go_to_launch_screen::<String>(None);
        }
        if should_return_to_mods_screen {
            match self.go_to_edit_mods_menu_without_update_check() {
                Ok(cmd) => return cmd,
                Err(err) => self.set_error(err),
            }
        }
        if should_return_to_download_screen {
            if let State::ModsDownload(menu) = &mut self.state {
                menu.opened_mod = None;
            }
        }

        Task::none()
    }

    fn key_change_selected_instance(&mut self, down: bool) -> Task<Message> {
        let State::Launch(menu) = &self.state else {
            return Task::none();
        };
        let list = if menu.is_viewing_server {
            &self.server_list
        } else {
            &self.client_list
        };

        let Some(list) = list else {
            return Task::none();
        };

        let idx = if let Some(selected_instance) = &mut self.selected_instance {
            if let Some(idx) = list
                .iter()
                .enumerate()
                .find_map(|(i, n)| (n == selected_instance.get_name()).then_some(i))
            {
                if down {
                    if idx + 1 < list.len() {
                        *selected_instance = InstanceSelection::new(
                            list.get(idx + 1).unwrap(),
                            menu.is_viewing_server,
                        );
                        idx + 1
                    } else {
                        idx
                    }
                } else if idx > 0 {
                    *selected_instance =
                        InstanceSelection::new(list.get(idx - 1).unwrap(), menu.is_viewing_server);
                    idx - 1
                } else {
                    idx
                }
            } else {
                debug_assert!(
                    false,
                    "Selected instance {selected_instance:?}, not found in list?"
                );
                0
            }
        } else {
            self.selected_instance = list
                .first()
                .map(|n| InstanceSelection::new(n, menu.is_viewing_server));
            0
        };

        let scroll_pos = idx as f32 / (list.len() as f32 - 1.0);
        let scroll_pos = scroll_pos * menu.sidebar_height;
        iced::widget::scrollable::scroll_to(
            iced::widget::scrollable::Id::new("MenuLaunch:sidebar"),
            iced::widget::scrollable::AbsoluteOffset {
                x: 0.0,
                y: scroll_pos,
            },
        )
    }
}
