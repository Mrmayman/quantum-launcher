use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::{mpsc, Arc, Mutex},
};

use chrono::Datelike;
use iced::{keyboard::Key, Task};
use ql_core::{
    err, file_utils, info, info_no_log,
    json::{instance_config::InstanceConfigJson, version::VersionDetails},
    DownloadProgress, InstanceSelection, IntoIoError, IntoStringError, JsonFileError, Loader,
    ModId,
};
use ql_instances::{AccountData, ListEntry};
use ql_mod_manager::{
    loaders,
    mod_manager::{ModIndex, RecommendedMod, RECOMMENDED_MODS},
};
use tokio::process::Child;

use crate::{
    config::ConfigAccount,
    get_entries,
    launcher_state::{
        ClientProcess, CreateInstanceMessage, EditPresetsMessage, InstallModsMessage,
        MenuCreateInstance, MenuEditInstance, MenuEditMods, MenuEditPresets, MenuEditPresetsInner,
        MenuInstallFabric, MenuInstallForge, MenuInstallOptifine, MenuLaunch, MenuLauncherUpdate,
        NEW_ACCOUNT_NAME,
    },
    Launcher, ManageModsMessage, Message, ProgressBar, SelectedState, ServerProcess, State,
};

pub const SIDEBAR_DRAG_LEEWAY: f32 = 10.0;
pub const SIDEBAR_SQUISH_LIMIT: u16 = 300;

