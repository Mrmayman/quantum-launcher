use std::sync::{mpsc, Arc};

use chrono::Datelike;
use iced::Command;
use ql_instances::{
    error::LauncherResult, file_utils, io_err,
    json_structs::json_instance_config::InstanceConfigJson, DownloadProgress, GameLaunchResult,
};

use crate::launcher_state::{
    GameProcess, Launcher, MenuCreateInstance, MenuEditInstance, MenuEditMods, Message, State,
};

impl Launcher {
    pub fn select_launch_instance(&mut self, instance_name: String) {
        self.selected_instance = Some(instance_name)
    }

    pub fn set_username(&mut self, username: String) {
        self.config.as_mut().unwrap().username = username;
    }

    pub fn launch_game(&mut self) -> Command<Message> {
        if let State::Launch(ref mut menu_launch) = self.state {
            let selected_instance = self.selected_instance.clone().unwrap();
            let username = self.config.as_ref().unwrap().username.clone();

            let (sender, receiver) = std::sync::mpsc::channel();
            menu_launch.recv = Some(receiver);

            if let Some(log) = self.logs.get_mut(&selected_instance) {
                log.clear();
            }

            return Command::perform(
                ql_instances::launch_wrapped(selected_instance, username, Some(sender)),
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
        format!("{} {} {}", day, month, year)
    }

    pub fn finish_launching(&mut self, result: GameLaunchResult) -> Command<Message> {
        match result {
            GameLaunchResult::Ok(child) => {
                if let Some(selected_instance) = self.selected_instance.to_owned() {
                    if let (Some(stdout), Some(stderr)) = {
                        let mut child = child.lock().unwrap();
                        (child.stdout.take(), child.stderr.take())
                    } {
                        let (sender, receiver) = std::sync::mpsc::channel();

                        self.processes.insert(
                            selected_instance.clone(),
                            GameProcess {
                                child: child.clone(),
                                receiver,
                            },
                        );

                        return Command::perform(
                            ql_instances::read_logs_wrapped(
                                stdout,
                                stderr,
                                child,
                                sender,
                                selected_instance,
                            ),
                            Message::LaunchEndedLog,
                        );
                    }
                } else {
                    eprintln!("[warning] Game Launched, but unknown instance!\n          This is a bug, please report it if found.")
                }
            }
            GameLaunchResult::Err(err) => self.set_error(err),
        }
        Command::none()
    }

    pub fn go_to_create_screen(&mut self) -> Command<Message> {
        const SKIP_LISTING_VERSIONS: bool = false;

        self.state = State::Create(MenuCreateInstance {
            instance_name: Default::default(),
            selected_version: None,
            versions: Vec::new(),
            progress_receiver: None,
            progress_number: None,
            progress_text: None,
            download_assets: true,
        });

        if SKIP_LISTING_VERSIONS {
            Command::none()
        } else {
            Command::perform(
                ql_instances::list_versions(),
                Message::CreateInstanceVersionsLoaded,
            )
        }
    }

    pub fn create_instance_finish_loading_versions_list(
        &mut self,
        result: Result<Arc<Vec<String>>, String>,
    ) {
        match result {
            Ok(version_list) => {
                if let State::Create(menu) = &mut self.state {
                    menu.versions.extend_from_slice(&version_list)
                }
            }
            Err(n) => self.state = State::Error { error: n },
        }
    }

    pub fn select_created_instance_version(&mut self, selected_version: String) {
        if let State::Create(menu) = &mut self.state {
            menu.selected_version = Some(selected_version)
        }
    }

    pub fn update_created_instance_name(&mut self, name: String) {
        if let State::Create(menu) = &mut self.state {
            menu.instance_name = name
        }
    }

    pub fn create_instance(&mut self) -> Command<Message> {
        if let State::Create(menu) = &mut self.state {
            let (sender, receiver) = mpsc::channel::<DownloadProgress>();
            menu.progress_receiver = Some(receiver);
            menu.progress_number = Some(0.0);
            menu.progress_text = Some("Started download".to_owned());

            // Create Instance asynchronously using iced Command.
            return Command::perform(
                ql_instances::create_instance(
                    menu.instance_name.to_owned(),
                    menu.selected_version.to_owned().unwrap(),
                    Some(sender),
                    menu.download_assets,
                ),
                Message::CreateInstanceEnd,
            );
        }
        Command::none()
    }

    pub fn delete_selected_instance(&mut self) {
        if let State::DeleteInstance(_) = &self.state {
            match file_utils::get_launcher_dir() {
                Ok(launcher_dir) => {
                    let instances_dir = launcher_dir.join("instances");
                    let deleted_instance_dir =
                        instances_dir.join(self.selected_instance.as_ref().unwrap());

                    if !deleted_instance_dir.starts_with(&instances_dir) {
                        self.set_error("Tried to delete instance folder located outside Launcher. Potential attack avoided.".to_owned());
                        return;
                    }

                    if let Err(err) = std::fs::remove_dir_all(&deleted_instance_dir) {
                        self.set_error(err.to_string());
                        return;
                    }

                    match Launcher::new(Some("Deleted Instance".to_owned())) {
                        Ok(launcher) => *self = launcher,
                        Err(err) => self.set_error(err.to_string()),
                    }
                }
                Err(err) => self.set_error(err.to_string()),
            }
        }
    }

    pub fn update_instance_creation_progress_bar(menu: &mut MenuCreateInstance) {
        if let Some(receiver) = &menu.progress_receiver {
            if let Ok(progress) = receiver.try_recv() {
                if let Some(progress_text) = &mut menu.progress_text {
                    *progress_text = progress.to_string()
                }
                if let Some(progress_num) = &mut menu.progress_number {
                    *progress_num = progress.into();
                }
            }
        }
    }

    pub fn edit_instance(&mut self, selected_instance: String) -> LauncherResult<()> {
        let launcher_dir = file_utils::get_launcher_dir()?;
        let config_path = launcher_dir
            .join("instances")
            .join(selected_instance)
            .join("config.json");

        let config_json = std::fs::read_to_string(&config_path).map_err(io_err!(config_path))?;
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

    pub fn save_config(instance_name: &str, config: &InstanceConfigJson) -> LauncherResult<()> {
        let launcher_dir = file_utils::get_launcher_dir()?;
        let config_path = launcher_dir
            .join("instances")
            .join(instance_name)
            .join("config.json");

        let config_json = serde_json::to_string(config)?;
        std::fs::write(&config_path, config_json).map_err(io_err!(config_path))?;
        Ok(())
    }

    pub fn go_to_edit_mods_menu(&mut self) -> LauncherResult<()> {
        let launcher_dir = file_utils::get_launcher_dir()?;
        let config_path = launcher_dir
            .join("instances")
            .join(self.selected_instance.as_ref().unwrap())
            .join("config.json");

        let config_json = std::fs::read_to_string(&config_path).map_err(io_err!(config_path))?;
        let config_json: InstanceConfigJson = serde_json::from_str(&config_json)?;

        self.state = State::EditMods(MenuEditMods {
            config: config_json,
        });
        Ok(())
    }
}

pub fn format_memory(memory_bytes: usize) -> String {
    const MB_TO_GB: usize = 1024;

    if memory_bytes >= MB_TO_GB {
        format!("{:.2} GB", memory_bytes as f64 / MB_TO_GB as f64)
    } else {
        format!("{memory_bytes} MB")
    }
}

pub fn open_file_explorer(path: &str) {
    use std::process::Command;

    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open").arg(path).spawn().unwrap();
    }

    #[cfg(target_os = "windows")]
    {
        Command::new("explorer").arg(path).spawn().unwrap();
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open").arg(path).spawn().unwrap();
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    eprintln!("[error] Opening file explorer not supported on this platform.")
}
