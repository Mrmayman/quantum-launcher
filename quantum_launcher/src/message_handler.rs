use std::{
    path::PathBuf,
    sync::{mpsc, Arc},
};

use iced::Command;
use quantum_launcher_backend::{
    download::progress::DownloadProgress, error::LauncherResult, file_utils,
    instance::instance_launch::GameLaunchResult, io_err,
    json_structs::json_instance_config::InstanceConfigJson,
};

use crate::launcher_state::{Launcher, MenuEditInstance, Message, State};

impl Launcher {
    pub fn select_launch_instance(&mut self, instance_name: String) {
        if let State::Launch(ref mut menu_launch) = self.state {
            menu_launch.selected_instance = Some(instance_name)
        }
    }

    pub fn set_username(&mut self, username: String) {
        self.config.as_mut().unwrap().username = username;
    }

    pub fn launch_game(&mut self) -> Command<Message> {
        if let State::Launch(ref mut menu_launch) = self.state {
            match self.config.as_ref().unwrap().save() {
                Ok(_) => {
                    let selected_instance = menu_launch.selected_instance.clone().unwrap();
                    let username = self.config.as_ref().unwrap().username.clone();
                    let manually_added_versions =
                        self.config.as_ref().unwrap().java_installs.clone();

                    return Command::perform(
                        quantum_launcher_backend::launch(
                            selected_instance,
                            username,
                            manually_added_versions,
                        ),
                        Message::LaunchEnd,
                    );
                }
                Err(err) => self.set_error(err.to_string()),
            };
        }
        Command::none()
    }

    pub fn finish_launching(&mut self, result: GameLaunchResult) {
        match result {
            GameLaunchResult::Ok(child) => self.spawned_process = Some(child),
            GameLaunchResult::Err(err) => self.set_error(err),
            GameLaunchResult::LocateJavaManually {
                required_java_version,
            } => {
                self.state = State::FindJavaVersion {
                    version: None,
                    required_version: required_java_version,
                }
            }
        }
    }

    pub fn go_to_create_screen(&mut self) -> Command<Message> {
        self.state = State::Create {
            instance_name: Default::default(),
            selected_version: None,
            versions: Vec::new(),
            progress_reciever: None,
            progress_number: None,
            progress_text: None,
        };
        Command::perform(
            quantum_launcher_backend::list_versions(),
            Message::CreateInstanceVersionsLoaded,
        )
    }

    pub fn create_instance_finish_loading_versions_list(
        &mut self,
        result: Result<Arc<Vec<String>>, String>,
    ) {
        match result {
            Ok(version_list) => {
                if let State::Create {
                    ref mut versions, ..
                } = self.state
                {
                    versions.extend_from_slice(&version_list)
                }
            }
            Err(n) => self.state = State::Error { error: n },
        }
    }

    pub fn select_created_instance_version(&mut self, selected_version: String) {
        if let State::Create {
            selected_version: ref mut version,
            ..
        } = self.state
        {
            *version = Some(selected_version)
        }
    }

    pub fn update_created_instance_name(&mut self, name: String) {
        if let State::Create {
            ref mut instance_name,
            ..
        } = self.state
        {
            *instance_name = name
        }
    }

    pub fn create_instance(&mut self) -> Command<Message> {
        if let State::Create {
            ref instance_name,
            selected_version: ref version,
            ref mut progress_reciever,
            ref mut progress_number,
            ref mut progress_text,
            ..
        } = self.state
        {
            let (sender, receiver) = mpsc::channel::<DownloadProgress>();
            *progress_reciever = Some(receiver);
            *progress_number = Some(0.0);
            *progress_text = Some("Started download".to_owned());

            // Create Instance asynchronously using iced Command.
            return Command::perform(
                quantum_launcher_backend::create_instance(
                    instance_name.to_owned(),
                    version.to_owned().unwrap(),
                    Some(sender),
                ),
                Message::CreateInstanceEnd,
            );
        }
        Command::none()
    }

