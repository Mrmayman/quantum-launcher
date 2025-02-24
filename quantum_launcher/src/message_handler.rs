use std::{
    collections::HashSet,
    path::PathBuf,
    sync::{mpsc, Arc},
};

use chrono::Datelike;
use iced::{keyboard::Key, Task};
use ql_core::{
    err, file_utils, info,
    json::{instance_config::InstanceConfigJson, version::VersionDetails},
    DownloadProgress, InstanceSelection, IntoIoError, JsonFileError,
};
use ql_instances::{AccountData, GameLaunchResult, ListEntry};
use ql_mod_manager::{
    loaders,
    mod_manager::{Loader, ModIndex, ModVersion, RECOMMENDED_MODS},
};

use crate::{
    get_entries,
    launcher_state::{
        ClientProcess, CreateInstanceMessage, EditPresetsMessage, InstallModsMessage,
        MenuCreateInstance, MenuEditInstance, MenuEditMods, MenuEditPresets, MenuEditPresetsInner,
        MenuInstallFabric, MenuInstallForge, MenuInstallOptifine, MenuLaunch, MenuLauncherUpdate,
        MenuServerManage,
    },
    Launcher, ManageModsMessage, Message, ProgressBar, SelectedState, ServerProcess, State,
};

pub const SIDEBAR_DRAG_LEEWAY: f32 = 10.0;
pub const SIDEBAR_SQUISH_LIMIT: u16 = 300;

impl Launcher {
    pub fn set_username(&mut self, username: String) {
        self.config.as_mut().unwrap().username = username;
    }

    pub fn launch_game(&mut self, account_data: Option<AccountData>) -> Task<Message> {
        if let State::Launch(ref mut menu_launch) = self.state {
            let selected_instance = self.selected_instance.as_ref().unwrap().get_name();
            let username = if let Some(account_data) = &account_data {
                // Microsoft account
                account_data.username.clone()
            } else {
                // Offline username
                self.config.as_ref().unwrap().username.clone()
            };

            let (sender, receiver) = std::sync::mpsc::channel();
            self.java_recv = Some(ProgressBar::with_recv(receiver));

            let (asset_sender, asset_receiver) = std::sync::mpsc::channel();
            menu_launch.asset_recv = Some(asset_receiver);

            if let Some(log) = self.client_logs.get_mut(selected_instance) {
                log.log.clear();
            }

            return Task::perform(
                ql_instances::launch_w(
                    selected_instance.to_owned(),
                    username,
                    Some(sender),
                    Some(asset_sender),
                    account_data,
                ),
                Message::LaunchEnd,
            );
        }
        Task::none()
    }

    pub fn get_current_date_formatted() -> String {
        // Get the current date and time in UTC
        let now = chrono::Local::now();

        // Extract the day, month, and year
        let day = now.day();
        let month = now.format("%B").to_string(); // Full month name (e.g., "September")
        let year = now.year();

        // Return the formatted string
        format!("{day} {month} {year}")
    }

