use std::{
    collections::HashSet,
    sync::{mpsc, Arc},
};

use chrono::Datelike;
use iced::Command;
use ql_core::{
    err, file_utils, json::instance_config::InstanceConfigJson, DownloadProgress,
    InstanceSelection, IntoIoError, JsonFileError,
};
use ql_instances::{GameLaunchResult, ListEntry};
use ql_mod_manager::mod_manager::ModIndex;

use crate::launcher_state::{
    ClientProcess, CreateInstanceMessage, Launcher, MenuCreateInstance, MenuEditInstance,
    MenuEditMods, Message, SelectedState, State,
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

            let instance_config = {
                let launcher_dir = file_utils::get_launcher_dir().unwrap();
                let config_path = launcher_dir
                    .join("instances")
                    .join(selected_instance)
                    .join("config.json");

                let config_json = std::fs::read_to_string(&config_path).unwrap();
                serde_json::from_str::<InstanceConfigJson>(&config_json).unwrap()
            };

            return Command::perform(
                ql_instances::launch_w(
                    selected_instance.to_owned(),
                    username,
                    Some(sender),
                    instance_config.enable_logger.unwrap_or(true),
                    Some(asset_sender),
                    instance_config.game_args.unwrap_or_default(),
                    instance_config.java_args.unwrap_or_default(),
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
                progress_receiver: None,
                progress_number: None,
                progress_text: None,
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
                    progress_receiver: None,
                    progress_number: None,
                    progress_text: None,
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
            progress_receiver,
            progress_text,
            progress_number,
            instance_name,
            download_assets,
            selected_version,
            ..
        }) = &mut self.state
        {
            let (sender, receiver) = mpsc::channel::<DownloadProgress>();
            *progress_receiver = Some(receiver);
            *progress_number = Some(0.0);
            *progress_text = Some("Started download".to_owned());

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
        if let State::DeleteInstance = &self.state {
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
                        Message::ManageModsUpdateCheckResult,
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