    pub fn delete_selected_instance(&mut self) {
        if let State::DeleteInstance {
            ref selected_instance,
        } = self.state
        {
            match quantum_launcher_backend::file_utils::get_launcher_dir() {
                Ok(launcher_dir) => {
                    let instances_dir = launcher_dir.join("instances");
                    let deleted_instance_dir = instances_dir.join(selected_instance);

                    if !deleted_instance_dir.starts_with(&instances_dir) {
                        self.set_error("Tried to delete instance folder located outside Launcher. Potential attack avoided.".to_owned());
                        return;
                    }

                    if let Err(err) = std::fs::remove_dir_all(&deleted_instance_dir) {
                        self.set_error(err.to_string());
                        return;
                    }

                    match Launcher::new() {
                        Ok(launcher) => *self = launcher,
                        Err(err) => self.set_error(err.to_string()),
                    }
                }
                Err(err) => self.set_error(err.to_string()),
            }
        }
    }

    pub fn confirm_instance_deletion(&mut self) {
        if let State::Launch(ref mut menu_launch) = self.state {
            self.state = State::DeleteInstance {
                selected_instance: menu_launch.selected_instance.clone().unwrap(),
            }
        }
    }

    pub fn update_instance_creation_progress_bar(&mut self) {
        if let State::Create {
            ref mut progress_number,
            ref progress_reciever,
            ref mut progress_text,
            ..
        } = self.state
        {
            if let Some(Ok(progress)) = progress_reciever.as_ref().map(|n| n.try_recv()) {
                if let Some(progress_text) = progress_text {
                    *progress_text = progress.to_string()
                }
                if let Some(progress_num) = progress_number {
                    *progress_num = progress.into();
                }
            }
        }
    }

    pub fn add_java_to_config(&mut self, path: Option<PathBuf>) {
        match (path.as_ref().map(|n| n.to_str()), &mut self.config) {
            // Config not loaded.
            (_, None) => self.set_error(
                "Could not open launcher config (QuantumLauncher/launcher.config)".to_owned(),
            ),
            // Couldn't find path.
            (None, _) => self.set_error("Selected Java path not found.".to_owned()),
            // Couldn't convert path to string.
            (Some(None), _) => {
                self.set_error("Selected Java path contains invalid characters".to_owned())
            }
            (Some(Some(path)), Some(config)) => {
                // Add java path to list of java installs.
                config.java_installs.push(path.to_owned());
                // Save config.
                match config.save() {
                    Ok(_) => {}
                    Err(err) => self.set_error(err.to_string()),
                }
                self.go_to_launch_screen()
            }
        }
    }

    pub fn edit_instance(&mut self, selected_instance: String) -> LauncherResult<()> {
        let launcher_dir = file_utils::get_launcher_dir()?;
        let config_path = launcher_dir
            .join("instances")
            .join(&selected_instance)
            .join("config.json");

        let config_json = std::fs::read_to_string(&config_path).map_err(io_err!(config_path))?;
        let config_json: InstanceConfigJson = serde_json::from_str(&config_json)?;

        let slider_value = f32::log2(config_json.ram_in_mb as f32);
        let memory_mb = config_json.ram_in_mb;

        self.state = State::EditInstance(MenuEditInstance {
            selected_instance,
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

    pub fn edit_mods(&mut self, selected_instance: String) -> LauncherResult<()> {
        let launcher_dir = file_utils::get_launcher_dir()?;
        let config_path = launcher_dir
            .join("instances")
            .join(&selected_instance)
            .join("config.json");

        let config_json = std::fs::read_to_string(&config_path).map_err(io_err!(config_path))?;
        let config_json: InstanceConfigJson = serde_json::from_str(&config_json)?;

        self.state = State::EditMods {
            selected_instance,
            config: config_json,
        };
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
    println!("[error] Opening file explorer not supported on this platform.")
}
