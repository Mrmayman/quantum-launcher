use iced::Command;

use crate::{
    launcher_state::{EditInstanceMessage, Launcher, Message, State},
    message_handler::format_memory,
};

impl Launcher {
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
