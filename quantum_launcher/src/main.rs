use std::time::Duration;

use colored::Colorize;
use iced::{
    executor,
    widget::{self, image::Handle},
    Application, Command, Settings,
};
use launcher_state::{
    reload_instances, Launcher, MenuDeleteInstance, MenuInstallFabric, MenuInstallForge,
    MenuInstallOptifine, MenuLaunch, MenuLauncherSettings, MenuLauncherUpdate, Message,
    SelectedMod, SelectedState, State,
};

use message_handler::{format_memory, open_file_explorer};
use ql_instances::{
    err, file_utils, info,
    json_structs::{json_instance_config::InstanceConfigJson, json_version::VersionDetails},
    UpdateCheckInfo, LAUNCHER_VERSION_NAME,
};
use ql_mod_manager::{
    instance_mod_installer,
    mod_manager::{ModIndex, ProjectInfo},
};
use stylesheet::styles::{LauncherStyle, LauncherTheme};

mod config;
mod icon_manager;
mod launcher_state;
mod menu_renderer;
mod message_handler;
mod mods_store;
mod stylesheet;
mod tick;

impl Application for Launcher {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = LauncherTheme;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, iced::Command<Self::Message>) {
        (
            match Launcher::new(None) {
                Ok(launcher) => launcher,
                Err(error) => Launcher::with_error(&error.to_string()),
            },
            Command::perform(
                ql_instances::check_for_updates_wrapped(),
                Message::UpdateCheckResult,
            ),
        )
    }

    fn title(&self) -> String {
        "Quantum Launcher".to_owned()
    }

    fn update(&mut self, message: Self::Message) -> iced::Command<Self::Message> {
        match message {
            Message::LaunchInstanceSelected(selected_instance) => {
                self.select_launch_instance(selected_instance);
            }
            Message::LaunchUsernameSet(username) => self.set_username(username),
            Message::LaunchStart => return self.launch_game(),
            Message::LaunchEnd(result) => {
                return self.finish_launching(result);
            }
            Message::CreateInstanceScreenOpen => return self.go_to_create_screen(),
            Message::CreateInstanceVersionsLoaded(result) => {
                self.create_instance_finish_loading_versions_list(result);
            }
            Message::CreateInstanceVersionSelected(selected_version) => {
                self.select_created_instance_version(selected_version);
            }
            Message::CreateInstanceNameInput(name) => self.update_created_instance_name(name),
            Message::CreateInstanceStart => return self.create_instance(),
            Message::CreateInstanceEnd(result) => match result {
                Ok(()) => match Launcher::new(Some("Created New Instance".to_owned())) {
                    Ok(launcher) => *self = launcher,
                    Err(err) => self.set_error(err.to_string()),
                },
                Err(n) => self.state = State::Error { error: n },
            },
            Message::DeleteInstanceMenu => {
                self.state = State::DeleteInstance(MenuDeleteInstance {});
            }
            Message::DeleteInstance => self.delete_selected_instance(),
            Message::LaunchScreenOpen(message) => {
                if let Some(message) = message {
                    self.go_to_launch_screen_with_message(message);
                } else {
                    self.go_to_launch_screen();
                }
            }
            Message::EditInstance => {
                self.edit_instance_wrapped();
            }
            Message::EditInstanceJavaOverride(n) => {
                if let State::EditInstance(menu_edit_instance) = &mut self.state {
                    menu_edit_instance.config.java_override = Some(n);
                }
            }
            Message::EditInstanceMemoryChanged(new_slider_value) => {
                if let State::EditInstance(menu_edit_instance) = &mut self.state {
                    menu_edit_instance.slider_value = new_slider_value;
                    menu_edit_instance.config.ram_in_mb = 2f32.powf(new_slider_value) as usize;
                    menu_edit_instance.slider_text =
                        format_memory(menu_edit_instance.config.ram_in_mb);
                }
            }
            Message::ManageModsScreenOpen => {
                if let Err(err) = self.go_to_edit_mods_menu() {
                    self.set_error(err.to_string());
                }
            }
            Message::InstallFabricScreenOpen => {
                self.state = State::InstallFabric(MenuInstallFabric {
                    fabric_version: None,
                    fabric_versions: Vec::new(),
                    progress_receiver: None,
                    progress_num: 0.0,
                });

                return Command::perform(
                    instance_mod_installer::fabric::get_list_of_versions(),
                    Message::InstallFabricVersionsLoaded,
                );
            }
            Message::InstallFabricVersionsLoaded(result) => match result {
                Ok(list_of_versions) => {
                    if let State::InstallFabric(menu) = &mut self.state {
                        menu.fabric_versions = list_of_versions
                            .iter()
                            .map(|ver| ver.version.clone())
                            .collect();
                    }
                }
                Err(err) => self.set_error(err),
            },
            Message::InstallFabricVersionSelected(selection) => {
                if let State::InstallFabric(menu) = &mut self.state {
                    menu.fabric_version = Some(selection);
                }
            }
            Message::InstallFabricClicked => {
                if let State::InstallFabric(menu) = &mut self.state {
                    let (sender, receiver) = std::sync::mpsc::channel();
                    menu.progress_receiver = Some(receiver);

                    return Command::perform(
                        instance_mod_installer::fabric::install_wrapped(
                            menu.fabric_version.clone().unwrap(),
                            self.selected_instance.clone().unwrap(),
                            Some(sender),
                        ),
                        Message::InstallFabricEnd,
                    );
                }
            }
            Message::InstallFabricEnd(result) => match result {
                Ok(()) => self.go_to_launch_screen_with_message("Installed Fabric".to_owned()),
                Err(err) => self.set_error(err),
            },
            Message::OpenDir(dir) => open_file_explorer(&dir),
            Message::CreateInstanceChangeAssetToggle(toggle) => {
                if let State::Create(menu) = &mut self.state {
                    menu.download_assets = toggle;
                }
            }
            Message::ErrorCopy => {
                if let State::Error { error } = &self.state {
                    return iced::clipboard::write(format!("QuantumLauncher Error: {error}"));
                }
            }
            Message::Tick => return self.tick(),
            Message::UninstallLoaderStart => {
                if let State::EditMods(menu) = &self.state {
                    if menu.config.mod_type == "Fabric" {
                        return Command::perform(
                            instance_mod_installer::fabric::uninstall_wrapped(
                                self.selected_instance.clone().unwrap(),
                            ),
                            Message::UninstallLoaderEnd,
                        );
                    }
                    if menu.config.mod_type == "Forge" {
                        return Command::perform(
                            instance_mod_installer::forge::uninstall_wrapped(
                                self.selected_instance.clone().unwrap(),
                            ),
                            Message::UninstallLoaderEnd,
                        );
                    }
                }
            }
            Message::UninstallLoaderEnd(result) => {
                if let Err(err) = result {
                    self.set_error(err);
                } else {
                    self.go_to_launch_screen_with_message("Uninstalled Fabric".to_owned());
                }
            }
            Message::InstallForgeStart => {
                let (f_sender, f_receiver) = std::sync::mpsc::channel();
                let (j_sender, j_receiver) = std::sync::mpsc::channel();

                let command = Command::perform(
                    instance_mod_installer::forge::install_wrapped(
                        self.selected_instance.clone().unwrap(),
                        Some(f_sender),
                        Some(j_sender),
                    ),
                    Message::InstallForgeEnd,
                );

                self.state = State::InstallForge(MenuInstallForge {
                    forge_progress_receiver: f_receiver,
                    forge_progress_num: 0.0,
                    java_progress_receiver: j_receiver,
                    java_progress_num: 0.0,
                    is_java_getting_installed: false,
                    forge_message: "Installing Forge".to_owned(),
                    java_message: None,
                });

                return command;
            }
            Message::InstallForgeEnd(result) => match result {
                Ok(()) => self.go_to_launch_screen_with_message("Installed Forge".to_owned()),
                Err(err) => self.set_error(err),
            },
            Message::LaunchEndedLog(result) => match result {
                Ok(status) => {
                    info!("Game exited with status: {status}");
                    if !status.success() {
                        self.set_game_crashed(status);
                    }
                }
                Err(err) => self.set_error(err),
            },
            Message::LaunchKill => {
                if let Some(process) = self
                    .processes
                    .remove(self.selected_instance.as_ref().unwrap())
                {
                    return Command::perform(
                        {
                            async move {
                                let mut child = process.child.lock().unwrap();
                                child.start_kill().map_err(|err| err.to_string())
                            }
                        },
                        Message::LaunchKillEnd,
                    );
                }
            }
            Message::InstallModsDownloadComplete(result) => match result {
                Ok(id) => {
                    if let State::ModsDownload(menu) = &mut self.state {
                        menu.mods_download_in_progress.remove(&id);
                    }
                }
                Err(err) => self.set_error(err),
            },
            Message::TickConfigSaved(result) | Message::LaunchKillEnd(result) => {
                if let Err(err) = result {
                    self.set_error(err);
                }
            }
            Message::LaunchCopyLog => {
                if let Some(log) = self.logs.get(self.selected_instance.as_ref().unwrap()) {
                    return iced::clipboard::write(log.log.clone());
                }
            }
            Message::UpdateCheckResult(update_check_info) => match update_check_info {
                Ok(info) => match info {
                    UpdateCheckInfo::UpToDate => {
                        info!("Launcher is latest version. No new updates");
                    }
                    UpdateCheckInfo::NewVersion { url } => {
                        self.state = State::UpdateFound(MenuLauncherUpdate {
                            url,
                            receiver: None,
                            progress: 0.0,
                            progress_message: None,
                        });
                    }
                },
                Err(err) => {
                    err!("Could not check for updates: {err}");
                }
            },
            Message::UpdateDownloadStart => {
                if let State::UpdateFound(MenuLauncherUpdate {
                    url,
                    receiver,
                    progress_message,
                    ..
                }) = &mut self.state
                {
                    let (sender, update_receiver) = std::sync::mpsc::channel();
                    *receiver = Some(update_receiver);
                    *progress_message = Some("Starting Update".to_owned());

                    return Command::perform(
                        ql_instances::install_update_wrapped(url.clone(), sender),
                        Message::UpdateDownloadEnd,
                    );
                }
            }
            Message::UpdateDownloadEnd(err) => {
                if let Err(err) = err {
                    self.set_error(err);
                } else {
                    self.go_to_launch_screen_with_message(
                        "Updated launcher! Close and reopen the launcher to see the new update"
                            .to_owned(),
                    );
                }
            }
            Message::InstallModsSearchResult(search) => {
                if let State::ModsDownload(menu) = &mut self.state {
                    menu.is_loading_search = false;
                    match search {
                        Ok((search, time)) => {
                            if time > menu.latest_load {
                                menu.results = Some(search);
                                menu.latest_load = time;
                            }
                        }
                        Err(err) => self.set_error(err.to_string()),
                    }
                }
            }
            Message::InstallModsOpen => match self.open_mods_screen() {
                Ok(command) => return command,
                Err(err) => self.set_error(err),
            },
            Message::InstallModsSearchInput(input) => {
                if let State::ModsDownload(menu) = &mut self.state {
                    menu.query = input;

                    return menu.search_modrinth();
                }
            }
            Message::InstallModsClick(i) => {
                if let State::ModsDownload(menu) = &mut self.state {
                    menu.opened_mod = Some(i);
                    if let Some(results) = &menu.results {
                        let hit = results.hits.get(i).unwrap();
                        if !menu.result_data.contains_key(&hit.project_id) {
                            let task = ProjectInfo::download_wrapped(hit.project_id.clone());
                            return Command::perform(task, Message::InstallModsLoadData);
                        }
                    }
                }
            }
            Message::InstallModsBackToMainScreen => {
                if let State::ModsDownload(menu) = &mut self.state {
                    menu.opened_mod = None;
                }
            }
            Message::InstallModsLoadData(project_info) => match project_info {
                Ok(info) => {
                    if let State::ModsDownload(menu) = &mut self.state {
                        let id = info.id.clone();
                        menu.result_data.insert(id, *info);
                    }
                }
                Err(err) => self.set_error(err),
            },
            Message::InstallModsImageDownloaded(image) => match image {
                Ok((name, path)) => {
                    self.images.insert(name, Handle::from_memory(path));
                }
                Err(err) => {
                    err!("Could not download image: {err}");
                }
            },
            Message::InstallModsDownload(index) => {
                if let Some(value) = self.mod_download(index) {
                    return value;
                }
            }
            Message::ManageModsToggleCheckbox((name, id), enable) => {
                if let State::EditMods(menu) = &mut self.state {
                    if enable {
                        menu.selected_mods.insert(SelectedMod { name, id });
                        menu.selected_state = SelectedState::Some;
                    } else {
                        menu.selected_mods.remove(&SelectedMod { name, id });
                        menu.selected_state = if menu.selected_mods.is_empty() {
                            SelectedState::None
                        } else {
                            SelectedState::Some
                        };
                    }
                }
            }
            Message::ManageModsDeleteSelected => {
                if let State::EditMods(menu) = &self.state {
                    let ids = menu
                        .selected_mods
                        .iter()
                        .map(|SelectedMod { name: _name, id }| id.clone())
                        .collect();

                    return Command::perform(
                        ql_mod_manager::mod_manager::delete_mods_wrapped(
                            ids,
                            self.selected_instance.clone().unwrap(),
                        ),
                        Message::ManageModsDeleteFinished,
                    );

                    //                     Command::perform(
                    //     ql_mod_manager::modrinth::delete_mod_wrapped(
                    //         id.to_owned(),
                    //         self.selected_instance.clone().unwrap(),
                    //     ),
                    //     Message::ManageModsDeleteFinished,
                    // )
                }
            }
            Message::ManageModsDeleteFinished(result) => match result {
                Ok(_) => {
                    self.update_mod_index();
                }
                Err(err) => self.set_error(err),
            },
            Message::LauncherSettingsThemePicked(theme) => {
                info!("Setting theme {theme}");
                if let Some(config) = self.config.as_mut() {
                    config.theme = Some(theme.clone());
                }
                match theme.as_str() {
                    "Light" => self.theme = LauncherTheme::Light,
                    "Dark" => self.theme = LauncherTheme::Dark,
                    _ => err!("Invalid theme {theme}"),
                }
            }
            Message::LauncherSettingsOpen => {
                self.state = State::LauncherSettings;
            }
            Message::LauncherSettingsStylePicked(style) => {
                info!("Setting style {style}");
                if let Some(config) = self.config.as_mut() {
                    config.style = Some(style.clone());
                }
                match style.as_str() {
                    "Purple" => *self.style.lock().unwrap() = LauncherStyle::Purple,
                    "Brown" => *self.style.lock().unwrap() = LauncherStyle::Brown,
                    _ => err!("Invalid theme {style}"),
                }
            }
            Message::ManageModsSelectAll => {
                if let State::EditMods(menu) = &mut self.state {
                    match menu.selected_state {
                        SelectedState::All => {
                            menu.selected_mods.clear();
                            menu.selected_state = SelectedState::None;
                        }
                        SelectedState::Some | SelectedState::None => {
                            menu.selected_mods = menu
                                .mods
                                .mods
                                .iter()
                                .filter_map(|(id, mod_info)| {
                                    mod_info.manually_installed.then_some(SelectedMod {
                                        name: mod_info.name.clone(),
                                        id: id.clone(),
                                    })
                                })
                                .collect();
                            menu.selected_state = SelectedState::All;
                        }
                    }
                }
            }
            Message::EditInstanceLoggingToggle(t) => {
                if let State::EditInstance(menu) = &mut self.state {
                    menu.config.enable_logger = Some(t);
                }
            }
            Message::ManageModsToggleSelected => {
                if let State::EditMods(menu) = &self.state {
                    let ids = menu
                        .selected_mods
                        .iter()
                        .map(|SelectedMod { name: _name, id }| id.clone())
                        .collect();
                    return Command::perform(
                        ql_mod_manager::mod_manager::toggle_mods_wrapped(
                            ids,
                            self.selected_instance.clone().unwrap(),
                        ),
                        Message::ManageModsToggleFinished,
                    );
                }
            }
            Message::ManageModsToggleFinished(err) => {
                if let Err(err) = err {
                    self.set_error(err);
                } else {
                    self.update_mod_index();
                }
            }
            Message::InstallOptifineScreenOpen => {
                self.state = State::InstallOptifine(MenuInstallOptifine { progress: None });
            }
            Message::InstallOptifineSelectInstallerStart => {
                return Command::perform(
                    rfd::AsyncFileDialog::new()
                        .add_filter("jar", &["jar"])
                        .set_title("Select OptiFine Installer")
                        .pick_file(),
                    Message::InstallOptifineSelectInstallerEnd,
                )
            }
            Message::InstallOptifineSelectInstallerEnd(handle) => {
                if let Some(handle) = handle {
                    let path = handle.path().to_owned();

                    return Command::perform(
                        ql_mod_manager::instance_mod_installer::optifine::install_optifine_wrapped(
                            self.selected_instance.clone().unwrap(),
                            path,
                        ),
                        Message::InstallOptifineEnd,
                    );
                }
            }
            Message::InstallOptifineEnd(result) => {
                if let Err(err) = result {
                    self.set_error(err)
                } else {
                    self.go_to_launch_screen_with_message("Installed OptiFine".to_owned())
                }
            }
        }
        Command::none()
    }

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        const UPDATES_PER_SECOND: u64 = 12;

        iced::time::every(Duration::from_millis(1000 / UPDATES_PER_SECOND)).map(|_| Message::Tick)
    }

    fn view(&self) -> iced::Element<'_, Self::Message, Self::Theme, iced::Renderer> {
        match &self.state {
            State::Launch(menu) => menu.view(
                self.config.as_ref(),
                self.instances.as_deref(),
                &self.processes,
                &self.logs,
                self.selected_instance.as_ref(),
            ),
            State::EditInstance(menu) => menu.view(self.selected_instance.as_ref().unwrap()),
            State::EditMods(menu) => menu.view(self.selected_instance.as_ref().unwrap()),
            State::Create(menu) => menu.view(),
            State::DeleteInstance(menu) => menu.view(self.selected_instance.as_ref().unwrap()),
            State::Error { error } => widget::scrollable(
                widget::column!(
                    widget::text(format!("Error: {error}")),
                    widget::button("Back").on_press(Message::LaunchScreenOpen(None)),
                    widget::button("Copy Error").on_press(Message::ErrorCopy),
                )
                .padding(10)
                .spacing(10),
            )
            .into(),
            State::InstallFabric(menu) => menu.view(self.selected_instance.as_ref().unwrap()),
            State::InstallForge(menu) => menu.view(),
            State::UpdateFound(menu) => menu.view(),
            State::InstallJava(menu) => menu.view(),
            State::ModsDownload(menu) => menu.view(&self.images, &self.images_to_load),
            State::LauncherSettings => MenuLauncherSettings::view(self.config.as_ref()),
            State::RedownloadAssets(menu) => widget::column!(
                widget::text("Redownloading Assets").size(20),
                widget::progress_bar(0.0..=1.0, menu.num),
            )
            .padding(10)
            .spacing(20)
            .into(),
            State::InstallOptifine(menu) => menu.view(),
        }
    }

    fn theme(&self) -> Self::Theme {
        self.theme.clone()
    }

    fn scale_factor(&self) -> f64 {
        1.0
    }
}

