use std::{
    collections::HashSet,
    ffi::OsStr,
    path::{Path, PathBuf},
    process::ExitStatus,
    sync::{Arc, Mutex},
};

use iced::Task;
use ql_core::{
    err, json::instance_config::InstanceConfigJson, InstanceSelection, IntoIoError, IntoJsonError,
    IntoStringError, JsonFileError,
};
use ql_instances::{AccountData, ReadError};
use ql_mod_manager::{loaders, store::ModIndex};
use tokio::process::Child;

use crate::{
    get_entries,
    launcher_state::{
        ClientProcess, EditPresetsMessage, ManageModsMessage, MenuEditInstance, MenuEditMods,
        MenuInstallForge, MenuLaunch, MenuLauncherUpdate, ProgressBar, SelectedState, State,
        NEW_ACCOUNT_NAME, OFFLINE_ACCOUNT_NAME,
    },
    Launcher, Message, ServerProcess,
};

pub const SIDEBAR_DRAG_LEEWAY: f32 = 10.0;
pub const SIDEBAR_LIMIT_RIGHT: u16 = 300;
pub const SIDEBAR_LIMIT_LEFT: f32 = 135.0;

mod iced_event;

impl Launcher {
    pub fn launch_game(&mut self, account_data: Option<AccountData>) -> Task<Message> {
        let selected_instance = self.selected_instance.as_ref().unwrap().get_name();
        let username = if let Some(account_data) = &account_data {
            // Microsoft account
            account_data.username.clone()
        } else {
            // Offline username
            self.config.username.clone()
        };

        let (sender, receiver) = std::sync::mpsc::channel();
        self.java_recv = Some(ProgressBar::with_recv(receiver));

        if let Some(log) = self.client_logs.get_mut(selected_instance) {
            log.log.clear();
        }

        let instance_name = selected_instance.to_owned();
        Task::perform(
            async move {
                ql_instances::launch(instance_name, username, Some(sender), account_data)
                    .await
                    .strerr()
            },
            Message::LaunchEnd,
        )
    }

    pub fn finish_launching(&mut self, result: Result<Arc<Mutex<Child>>, String>) -> Task<Message> {
        self.java_recv = None;
        match result {
            Ok(child) => {
                let Some(InstanceSelection::Instance(selected_instance)) =
                    self.selected_instance.clone()
                else {
                    err!("Game Launched, but unknown instance!\n          This is a bug, please report it if found.");
                    return Task::none();
                };
                if let (Some(stdout), Some(stderr)) = {
                    let mut child = child.lock().unwrap();
                    (child.stdout.take(), child.stderr.take())
                } {
                    let (sender, receiver) = std::sync::mpsc::channel();

                    self.client_processes.insert(
                        selected_instance.clone(),
                        ClientProcess {
                            child: child.clone(),
                            receiver: Some(receiver),
                        },
                    );

                    return Task::perform(
                        async move {
                            let result = ql_instances::read_logs(
                                stdout,
                                stderr,
                                child,
                                sender,
                                selected_instance.clone(),
                            )
                            .await;

                            match result {
                                Err(ReadError::Io(io))
                                    if io.kind() == std::io::ErrorKind::InvalidData =>
                                {
                                    err!("Minecraft log contains invalid unicode! Stopping the logging...\n(note: the game will continue to run despite the next message)");
                                    Ok((ExitStatus::default(), selected_instance))
                                }
                                _ => result.strerr(),
                            }
                        },
                        Message::LaunchEndedLog,
                    );
                }
                self.client_processes.insert(
                    selected_instance.clone(),
                    ClientProcess {
                        child: child.clone(),
                        receiver: None,
                    },
                );
            }
            Err(err) => self.set_error(err),
        }
        Task::none()
    }