    pub fn finish_launching(&mut self, result: GameLaunchResult) -> Task<Message> {
        self.java_recv = None;
        if let State::Launch(menu) = &mut self.state {
            menu.asset_recv = None;
        }
        match result {
            GameLaunchResult::Ok(child) => {
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
                        ql_instances::read_logs_w(stdout, stderr, child, sender, selected_instance),
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
            GameLaunchResult::Err(err) => self.set_error(err),
        }
        Task::none()
    }

    pub fn go_to_create_screen(&mut self) -> Task<Message> {
        if let Some(versions) = self.client_version_list_cache.clone() {
            let combo_state = iced::widget::combo_box::State::new(versions.clone());
            self.state = State::Create(MenuCreateInstance::Loaded {
                instance_name: String::new(),
                selected_version: None,
                progress: None,
                download_assets: true,
                combo_state: Box::new(combo_state),
            });
            Task::none()
        } else {
            let (sender, receiver) = mpsc::channel();
            self.state = State::Create(MenuCreateInstance::Loading {
                progress_receiver: receiver,
                progress_number: 0.0,
            });
            Task::perform(ql_instances::list_versions(Some(Arc::new(sender))), |n| {
                Message::CreateInstance(CreateInstanceMessage::VersionsLoaded(n))
            })
        }
    }

    pub fn create_instance_finish_loading_versions_list(
        &mut self,
        result: Result<Vec<ListEntry>, String>,
    ) {
        match result {
            Ok(versions) => {
                self.client_version_list_cache = Some(versions.clone());
                let combo_state = iced::widget::combo_box::State::new(versions.clone());
                self.state = State::Create(MenuCreateInstance::Loaded {
                    instance_name: String::new(),
                    selected_version: None,
                    progress: None,
                    download_assets: true,
                    combo_state: Box::new(combo_state),
                });
            }
            Err(n) => self.set_error(n),
        }
    }

    pub fn select_created_instance_version(&mut self, entry: ListEntry) {
        if let State::Create(MenuCreateInstance::Loaded {
            selected_version, ..
        }) = &mut self.state
        {
            *selected_version = Some(entry);
        }
    }

    pub fn update_created_instance_name(&mut self, name: String) {
        if let State::Create(MenuCreateInstance::Loaded { instance_name, .. }) = &mut self.state {
            *instance_name = name;
        }
    }

    pub fn create_instance(&mut self) -> Task<Message> {
        if let State::Create(MenuCreateInstance::Loaded {
            progress,
            instance_name,
            download_assets,
            selected_version,
            ..
        }) = &mut self.state
        {
            let (sender, receiver) = mpsc::channel::<DownloadProgress>();
            *progress = Some(ProgressBar {
                num: 0.0,
                message: Some("Started download".to_owned()),
                receiver,
                progress: DownloadProgress::DownloadingJsonManifest,
            });

            // Create Instance asynchronously using iced Command.
            return Task::perform(
                ql_instances::create_instance_w(
                    instance_name.clone(),
                    selected_version.clone().unwrap(),
                    Some(sender),
                    *download_assets,
                ),
                |n| Message::CreateInstance(CreateInstanceMessage::End(n)),
            );
        }
        Task::none()
    }

    pub fn delete_selected_instance(&mut self) -> Task<Message> {
        if let State::ConfirmAction { .. } = &self.state {
            let selected_instance = self.selected_instance.as_ref().unwrap();
            let deleted_instance_dir = selected_instance.get_instance_path(&self.dir);
            if let Err(err) = std::fs::remove_dir_all(&deleted_instance_dir) {
                self.set_error(err);
                return Task::none();
            }

            self.selected_instance = None;
            return self.go_to_launch_screen(Some("Deleted Instance".to_owned()));
        }
        Task::none()
    }

    pub fn edit_instance(
        &mut self,
        selected_instance: &InstanceSelection,
    ) -> Result<(), JsonFileError> {
        let State::Launch(MenuLaunch { edit_instance, .. }) = &mut self.state else {
            return Ok(());
        };

        let config_path = selected_instance
            .get_instance_path(&self.dir)
            .join("config.json");

        let config_json = std::fs::read_to_string(&config_path).path(config_path)?;
        let config_json: InstanceConfigJson = serde_json::from_str(&config_json)?;

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

    pub async fn save_config_w(
        instance: InstanceSelection,
        config: InstanceConfigJson,
        dir: PathBuf,
    ) -> Result<(), String> {
        Self::save_config(instance, config, dir)
            .await
            .map_err(|n| n.to_string())
    }

    pub async fn save_config(
        instance: InstanceSelection,
        config: InstanceConfigJson,
        dir: PathBuf,
    ) -> Result<(), JsonFileError> {
        let mut config = config.clone();
        if config.enable_logger.is_none() {
            config.enable_logger = Some(true);
        }
        let config_path = instance.get_instance_path(&dir).join("config.json");

        let config_json = serde_json::to_string(&config)?;
        tokio::fs::write(&config_path, config_json)
            .await
            .path(config_path)?;
        Ok(())
    }

    pub fn go_to_edit_mods_menu_without_update_check(
        &mut self,
    ) -> Result<Task<Message>, JsonFileError> {
        let selected_instance = self.selected_instance.as_ref().unwrap();
        let config_path = selected_instance
            .get_instance_path(&self.dir)
            .join("config.json");

        let config_json = std::fs::read_to_string(&config_path).path(config_path)?;
        let config_json: InstanceConfigJson = serde_json::from_str(&config_json)?;

        match ModIndex::get_s(selected_instance).map_err(|err| err.to_string()) {
            Ok(idx) => {
                let locally_installed_mods =
                    MenuEditMods::update_locally_installed_mods(&idx, selected_instance, &self.dir);

                self.state = State::EditMods(MenuEditMods {
                    config: config_json,
                    mods: idx,
                    selected_mods: HashSet::new(),
                    sorted_mods_list: Vec::new(),
                    selected_state: SelectedState::None,
                    available_updates: Vec::new(),
                    mod_update_progress: None,
                    locally_installed_mods: HashSet::new(),
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
        let config_path = file_utils::get_instance_dir_s(selected_instance)?.join("config.json");

        let config_json = std::fs::read_to_string(&config_path).path(config_path)?;
        let config_json: InstanceConfigJson = serde_json::from_str(&config_json)?;

        let is_vanilla = config_json.mod_type == "Vanilla";

        match ModIndex::get_s(selected_instance).map_err(|err| err.to_string()) {
            Ok(idx) => {
                let locally_installed_mods =
                    MenuEditMods::update_locally_installed_mods(&idx, selected_instance, &self.dir);

                self.state = State::EditMods(MenuEditMods {
                    config: config_json,
                    mods: idx,
                    selected_mods: HashSet::new(),
                    sorted_mods_list: Vec::new(),
                    selected_state: SelectedState::None,
                    available_updates: Vec::new(),
                    mod_update_progress: None,
                    locally_installed_mods: HashSet::new(),
                });

                let update_cmd = if is_vanilla {
                    Task::none()
                } else {
                    Task::perform(
                        ql_mod_manager::mod_manager::check_for_updates(selected_instance.clone()),
                        |n| Message::ManageMods(ManageModsMessage::UpdateCheckResult(n)),
                    )
                };

                return Ok(Task::batch([locally_installed_mods, update_cmd]));
            }
            Err(err) => {
                self.set_error(err);
            }
        }
        Ok(Task::none())
    }

    pub fn mod_download(&mut self, index: usize) -> Option<Task<Message>> {
        let selected_instance = self.selected_instance.clone()?;
        let State::ModsDownload(menu) = &mut self.state else {
            return None;
        };
        let Some(results) = &menu.results else {
            err!("Couldn't download mod: Search results empty");
            return None;
        };
        let Some(hit) = results.hits.get(index) else {
            err!("Couldn't download mod: Not present in results");
            return None;
        };

        menu.mods_download_in_progress
            .insert(hit.project_id.clone());
        Some(Task::perform(
            ql_mod_manager::mod_manager::download_mod_w(hit.project_id.clone(), selected_instance),
            |n| Message::InstallMods(InstallModsMessage::DownloadComplete(n)),
        ))
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

    pub fn update_mod_index(&mut self) {
        if let State::EditMods(menu) = &mut self.state {
            match ModIndex::get_s(self.selected_instance.as_ref().unwrap())
                .map_err(|err| err.to_string())
            {
                Ok(idx) => menu.mods = idx,
                Err(err) => self.set_error(err),
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
            Task::perform(
                ql_mod_manager::mod_manager::apply_updates_w(
                    self.selected_instance.clone().unwrap(),
                    updates,
                    Some(sender),
                ),
                |n| Message::ManageMods(ManageModsMessage::UpdateModsFinished(n)),
            )
        } else {
            Task::none()
        }
    }

    pub fn go_to_server_manage_menu(&mut self, message: Option<String>) -> Task<Message> {
        self.state = State::ServerManage(MenuServerManage { message });
        Task::perform(
            get_entries("servers".to_owned(), true),
            Message::CoreListLoaded,
        )
    }

    pub fn install_forge(&mut self) -> Task<Message> {
        let (f_sender, f_receiver) = std::sync::mpsc::channel();
        let (j_sender, j_receiver) = std::sync::mpsc::channel();

        let command = Task::perform(
            loaders::forge::install_w(
                self.selected_instance.clone().unwrap(),
                Some(f_sender),
                Some(j_sender),
            ),
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
                    name: selected_server.clone(),
                    has_issued_stop_command: false,
                },
            );

            return Task::perform(
                ql_servers::read_logs_w(stdout, stderr, child, sender, selected_server.clone()),
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
                name: "Unknown".to_owned(),
                has_issued_stop_command: false,
            },
        );
        Task::none()
    }

    pub fn go_to_main_menu_with_message(&mut self, message: impl ToString) -> Task<Message> {
        let message = Some(message.to_string());
        match self.selected_instance.as_ref().unwrap() {
            InstanceSelection::Instance(_) => self.go_to_launch_screen(message),
            InstanceSelection::Server(_) => self.go_to_server_manage_menu(message),
        }
    }

    pub fn load_preset(&mut self) -> Task<Message> {
        let Some(file) = rfd::FileDialog::new()
            .add_filter("QuantumLauncher Mod Preset", &["qmp"])
            .set_title("Select Mod Preset to Load")
            .pick_file()
        else {
            return Task::none();
        };
        let file = match std::fs::read(&file).path(&file) {
            Ok(n) => n,
            Err(err) => {
                self.set_error(err);
                return Task::none();
            }
        };

        match tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(ql_mod_manager::PresetJson::load_w(
                self.selected_instance.clone().unwrap(),
                file,
            )) {
            Ok(mods) => {
                let (sender, receiver) = std::sync::mpsc::channel();
                if let State::ManagePresets(menu) = &mut self.state {
                    menu.progress = Some(ProgressBar::with_recv(receiver));
                }
                return Task::perform(
                    ql_mod_manager::PresetJson::download_entries_w(
                        mods,
                        self.selected_instance.clone().unwrap(),
                        sender,
                    ),
                    |n| Message::EditPresets(EditPresetsMessage::LoadComplete(n)),
                );
            }
            Err(err) => self.set_error(err),
        }

        Task::none()
    }

    pub fn go_to_edit_presets_menu(&mut self) -> Task<Message> {
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
                    mods: None,
                    progress: ProgressBar::with_recv(receiver),
                    error: None,
                }
            } else {
                MenuEditPresetsInner::Build {
                    mods: menu.sorted_mods_list.clone(),
                    selected_mods,
                    selected_state: SelectedState::All,
                    is_building: false,
                }
            },
            progress: None,
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

        Task::perform(
            ModVersion::get_compatible_mods_w(
                RECOMMENDED_MODS.to_owned(),
                json.id.clone(),
                loader,
                sender,
            ),
            |n| Message::EditPresets(EditPresetsMessage::RecommendedModCheck(n)),
        )
    }

    pub fn get_selected_instance_dir(&self) -> Option<PathBuf> {
        Some(
            self.selected_instance
                .as_ref()?
                .get_instance_path(&self.dir),
        )
    }

    pub fn get_selected_dot_minecraft_dir(&self) -> Option<PathBuf> {
        Some(
            self.selected_instance
                .as_ref()?
                .get_dot_minecraft_path(&self.dir),
        )
    }

    fn escape_back_button(&mut self) -> Task<Message> {
        let mut should_return_to_main_screen = false;
        let mut should_return_to_mods_screen = false;

        match &self.state {
            State::ChangeLog
            | State::EditMods(MenuEditMods {
                mod_update_progress: None,
                ..
            })
            | State::Create(MenuCreateInstance::Loaded { progress: None, .. })
            | State::Error { .. }
            | State::UpdateFound(MenuLauncherUpdate { progress: None, .. })
            | State::LauncherSettings
            | State::Welcome => {
                should_return_to_main_screen = true;
            }
            State::ConfirmAction { no, .. } => return self.update(no.clone()),

            State::InstallOptifine(MenuInstallOptifine {
                optifine_install_progress: None,
                java_install_progress: None,
                ..
            })
            | State::InstallFabric(MenuInstallFabric::Loaded { progress: None, .. }) => {
                should_return_to_mods_screen = true;
            }
            State::ModsDownload(menu) if menu.mods_download_in_progress.is_empty() => {
                should_return_to_mods_screen = true;
            }
            State::AccountLogin { cancel_handle, .. } => {
                cancel_handle.abort();
                should_return_to_main_screen = true;
            }
            State::InstallPaper
            | State::InstallForge(_)
            | State::InstallJava
            | State::InstallOptifine(_)
            | State::UpdateFound(_)
            | State::RedownloadAssets { .. }
            | State::InstallFabric(_)
            | State::EditMods(_)
            | State::Create(_)
            | State::ManagePresets(_)
            | State::ModsDownload(_)
            | State::ServerManage(_)
            | State::ServerCreate(_)
            | State::GenericMessage(_)
            | State::AccountLoginProgress(_)
            | State::Launch(_) => {}
        }

        if should_return_to_main_screen {
            return self.go_to_launch_screen(None);
        }
        if should_return_to_mods_screen {
            match self.go_to_edit_mods_menu_without_update_check() {
                Ok(cmd) => return cmd,
                Err(err) => self.set_error(err),
            }
        }

        Task::none()
    }

    pub fn iced_event(&mut self, event: iced::Event, status: iced::event::Status) -> Task<Message> {
        if let State::Launch(MenuLaunch { sidebar_width, .. }) = &mut self.state {
            if let Some(config) = &mut self.config {
                config.sidebar_width = Some(*sidebar_width as u32);
            }

            if self.window_size.0 > SIDEBAR_SQUISH_LIMIT as f32
                && *sidebar_width > self.window_size.0 as u16 - SIDEBAR_SQUISH_LIMIT
            {
                *sidebar_width = self.window_size.0 as u16 - SIDEBAR_SQUISH_LIMIT;
            }

            if self.window_size.0 > 100.0 && *sidebar_width < 100 {
                *sidebar_width = 100;
            }
        }

        match event {
            iced::Event::Window(event) => match event {
                iced::window::Event::CloseRequested => {
                    info!("Shutting down launcher (1)");
                    std::process::exit(0);
                }
                iced::window::Event::Closed => {
                    info!("Shutting down launcher (2)");
                }
                iced::window::Event::Resized(size) => {
                    self.window_size = (size.width, size.height);
                }
                iced::window::Event::RedrawRequested(_)
                | iced::window::Event::Moved { .. }
                | iced::window::Event::Opened { .. }
                | iced::window::Event::Focused
                | iced::window::Event::Unfocused
                | iced::window::Event::FileHovered(_)
                | iced::window::Event::FileDropped(_)
                | iced::window::Event::FilesHoveredLeft => {}
            },
            iced::Event::Keyboard(event) => match event {
                iced::keyboard::Event::KeyPressed {
                    key,
                    // location,
                    // modifiers,
                    ..
                } => {
                    if let iced::event::Status::Ignored = status {
                        if let Key::Named(iced::keyboard::key::Named::Escape) = key {
                            return self.escape_back_button();
                        } else {
                            // TODO: Ctrl Q to quit
                        }
                    } else {
                        // FUTURE
                    }
                }
                iced::keyboard::Event::KeyReleased { .. }
                | iced::keyboard::Event::ModifiersChanged(_) => {}
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
                        if self.mouse_pos.0 < 100.0 {
                            *sidebar_width = 100;
                        } else if (self.mouse_pos.0 + SIDEBAR_SQUISH_LIMIT as f32
                            > self.window_size.0)
                            && self.window_size.0 as u16 > SIDEBAR_SQUISH_LIMIT
                        {
                            *sidebar_width = self.window_size.0 as u16 - SIDEBAR_SQUISH_LIMIT;
                        } else {
                            *sidebar_width = self.mouse_pos.0 as u16;
                        }
                    }
                }
                iced::mouse::Event::ButtonPressed(button) => {
                    if let (State::Launch(menu), iced::mouse::Button::Left) =
                        (&mut self.state, button)
                    {
                        let difference = self.mouse_pos.0 - menu.sidebar_width as f32;
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
                iced::mouse::Event::WheelScrolled { .. }
                | iced::mouse::Event::CursorEntered
                | iced::mouse::Event::CursorLeft => {}
            },
            iced::Event::Touch(_) => {}
        }
        Task::none()
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
        let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if blacklist.contains(&file_name.to_owned()) {
            continue;
        }
        let Some(extension) = path.extension().and_then(|n| n.to_str()) else {
            continue;
        };
        if extension == "jar" {
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