impl Launcher {
    fn mod_download(&mut self, index: usize) -> Option<Command<Message>> {
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
        let Some(selected_instance) = &self.selected_instance else {
            return None;
        };

        menu.mods_download_in_progress
            .insert(hit.project_id.clone());
        Some(Command::perform(
            ql_mod_manager::mod_manager::download_mod_wrapped(
                hit.project_id.clone(),
                selected_instance.to_owned(),
            ),
            Message::InstallModsDownloadComplete,
        ))
    }

    fn set_game_crashed(&mut self, status: std::process::ExitStatus) {
        if let State::Launch(MenuLaunch { message, .. }) = &mut self.state {
            *message = format!("Game Crashed with code: {status}\nCheck Logs for more information");
            if let Some(log) = self.logs.get_mut(self.selected_instance.as_ref().unwrap()) {
                log.has_crashed = true;
            }
        }
    }

    fn update_mod_index(&mut self) {
        if let State::EditMods(menu) = &mut self.state {
            match ModIndex::get(self.selected_instance.as_ref().unwrap())
                .map_err(|err| err.to_string())
            {
                Ok(idx) => menu.mods = idx,
                Err(err) => self.set_error(err),
            }
        }
    }
}

// async fn pick_file() -> Option<PathBuf> {
//     const MESSAGE: &str = if cfg!(windows) {
//         "Select the java.exe executable"
//     } else {
//         "Select the java executable"
//     };