    pub fn delete_instance_confirm(&mut self) -> Task<Message> {
        if let State::ConfirmAction { .. } = &self.state {
            let selected_instance = self.selected_instance.as_ref().unwrap();
            let deleted_instance_dir = selected_instance.get_instance_path();
            if let Err(err) = std::fs::remove_dir_all(&deleted_instance_dir) {
                self.set_error(err);
                return Task::none();
            }

            self.selected_instance = None;
            return self.go_to_launch_screen(Some("Deleted Instance".to_owned()));
        }
        Task::none()
    }

    pub fn edit_instance(&mut self) -> Result<(), JsonFileError> {
        let State::Launch(MenuLaunch { edit_instance, .. }) = &mut self.state else {
            return Ok(());
        };

        let Some(selected_instance) = self.selected_instance.as_ref() else {
            return Ok(());
        };

        let config_path = selected_instance.get_instance_path().join("config.json");

        let config_json = std::fs::read_to_string(&config_path).path(config_path)?;
        let config_json: InstanceConfigJson =
            serde_json::from_str(&config_json).json(config_json)?;

        let slider_value = f32::log2(config_json.ram_in_mb as f32);
        let memory_mb = config_json.ram_in_mb;

        let instance_name = selected_instance.get_name();

        *edit_instance = Some(MenuEditInstance {
            config: config_json,
            slider_value,
            instance_name: instance_name.to_owned(),
            old_instance_name: instance_name.to_owned(),
            slider_text: format_memory(memory_mb),
        });
        Ok(())
    }

    pub fn go_to_edit_mods_menu_without_update_check(
        &mut self,
    ) -> Result<Task<Message>, JsonFileError> {
        let selected_instance = self.selected_instance.as_ref().unwrap();
        let config_path = selected_instance.get_instance_path().join("config.json");

        let config_json = std::fs::read_to_string(&config_path).path(config_path)?;
        let config_json: InstanceConfigJson =
            serde_json::from_str(&config_json).json(config_json)?;

        match ModIndex::get_s(selected_instance).strerr() {
            Ok(idx) => {
                let locally_installed_mods =
                    MenuEditMods::update_locally_installed_mods(&idx, selected_instance);

                self.state = State::EditMods(MenuEditMods {
                    config: config_json,
                    mods: idx,
                    selected_mods: HashSet::new(),
                    sorted_mods_list: Vec::new(),
                    selected_state: SelectedState::None,
                    available_updates: Vec::new(),
                    mod_update_progress: None,
                    locally_installed_mods: HashSet::new(),
                    drag_and_drop_hovered: false,
                    update_check_handle: None,
                });

                Ok(locally_installed_mods)
            }
            Err(err) => {
                self.set_error(err);
                Ok(Task::none())
            }
        }
    }

    pub fn go_to_edit_mods_menu(&mut self) -> Result<Task<Message>, JsonFileError> {
        let selected_instance = self.selected_instance.as_ref().unwrap();
        let config_path = selected_instance.get_instance_path().join("config.json");

        let config_json = std::fs::read_to_string(&config_path).path(config_path)?;
        let config_json: InstanceConfigJson =
            serde_json::from_str(&config_json).json(config_json)?;

        let is_vanilla = config_json.mod_type == "Vanilla";

        match ModIndex::get_s(selected_instance).strerr() {
            Ok(idx) => {
                let locally_installed_mods =
                    MenuEditMods::update_locally_installed_mods(&idx, selected_instance);

                let (update_cmd, update_check_handle) = if is_vanilla {
                    (Task::none(), None)
                } else {
                    let (a, b) = Task::perform(
                        ql_mod_manager::store::check_for_updates(selected_instance.clone()),
                        |n| Message::ManageMods(ManageModsMessage::UpdateCheckResult(n.strerr())),
                    )
                    .abortable();
                    (a, Some(b.abort_on_drop()))
                };

                self.state = State::EditMods(MenuEditMods {
                    config: config_json,
                    mods: idx,
                    selected_mods: HashSet::new(),
                    sorted_mods_list: Vec::new(),
                    selected_state: SelectedState::None,
                    available_updates: Vec::new(),
                    mod_update_progress: None,
                    locally_installed_mods: HashSet::new(),
                    drag_and_drop_hovered: false,
                    update_check_handle,
                });

                return Ok(Task::batch([locally_installed_mods, update_cmd]));
            }
            Err(err) => {
                self.set_error(err);
            }
        }
        Ok(Task::none())
    }