impl Launcher {
    pub fn launch_game(&mut self, account_data: Option<AccountData>) -> Task<Message> {
        if let State::Launch(ref mut menu_launch) = self.state {
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

            let (asset_sender, asset_receiver) = std::sync::mpsc::channel();
            menu_launch.asset_recv = Some(asset_receiver);

            if let Some(log) = self.client_logs.get_mut(selected_instance) {
                log.log.clear();
            }

            let instance_name = selected_instance.to_owned();
            return Task::perform(
                async move {
                    ql_instances::launch(
                        instance_name,
                        username,
                        Some(sender),
                        Some(asset_sender),
                        account_data,
                    )
                    .await
                    .strerr()
                },
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

    pub fn finish_launching(&mut self, result: Result<Arc<Mutex<Child>>, String>) -> Task<Message> {
        self.java_recv = None;
        if let State::Launch(menu) = &mut self.state {
            menu.asset_recv = None;
        }
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
                            ql_instances::read_logs(
                                stdout,
                                stderr,
                                child,
                                sender,
                                selected_instance,
                            )
                            .await
                            .strerr()
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

            let (task, handle) =
                Task::perform(ql_instances::list_versions(Some(Arc::new(sender))), |n| {
                    Message::CreateInstance(CreateInstanceMessage::VersionsLoaded(n.strerr()))
                })
                .abortable();

            self.state = State::Create(MenuCreateInstance::Loading {
                receiver,
                number: 0.0,
                _handle: handle.abort_on_drop(),
            });

            task
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

            let instance_name = instance_name.clone();
            let version = selected_version.clone().unwrap();
            let download_assets = *download_assets;

            // Create Instance asynchronously using iced Command.
            return Task::perform(
                ql_instances::create_instance(
                    instance_name.clone(),
                    version,
                    Some(sender),
                    download_assets,
                ),
                |n| Message::CreateInstance(CreateInstanceMessage::End(n.strerr())),
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

        match ModIndex::get_s(selected_instance).strerr() {
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

        match ModIndex::get_s(selected_instance).strerr() {
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
                        |n| Message::ManageMods(ManageModsMessage::UpdateCheckResult(n.strerr())),
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
        let Some(hit) = results.mods.get(index) else {
            err!("Couldn't download mod: Not present in results");
            return None;
        };

        menu.mods_download_in_progress
            .insert(ModId::Modrinth(hit.id.clone()));

        let project_id = hit.id.clone();
        let backend = menu.backend;
        let id = ModId::from_pair(&project_id, backend);

        Some(Task::perform(
            async move {
                ql_mod_manager::mod_manager::download_mod(&id, &selected_instance)
                    .await
                    .map(|()| ModId::Modrinth(project_id))
            },
            |n| Message::InstallMods(InstallModsMessage::DownloadComplete(n.strerr())),
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
            match ModIndex::get_s(self.selected_instance.as_ref().unwrap()).strerr() {
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
            let selected_instance = self.selected_instance.clone().unwrap();
            Task::perform(
                ql_mod_manager::mod_manager::apply_updates(
                    selected_instance,
                    updates,
                    Some(sender),
                ),
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
                menu.message = message
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
                    name: selected_server.clone(),
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
                name: "Unknown".to_owned(),
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
            .block_on(ql_mod_manager::PresetJson::load(
                self.selected_instance.clone().unwrap(),
                file,
            )) {
            Ok(mods) => {
                let (sender, receiver) = std::sync::mpsc::channel();
                if let State::ManagePresets(menu) = &mut self.state {
                    menu.progress = Some(ProgressBar::with_recv(receiver));
                }
                let instance_name = self.selected_instance.clone().unwrap();
                return Task::perform(
                    ql_mod_manager::mod_manager::download_mods_bulk(
                        mods,
                        instance_name,
                        Some(sender),
                    ),
                    |n| Message::EditPresets(EditPresetsMessage::LoadComplete(n.strerr())),
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

        let version = json.id.clone();
        let ids = RECOMMENDED_MODS.to_owned();
        Task::perform(
            RecommendedMod::get_compatible_mods(ids, version, loader, sender),
            |n| Message::EditPresets(EditPresetsMessage::RecommendedModCheck(n.strerr())),
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
            | State::ServerCreate(_)
            | State::GenericMessage(_)
            | State::AccountLoginProgress(_)
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

        Task::none()
    }

    pub fn iced_event(&mut self, event: iced::Event, status: iced::event::Status) -> Task<Message> {
        if let State::Launch(MenuLaunch { sidebar_width, .. }) = &mut self.state {
            self.config.sidebar_width = Some(*sidebar_width as u32);

            if self.window_size.0 > f32::from(SIDEBAR_SQUISH_LIMIT)
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
                    info_no_log!("Shutting down launcher (1)");
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
                        } else if (self.mouse_pos.0 + f32::from(SIDEBAR_SQUISH_LIMIT)
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
                iced::mouse::Event::WheelScrolled { .. }
                | iced::mouse::Event::CursorEntered
                | iced::mouse::Event::CursorLeft => {}
            },
            iced::Event::Touch(_) => {}
        }
        Task::none()
    }

    pub fn account_selected(&mut self, account: String) -> Task<Message> {
        if account == NEW_ACCOUNT_NAME {
            self.state = State::GenericMessage("Loading Login...".to_owned());
            Task::perform(
                async move { ql_instances::login_1_link().await.strerr() },
                Message::AccountResponse1,
            )
        } else {
            self.accounts_selected = Some(account);
            Task::none()
        }
    }

    pub fn account_refresh(&mut self, account: &ql_instances::AccountData) -> Task<Message> {
        let (sender, receiver) = std::sync::mpsc::channel();

        self.state = State::AccountLoginProgress(ProgressBar::with_recv(receiver));

        let username = account.username.clone();
        let refresh_token = account.refresh_token.clone();
        Task::perform(
            async move {
                ql_instances::login_refresh(username, refresh_token, Some(sender))
                    .await
                    .strerr()
            },
            Message::AccountRefreshComplete,
        )
    }

    pub fn account_response_3(&mut self, data: ql_instances::AccountData) -> Task<Message> {
        self.accounts_dropdown.insert(0, data.username.clone());

        if self.config.accounts.is_none() {
            self.config.accounts = Some(HashMap::new());
        }
        let accounts = self.config.accounts.as_mut().unwrap();
        accounts.insert(
            data.username.clone(),
            ConfigAccount {
                uuid: data.uuid.clone(),
                skin: None,
            },
        );

        self.accounts_selected = Some(data.username.clone());
        self.accounts.insert(data.username.clone(), data);

        self.go_to_launch_screen::<String>(None)
    }

    pub fn account_response_2(&mut self, token: ql_instances::AuthTokenResponse) -> Task<Message> {
        let (sender, receiver) = std::sync::mpsc::channel();
        self.state = State::AccountLoginProgress(ProgressBar::with_recv(receiver));
        Task::perform(
            async move {
                ql_instances::login_3_xbox(token, Some(sender))
                    .await
                    .strerr()
            },
            Message::AccountResponse3,
        )
    }

    pub fn account_response_1(&mut self, code: ql_instances::AuthCodeResponse) -> Task<Message> {
        // I have no idea how many rustaceans will
        // yell at me after they see this. (WTF: )
        let code2 = code.clone();
        let (task, handle) = Task::perform(
            async move { ql_instances::login_2_wait(code2).await.strerr() },
            Message::AccountResponse2,
        )
        .abortable();
        self.state = State::AccountLogin {
            url: code.verification_uri,
            code: code.user_code,
            cancel_handle: handle,
        };
        task
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