//     rfd::AsyncFileDialog::new()
//         .set_title(MESSAGE)
//         .pick_file()
//         .await
//         .map(|n| n.path().to_owned())
// }

const WINDOW_HEIGHT: f32 = 450.0;
const WINDOW_WIDTH: f32 = 650.0;

fn main() {
    let args = std::env::args();
    let mut info = ArgumentInfo {
        headless: false,
        operation: None,
        is_used: false,
    };
    process_args(args, &mut info);

    if !info.is_used {
        info!("Welcome to QuantumLauncher! This terminal window just outputs some debug info. You can ignore it.");
    }

    if let Some(op) = info.operation {
        match op {
            ArgumentOperation::ListInstances => {
                match reload_instances().map_err(|err| err.to_string()) {
                    Ok(instances) => {
                        for instance in instances {
                            let launcher_dir = file_utils::get_launcher_dir().unwrap();
                            let instance_dir = launcher_dir.join("instances").join(&instance);

                            let json =
                                std::fs::read_to_string(instance_dir.join("details.json")).unwrap();
                            let json: VersionDetails = serde_json::from_str(&json).unwrap();

                            let config_json =
                                std::fs::read_to_string(instance_dir.join("config.json")).unwrap();
                            let config_json: InstanceConfigJson =
                                serde_json::from_str(&config_json).unwrap();

                            println!("{instance} : {} : {}", json.id, config_json.mod_type);
                        }
                    }
                    Err(err) => eprintln!("[cmd.error] {err}"),
                }
            }
        }
    }

    if info.headless {
        return;
    }
    info!("Starting up the launcher...");

    Launcher::run(Settings {
        window: iced::window::Settings {
            size: iced::Size {
                width: WINDOW_WIDTH,
                height: WINDOW_HEIGHT,
            },
            resizable: true,
            ..Default::default()
        },
        fonts: vec![
            include_bytes!("../../assets/Inter-Regular.ttf")
                .as_slice()
                .into(),
            include_bytes!("../../assets/launcher-icons.ttf")
                .as_slice()
                .into(),
            include_bytes!("../../assets/JetBrainsMono-Regular.ttf")
                .as_slice()
                .into(),
        ],
        default_font: iced::Font::with_name("Inter"),
        ..Default::default()
    })
    .unwrap();
}