    pub fn set_game_crashed(&mut self, status: std::process::ExitStatus, name: &str) {
        if let State::Launch(MenuLaunch { message, .. }) = &mut self.state {
            let has_crashed = !status.success();
            if has_crashed {
                *message =
                    format!("Game Crashed with code: {status}\nCheck Logs for more information");
            }
            if let Some(log) = self.client_logs.get_mut(name) {
                log.has_crashed = has_crashed;
            }
        }
    }

    pub fn update_mods(&mut self) -> Task<Message> {
        if let State::EditMods(menu) = &mut self.state {
            let updates = menu
                .available_updates
                .clone()
                .into_iter()
                .map(|(n, _, _)| n)
                .collect();
            let (sender, receiver) = std::sync::mpsc::channel();
            menu.mod_update_progress = Some(ProgressBar::with_recv_and_msg(
                receiver,
                "Deleting Mods".to_owned(),
            ));
            let selected_instance = self.selected_instance.clone().unwrap();
            Task::perform(
                ql_mod_manager::store::apply_updates(selected_instance, updates, Some(sender)),
                |n| Message::ManageMods(ManageModsMessage::UpdateModsFinished(n.strerr())),
            )
        } else {
            Task::none()
        }
    }

    pub fn go_to_server_manage_menu(&mut self, message: Option<String>) -> Task<Message> {
        if let State::Launch(menu) = &mut self.state {
            menu.is_viewing_server = true;
            if let Some(message) = message {
                menu.message = message;
            }
        } else {
            let mut menu_launch = match message {
                Some(message) => MenuLaunch::with_message(message),
                None => MenuLaunch::default(),
            };
            menu_launch.is_viewing_server = true;
            if let Some(width) = self.config.sidebar_width {
                menu_launch.sidebar_width = width as u16;
            }
            self.state = State::Launch(menu_launch);
        }
        Task::perform(
            get_entries("servers".to_owned(), true),
            Message::CoreListLoaded,
        )
    }

    pub fn install_forge(&mut self, is_neoforge: bool) -> Task<Message> {
        let (f_sender, f_receiver) = std::sync::mpsc::channel();
        let (j_sender, j_receiver) = std::sync::mpsc::channel();

        let instance_selection = self.selected_instance.clone().unwrap();
        let command = Task::perform(
            async move {
                if is_neoforge {
                    loaders::neoforge::install(instance_selection, Some(f_sender), Some(j_sender))
                        .await
                } else {
                    loaders::forge::install(instance_selection, Some(f_sender), Some(j_sender))
                        .await
                }
                .strerr()
            },
            Message::InstallForgeEnd,
        );

        self.state = State::InstallForge(MenuInstallForge {
            forge_progress: ProgressBar::with_recv(f_receiver),
            java_progress: ProgressBar::with_recv(j_receiver),
            is_java_getting_installed: false,
        });
        command
    }

