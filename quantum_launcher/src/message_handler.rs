use std::{
    collections::HashSet,
    sync::{mpsc, Arc},
};

use chrono::Datelike;
use iced::Command;
use ql_core::{
    err, file_utils,
    json::{instance_config::InstanceConfigJson, version::VersionDetails},
    DownloadProgress, InstanceSelection, IntoIoError, JsonFileError,
};
use ql_instances::{GameLaunchResult, ListEntry};
use ql_mod_manager::{
    loaders,
    mod_manager::{Loader, ModIndex, ModVersion, RECOMMENDED_MODS},
};

use crate::launcher_state::{
    get_entries, ClientProcess, CreateInstanceMessage, EditPresetsMessage, InstallModsMessage,
    Launcher, ManageModsMessage, MenuCreateInstance, MenuEditInstance, MenuEditMods,
    MenuEditPresets, MenuEditPresetsInner, MenuInstallForge, MenuLaunch, MenuServerManage, Message,
    ProgressBar, SelectedState, ServerProcess, State,
};

impl Launcher {
    pub fn set_username(&mut self, username: String) {
        self.config.as_mut().unwrap().username = username;
    }

    pub fn launch_game(&mut self) -> Command<Message> {
        if let State::Launch(ref mut menu_launch) = self.state {
            let selected_instance = self.selected_instance.as_ref().unwrap().get_name();
            let username = self.config.as_ref().unwrap().username.clone();

            let (sender, receiver) = std::sync::mpsc::channel();
            menu_launch.java_recv = Some(receiver);

            let (asset_sender, asset_receiver) = std::sync::mpsc::channel();
            menu_launch.asset_recv = Some(asset_receiver);

            if let Some(log) = self.client_logs.get_mut(selected_instance) {
                log.log.clear();
            }

            return Command::perform(
                ql_instances::launch_w(
                    selected_instance.to_owned(),
                    username,
                    Some(sender),
                    Some(asset_sender),
                ),
                Message::LaunchEnd,
            );
        }
        Command::none()
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

    pub fn finish_launching(&mut self, result: GameLaunchResult) -> Command<Message> {
        match result {
            GameLaunchResult::Ok(child) => {
                let Some(InstanceSelection::Instance(selected_instance)) =
                    self.selected_instance.clone()
                else {
                    err!("Game Launched, but unknown instance!\n          This is a bug, please report it if found.");
                    return Command::none();
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

                    return Command::perform(
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
        Command::none()
    }

    pub fn go_to_create_screen(&mut self) -> Command<Message> {
        if let Some(versions) = self.client_version_list_cache.clone() {
            let combo_state = iced::widget::combo_box::State::new(versions.clone());
            self.state = State::Create(MenuCreateInstance::Loaded {
                instance_name: String::new(),
                selected_version: None,
                progress: None,
                download_assets: true,
                combo_state: Box::new(combo_state),
            });
            Command::none()
        } else {
            let (sender, receiver) = mpsc::channel();
            self.state = State::Create(MenuCreateInstance::Loading {
                progress_receiver: receiver,
                progress_number: 0.0,
            });
            Command::perform(ql_instances::list_versions(Some(Arc::new(sender))), |n| {
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

    pub fn create_instance(&mut self) -> Command<Message> {
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
            return Command::perform(
                ql_instances::create_instance_w(
                    instance_name.clone(),
                    selected_version.clone().unwrap(),
                    Some(sender),
                    *download_assets,
                ),
                |n| Message::CreateInstance(CreateInstanceMessage::End(n)),
            );
        }
        Command::none()
    }

    pub fn delete_selected_instance(&mut self) -> Command<Message> {
        if let State::ConfirmAction { .. } = &self.state {
            let selected_instance = self.selected_instance.as_ref().unwrap();
            match (
                file_utils::get_instance_dir(selected_instance),
                file_utils::get_launcher_dir(),
            ) {
                (Ok(deleted_instance_dir), Ok(launcher_dir)) => {
                    let instances_dir = launcher_dir.join("instances");

                    if !deleted_instance_dir.starts_with(&instances_dir) {
                        self.set_error("Tried to delete instance folder located outside Launcher. Potential attack avoided.".to_owned());
                        return Command::none();
                    }

                    if let Err(err) = std::fs::remove_dir_all(&deleted_instance_dir) {
                        self.set_error(err);
                        return Command::none();
                    }

                    self.selected_instance = None;
                    return self.go_to_launch_screen(Some("Deleted Instance".to_owned()));
                }
                (Err(err), Ok(_) | Err(_)) | (Ok(_), Err(err)) => self.set_error(err.to_string()),
            }
        }
        Command::none()
    }

    pub fn edit_instance(
        &mut self,
        selected_instance: &InstanceSelection,
    ) -> Result<(), JsonFileError> {
        let config_path = file_utils::get_instance_dir(selected_instance)?.join("config.json");

        let config_json = std::fs::read_to_string(&config_path).path(config_path)?;
        let config_json: InstanceConfigJson = serde_json::from_str(&config_json)?;

        let slider_value = f32::log2(config_json.ram_in_mb as f32);
        let memory_mb = config_json.ram_in_mb;

        self.state = State::EditInstance(MenuEditInstance {
            config: config_json,
            slider_value,
            slider_text: format_memory(memory_mb),
        });
        Ok(())
    }

    pub fn save_config(
        instance_name: &InstanceSelection,
        config: &InstanceConfigJson,
    ) -> Result<(), JsonFileError> {
        let mut config = config.clone();
        if config.enable_logger.is_none() {
            config.enable_logger = Some(true);
        }
        let config_path = file_utils::get_instance_dir(instance_name)?.join("config.json");

        let config_json = serde_json::to_string(&config)?;
        std::fs::write(&config_path, config_json).path(config_path)?;
        Ok(())
    }

    pub fn go_to_edit_mods_menu_without_update_check(
        &mut self,
    ) -> Result<Command<Message>, JsonFileError> {
        let selected_instance = self.selected_instance.as_ref().unwrap();
        let config_path = file_utils::get_instance_dir(selected_instance)?.join("config.json");

        let config_json = std::fs::read_to_string(&config_path).path(config_path)?;
        let config_json: InstanceConfigJson = serde_json::from_str(&config_json)?;

        match ModIndex::get(selected_instance).map_err(|err| err.to_string()) {
            Ok(idx) => {
                let locally_installed_mods =
                    MenuEditMods::update_locally_installed_mods(&idx, selected_instance.clone());

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

                Ok(Command::batch(vec![locally_installed_mods]))
            }
            Err(err) => {
                self.set_error(err);
                Ok(Command::none())
            }
        }
    }

    pub fn go_to_edit_mods_menu(&mut self) -> Result<Command<Message>, JsonFileError> {
        let selected_instance = self.selected_instance.as_ref().unwrap();
        let config_path = file_utils::get_instance_dir(selected_instance)?.join("config.json");

        let config_json = std::fs::read_to_string(&config_path).path(config_path)?;
        let config_json: InstanceConfigJson = serde_json::from_str(&config_json)?;

        let is_vanilla = config_json.mod_type == "Vanilla";

        match ModIndex::get(selected_instance).map_err(|err| err.to_string()) {
            Ok(idx) => {
                let locally_installed_mods =
                    MenuEditMods::update_locally_installed_mods(&idx, selected_instance.clone());

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
                    Command::none()
                } else {
                    Command::perform(
                        ql_mod_manager::mod_manager::check_for_updates(selected_instance.clone()),
                        |n| Message::ManageMods(ManageModsMessage::UpdateCheckResult(n)),
                    )
                };

                return Ok(Command::batch(vec![locally_installed_mods, update_cmd]));
            }
            Err(err) => {
                self.set_error(err);
            }
        }
        Ok(Command::none())
    }

    pub fn mod_download(&mut self, index: usize) -> Option<Command<Message>> {
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
        Some(Command::perform(
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
            match ModIndex::get(self.selected_instance.as_ref().unwrap())
                .map_err(|err| err.to_string())
            {
                Ok(idx) => menu.mods = idx,
                Err(err) => self.set_error(err),
            }
        }
    }

    pub fn update_mods(&mut self) -> Command<Message> {
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
            Command::perform(
                ql_mod_manager::mod_manager::apply_updates_w(
                    self.selected_instance.clone().unwrap(),
                    updates,
                    Some(sender),
                ),
                |n| Message::ManageMods(ManageModsMessage::UpdateModsFinished(n)),
            )
        } else {
            Command::none()
        }
    }

    pub fn go_to_server_manage_menu(&mut self, message: Option<String>) -> Command<Message> {
        self.state = State::ServerManage(MenuServerManage {
            java_install_recv: None,
            message,
        });
        Command::perform(
            get_entries("servers".to_owned(), true),
            Message::CoreListLoaded,
        )
    }

    pub fn install_forge(&mut self) -> Command<Message> {
        let (f_sender, f_receiver) = std::sync::mpsc::channel();
        let (j_sender, j_receiver) = std::sync::mpsc::channel();

        let command = Command::perform(
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
    ) -> Command<Message> {
        let Some(InstanceSelection::Server(selected_server)) = &self.selected_instance else {
            err!("Launched server but can't identify which one! This is a bug, please report it");
            return Command::none();
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

            return Command::perform(
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
        Command::none()
    }

    pub fn go_to_main_menu_with_message(&mut self, message: impl ToString) -> Command<Message> {
        let message = Some(message.to_string());
        match self.selected_instance.as_ref().unwrap() {
            InstanceSelection::Instance(_) => self.go_to_launch_screen(message),
            InstanceSelection::Server(_) => self.go_to_server_manage_menu(message),
        }
    }

    pub fn load_preset(&mut self) -> Command<Message> {
        let Some(file) = rfd::FileDialog::new()
            .add_filter("QuantumLauncher Mod Preset", &["qmp"])
            .set_title("Select Mod Preset to Load")
            .pick_file()
        else {
            return Command::none();
        };
        let file = match std::fs::read(&file).path(&file) {
            Ok(n) => n,
            Err(err) => {
                self.set_error(err);
                return Command::none();
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
                    menu.progress = Some(ProgressBar::with_recv(receiver))
                }
                return Command::perform(
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

        Command::none()
    }

    pub fn go_to_edit_presets_menu(&mut self) -> Command<Message> {
        let State::EditMods(menu) = &self.state else {
            return Command::none();
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
            return Command::none();
        }

        let Some(json) = VersionDetails::load(self.selected_instance.as_ref().unwrap()) else {
            return Command::none();
        };

        let Ok(loader) = Loader::try_from(mod_type.as_str()) else {
            return Command::none();
        };

        Command::perform(
            ModVersion::get_compatible_mods_w(
                RECOMMENDED_MODS.to_owned(),
                json.id.clone(),
                loader,
                sender,
            ),
            |n| Message::EditPresets(EditPresetsMessage::RecommendedModCheck(n)),
        )
    }
}

pub async fn get_locally_installed_mods(
    selected_instance: InstanceSelection,
    blacklist: Vec<String>,
) -> HashSet<String> {
    let mods_dir_path = file_utils::get_dot_minecraft_dir(&selected_instance)
        .unwrap()
        .join("mods");

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

#[allow(clippy::zombie_processes)]
pub fn open_file_explorer(path: &str) {
    use std::process::Command;

    #[cfg(target_os = "linux")]
    {
        let _ = Command::new("xdg-open").arg(path).spawn().unwrap();
    }

    #[cfg(target_os = "windows")]
    {
        let _ = Command::new("explorer").arg(path).spawn().unwrap();
    }

    #[cfg(target_os = "macos")]
    {
        let _ = Command::new("open").arg(path).spawn().unwrap();
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    panic!("Opening file explorer not supported on this platform.")
}