struct ArgumentInfo {
    pub headless: bool,
    pub is_used: bool,
    pub operation: Option<ArgumentOperation>,
}

enum ArgumentOperation {
    ListInstances,
}

fn process_args(mut args: std::env::Args, info: &mut ArgumentInfo) -> Option<()> {
    let program = args.next()?;
    let mut first_argument = true;

    loop {
        let Some(command) = args.next() else {
            if first_argument {
                info!(
                    "You can run {} to see the possible command line arguments",
                    format!("{program} --help").yellow()
                );
            }
            return None;
        };
        info.is_used = true;
        match command.as_str() {
            "--help" => {
                println!(
                    r#"Usage: {}
    --help           : Print a list of valid command line flags
    --version        : Print the launcher version
    --command        : Run a command with the launcher in headless mode (command line)
                       For more info, type {}
    --list-instances : Print a list of instances (name, version and type (Vanilla/Fabric/Forge/...))
"#,
                    format!("{program} [FLAGS]").yellow(),
                    format!("{program} --command help").yellow()
                );
            }
            "--version" => {
                println!(
                    "{}",
                    format!("QuantumLauncher v{LAUNCHER_VERSION_NAME} - made by Mrmayman").bold()
                );
            }
            "--command" => {
                info.headless = true;
            }
            "--list-instances" => info.operation = Some(ArgumentOperation::ListInstances),
            _ => {
                eprintln!(
                    "{} Unknown flag! Type {} to see all the command-line flags.",
                    "[error]".red(),
                    format!("{program} --help").yellow()
                );
            }
        }
        first_argument = false;
    }
}