    pub fn add_server_to_processes(
        &mut self,
        child: Arc<std::sync::Mutex<tokio::process::Child>>,
        is_classic_server: bool,
    ) -> Task<Message> {
        let Some(InstanceSelection::Server(selected_server)) = &self.selected_instance else {
            err!("Launched server but can't identify which one! This is a bug, please report it");
            return Task::none();
        };
        if let (Some(stdout), Some(stderr), Some(stdin)) = {
            let mut child = child.lock().unwrap();
            (child.stdout.take(), child.stderr.take(), child.stdin.take())
        } {
            let (sender, receiver) = std::sync::mpsc::channel();

            self.server_processes.insert(
                selected_server.clone(),
                ServerProcess {
                    child: child.clone(),
                    receiver: Some(receiver),
                    stdin: Some(stdin),
                    is_classic_server,
                    has_issued_stop_command: false,
                },
            );

            let selected_server = selected_server.clone();
            return Task::perform(
                async move {
                    ql_servers::read_logs(stdout, stderr, child, sender, selected_server)
                        .await
                        .strerr()
                },
                Message::ServerManageEndedLog,
            );
        }

        self.server_processes.insert(
            selected_server.clone(),
            ServerProcess {
                child: child.clone(),
                receiver: None,
                stdin: None,
                is_classic_server,
                has_issued_stop_command: false,
            },
        );
        Task::none()
    }

    pub fn go_to_main_menu_with_message(
        &mut self,
        message: Option<impl ToString>,
    ) -> Task<Message> {
        let message = message.map(|n| n.to_string());
        match &self.selected_instance {
            None | Some(InstanceSelection::Instance(_)) => self.go_to_launch_screen(message),
            Some(InstanceSelection::Server(_)) => self.go_to_server_manage_menu(message),
        }
    }

    pub fn get_selected_instance_dir(&self) -> Option<PathBuf> {
        Some(self.selected_instance.as_ref()?.get_instance_path())
    }

    pub fn get_selected_dot_minecraft_dir(&self) -> Option<PathBuf> {
        Some(self.selected_instance.as_ref()?.get_dot_minecraft_path())
    }

    fn load_modpack_from_path(&mut self, path: PathBuf) -> Task<Message> {
        let (sender, receiver) = std::sync::mpsc::channel();

        self.state = State::ImportModpack(ProgressBar::with_recv(receiver));

        Task::perform(
            ql_mod_manager::add_files(
                self.selected_instance.clone().unwrap(),
                vec![path],
                Some(sender),
            ),
            |n| Message::ManageMods(ManageModsMessage::AddFileDone(n.strerr())),
        )
    }

    fn load_jar_from_path(&mut self, path: &PathBuf, filename: &str) {
        let selected_instance = self.selected_instance.as_ref().unwrap();
        let new_path = selected_instance
            .get_dot_minecraft_path()
            .join("mods")
            .join(filename);
        if *path != new_path {
            if let Err(err) = std::fs::copy(path, &new_path) {
                err!("Couldn't drag and drop mod file in: {err}");
            }
        }
    }

    pub fn load_qmp_from_path(&mut self, path: &Path) -> Task<Message> {
        let file = match std::fs::read(path) {
            Ok(n) => n,
            Err(err) => {
                err!("Couldn't drag and drop preset file: {err}");
                return Task::none();
            }
        };
        match tokio::runtime::Handle::current().block_on(ql_mod_manager::PresetJson::load(
            self.selected_instance.clone().unwrap(),
            file,
        )) {
            Ok(mods) => {
                let (sender, receiver) = std::sync::mpsc::channel();
                if let State::ManagePresets(menu) = &mut self.state {
                    menu.progress = Some(ProgressBar::with_recv(receiver));
                }
                let instance_name = self.selected_instance.clone().unwrap();
                Task::perform(
                    ql_mod_manager::store::download_mods_bulk(mods, instance_name, Some(sender)),
                    |n| Message::EditPresets(EditPresetsMessage::LoadComplete(n.strerr())),
                )
            }
            Err(err) => {
                self.set_error(err);
                Task::none()
            }
        }
    }

    fn set_drag_and_drop_hover(&mut self, is_hovered: bool) {
        if let State::EditMods(menu) = &mut self.state {
            menu.drag_and_drop_hovered = is_hovered;
        } else if let State::ManagePresets(menu) = &mut self.state {
            menu.drag_and_drop_hovered = is_hovered;
        } else if let State::EditJarMods(menu) = &mut self.state {
            menu.drag_and_drop_hovered = is_hovered;
        }
    }

