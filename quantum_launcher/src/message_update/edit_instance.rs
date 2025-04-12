use iced::Task;
use ql_core::{err, IntoIoError, IntoStringError};

use crate::{
    launcher_state::{get_entries, EditInstanceMessage, Launcher, MenuLaunch, Message, State},
    message_handler::format_memory,
};

impl Launcher {
    pub fn update_edit_instance(
        &mut self,
        message: EditInstanceMessage,
    ) -> Result<Task<Message>, String> {
        match message {
            EditInstanceMessage::JavaOverride(n) => {
                if let State::Launch(MenuLaunch {
                    edit_instance: Some(menu),
                    ..
                }) = &mut self.state
                {
                    menu.config.java_override = Some(n);
                }
            }
            EditInstanceMessage::MemoryChanged(new_slider_value) => {
                if let State::Launch(MenuLaunch {
                    edit_instance: Some(menu),
                    ..
                }) = &mut self.state
                {
                    menu.slider_value = new_slider_value;
                    menu.config.ram_in_mb = 2f32.powf(new_slider_value) as usize;
                    menu.slider_text = format_memory(menu.config.ram_in_mb);
                }
            }
            EditInstanceMessage::LoggingToggle(t) => {
                if let State::Launch(MenuLaunch {
                    edit_instance: Some(menu),
                    ..
                }) = &mut self.state
                {
                    menu.config.enable_logger = Some(t);
                }
            }
            EditInstanceMessage::CloseLauncherToggle(t) => {
                if let State::Launch(MenuLaunch {
                    edit_instance: Some(menu),
                    ..
                }) = &mut self.state
                {
                    menu.config.close_on_start = Some(t);
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
            EditInstanceMessage::RenameEdit(n) => {
                if let State::Launch(MenuLaunch {
                    edit_instance: Some(menu),
                    ..
                }) = &mut self.state
                {
                    menu.instance_name = n;
                }
            }
            EditInstanceMessage::RenameApply => return self.rename_instance(),
            EditInstanceMessage::ConfigSaved(res) => res?,
        }
        Ok(Task::none())
    }

    fn rename_instance(&mut self) -> Result<Task<Message>, String> {
        let State::Launch(MenuLaunch {
            edit_instance: Some(menu),
            ..
        }) = &mut self.state
        else {
            return Ok(Task::none());
        };
        let mut disallowed = vec![
            '/', '\\', ':', '*', '?', '"', '<', '>', '|', '\'', '\0', '\u{7F}',
        ];

        disallowed.extend('\u{1}'..='\u{1F}');

        // Remove disallowed characters

        let mut instance_name = menu.instance_name.clone();
        instance_name.retain(|c| !disallowed.contains(&c));
        let instance_name = instance_name.trim();

        if instance_name.is_empty() {
            err!("New name is empty or invalid");
            return Ok(Task::none());
        }

        if menu.old_instance_name == menu.instance_name {
            // Don't waste time talking to OS
            // and "renaming" instance if nothing has changed.
            Ok(Task::none())
        } else {
            let instances_dir =
                self.dir
                    .join(if self.selected_instance.as_ref().unwrap().is_server() {
                        "servers"
                    } else {
                        "instances"
                    });

            let old_path = instances_dir.join(&menu.old_instance_name);
            let new_path = instances_dir.join(&menu.instance_name);

            menu.old_instance_name = menu.instance_name.clone();
            if let Some(n) = &mut self.selected_instance {
                n.set_name(&menu.instance_name);
            }
            std::fs::rename(&old_path, &new_path)
                .path(&old_path)
                .strerr()?;

            Ok(Task::perform(
                get_entries(
                    match self.selected_instance.as_ref().unwrap() {
                        ql_core::InstanceSelection::Instance(_) => "instances",
                        ql_core::InstanceSelection::Server(_) => "servers",
                    }
                    .to_owned(),
                    false,
                ),
                Message::CoreListLoaded,
            ))
        }
    }

    fn e_java_arg_add(&mut self) {
        if let State::Launch(MenuLaunch {
            edit_instance: Some(menu),
            ..
        }) = &mut self.state
        {
            menu.config
                .java_args
                .get_or_insert_with(Vec::new)
                .push(String::new());
        }
    }

    fn e_java_arg_edit(&mut self, msg: String, idx: usize) {
        let State::Launch(MenuLaunch {
            edit_instance: Some(menu),
            ..
        }) = &mut self.state
        else {
            return;
        };
        let Some(args) = menu.config.java_args.as_mut() else {
            return;
        };
        add_to_arguments_list(msg, args, idx);
    }

    fn e_java_arg_delete(&mut self, idx: usize) {
        if let State::Launch(MenuLaunch {
            edit_instance: Some(menu),
            ..
        }) = &mut self.state
        {
            if let Some(args) = &mut menu.config.java_args {
                args.remove(idx);
            }
        }
    }

    fn e_game_arg_add(&mut self) {
        if let State::Launch(MenuLaunch {
            edit_instance: Some(menu),
            ..
        }) = &mut self.state
        {
            menu.config
                .game_args
                .get_or_insert_with(Vec::new)
                .push(String::new());
        }
    }

    fn e_game_arg_edit(&mut self, msg: String, idx: usize) {
        let State::Launch(MenuLaunch {
            edit_instance: Some(menu),
            ..
        }) = &mut self.state
        else {
            return;
        };
        let Some(args) = &mut menu.config.game_args else {
            return;
        };
        add_to_arguments_list(msg, args, idx);
    }

    fn e_game_arg_delete(&mut self, idx: usize) {
        if let State::Launch(MenuLaunch {
            edit_instance: Some(menu),
            ..
        }) = &mut self.state
        {
            if let Some(args) = &mut menu.config.game_args {
                args.remove(idx);
            }
        }
    }

    fn e_java_arg_shift_up(&mut self, idx: usize) {
        let State::Launch(MenuLaunch {
            edit_instance: Some(menu),
            ..
        }) = &mut self.state
        else {
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
        let State::Launch(MenuLaunch {
            edit_instance: Some(menu),
            ..
        }) = &mut self.state
        else {
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
        let State::Launch(MenuLaunch {
            edit_instance: Some(menu),
            ..
        }) = &mut self.state
        else {
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
        let State::Launch(MenuLaunch {
            edit_instance: Some(menu),
            ..
        }) = &mut self.state
        else {
            return;
        };
        let Some(args) = &mut menu.config.game_args else {
            return;
        };
        if idx + 1 < args.len() {
            args.swap(idx, idx + 1);
        }
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