    pub fn update_download_start(&mut self) -> Task<Message> {
        if let State::UpdateFound(MenuLauncherUpdate { url, progress, .. }) = &mut self.state {
            let (sender, update_receiver) = std::sync::mpsc::channel();
            *progress = Some(ProgressBar::with_recv_and_msg(
                update_receiver,
                "Starting Update".to_owned(),
            ));

            let url = url.clone();

            Task::perform(
                async move {
                    ql_instances::install_launcher_update(url, sender)
                        .await
                        .strerr()
                },
                Message::UpdateDownloadEnd,
            )
        } else {
            Task::none()
        }
    }

    pub fn kill_selected_instance(&mut self) -> Task<Message> {
        let Some(selected_instance) = &self.selected_instance else {
            return Task::none();
        };
        if let Some(process) = self.client_processes.remove(selected_instance.get_name()) {
            Task::perform(
                {
                    async move {
                        let mut child = process.child.lock().unwrap();
                        child.start_kill().strerr()
                    }
                },
                Message::LaunchKillEnd,
            )
        } else {
            Task::none()
        }
    }

    pub fn go_to_delete_instance_menu(&mut self) {
        self.state = State::ConfirmAction {
            msg1: format!(
                "delete the instance {}",
                self.selected_instance.as_ref().unwrap().get_name()
            ),
            msg2: "All your data, including worlds, will be lost".to_owned(),
            yes: Message::DeleteInstance,
            no: Message::LaunchScreenOpen {
                message: None,
                clear_selection: false,
            },
        };
    }

    pub fn launch_start(&mut self) -> Task<Message> {
        let Some(selected_instance) = &self.selected_instance else {
            return Task::none();
        };

        if let Some(account) = &self.accounts_selected {
            if account == OFFLINE_ACCOUNT_NAME
                && (self.config.username.is_empty() || self.config.username.contains(' '))
            {
                return Task::none();
            }
        }

        let is_alive = match selected_instance {
            InstanceSelection::Instance(name) => self.client_processes.contains_key(name),
            InstanceSelection::Server(name) => self.server_processes.contains_key(name),
        };
        if is_alive {
            return Task::none();
        }

        let account_data = if let Some(account) = &self.accounts_selected {
            if account == NEW_ACCOUNT_NAME || account == OFFLINE_ACCOUNT_NAME {
                None
            } else {
                self.accounts.get(account).cloned()
            }
        } else {
            None
        };
        if let Some(account) = &account_data {
            if account.access_token.is_none() || account.needs_refresh {
                return self.account_refresh(account);
            }
        }

        // If the user is loading an existing login from disk
        // then first refresh the tokens

        // Or, if the account is freshly added,
        // just directly launch the game.
        self.launch_game(account_data)
    }
}

pub async fn get_locally_installed_mods(
    selected_instance: PathBuf,
    blacklist: Vec<String>,
) -> HashSet<String> {
    let mods_dir_path = selected_instance.join("mods");

    let Ok(mut dir) = tokio::fs::read_dir(&mods_dir_path).await else {
        err!("Error reading mods directory");
        return HashSet::new();
    };
    let mut set = HashSet::new();
    while let Ok(Some(entry)) = dir.next_entry().await {
        let path = entry.path();
        let Some(file_name) = path.file_name().and_then(OsStr::to_str) else {
            continue;
        };
        if blacklist.contains(&file_name.to_owned()) {
            continue;
        }
        let Some(extension) = path.extension() else {
            continue;
        };
        if extension == "jar" || extension == "disabled" {
            set.insert(file_name.to_owned());
        }
    }
    set
}

pub fn format_memory(memory_bytes: usize) -> String {
    const MB_TO_GB: usize = 1024;

    if memory_bytes >= MB_TO_GB {
        format!("{:.2} GB", memory_bytes as f64 / MB_TO_GB as f64)
    } else {
        format!("{memory_bytes} MB")
    }
}
