use iced::{futures::executor::block_on, Task};
use ql_core::{
    err, err_no_log, file_utils::DirItem, info, info_no_log, open_file_explorer, InstanceSelection,
    IntoIoError, IntoStringError, LOGGER,
};
use ql_instances::UpdateCheckInfo;
use ql_mod_manager::loaders;
use tokio::io::AsyncWriteExt;

use crate::state::{
    LaunchTabId, Launcher, ManageModsMessage, MenuExportInstance, MenuLaunch, MenuLauncherUpdate,
    MenuServerCreate, MenuWelcome, Message, ProgressBar, ServerProcess, State,
};

impl Launcher {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Nothing | Message::CoreLogCleanComplete(Ok(())) => {}

            Message::CoreTickConfigSaved(result)
            | Message::LaunchKillEnd(result)
            | Message::UpdateDownloadEnd(result) => {
                if let Err(err) = result {
                    self.set_error(err);
                }
            }

            Message::UpdateCheckResult(Err(err)) | Message::CoreLogCleanComplete(Err(err)) => {
                err_no_log!("{err}");
            }

            Message::ServerCreateEnd(Err(err))
            | Message::ServerCreateVersionsLoaded(Err(err))
            | Message::UninstallLoaderEnd(Err(err))
            | Message::ServerManageStartServerFinish(Err(err))
            | Message::InstallForgeEnd(Err(err))
            | Message::LaunchEndedLog(Err(err))
            | Message::ServerManageEndedLog(Err(err))
            | Message::CoreListLoaded(Err(err)) => self.set_error(err),

            Message::WelcomeContinueToTheme => {
                self.state = State::Welcome(MenuWelcome::P2Theme);
            }
            Message::WelcomeContinueToAuth => {
                self.state = State::Welcome(MenuWelcome::P3Auth);
            }

            Message::Account(msg) => return self.update_account(msg),
            Message::ManageMods(message) => return self.update_manage_mods(message),
            Message::ManageJarMods(message) => return self.update_manage_jar_mods(message),
            Message::LaunchInstanceSelected { name, is_server } => {
                self.selected_instance = Some(InstanceSelection::new(&name, is_server));

                if let State::Launch(MenuLaunch { .. }) = self.state {
                    if let Err(err) = self.edit_instance() {
                        err!("Could not open edit instance menu: {err}");
                        if let State::Launch(MenuLaunch { edit_instance, .. }) = &mut self.state {
                            *edit_instance = None;
                        }
                    }
                }
            }
            Message::LaunchUsernameSet(username) => {
                self.config.username = username;
            }
            Message::LaunchStart => return self.launch_start(),
            Message::LaunchEnd(result) => return self.finish_launching(result),
            Message::CreateInstance(message) => return self.update_create_instance(message),
            Message::DeleteInstanceMenu => self.go_to_delete_instance_menu(),
            Message::DeleteInstance => return self.delete_instance_confirm(),

            Message::LaunchScreenOpen {
                message,
                clear_selection,
            } => {
                if clear_selection {
                    self.selected_instance = None;
                }
                return self.go_to_launch_screen(message);
            }
            Message::EditInstance(message) => match self.update_edit_instance(message) {
                Ok(n) => return n,
                Err(err) => self.set_error(err),
            },
            Message::InstallFabric(message) => return self.update_install_fabric(message),
            Message::CoreOpenLink(dir) => open_file_explorer(&dir),
            Message::CoreOpenPath(dir) => open_file_explorer(&dir),
            Message::CoreErrorCopy => {
                if let State::Error { error } = &self.state {
                    return iced::clipboard::write(format!("(QuantumLauncher): {error}"));
                }
            }
            Message::CoreErrorCopyLog => {
                let text = {
                    if let Some(logger) = LOGGER.as_ref() {
                        let logger = logger.lock().unwrap();
                        logger.text.clone()
                    } else {
                        Vec::new()
                    }
                };

                let mut log = String::new();
                for (line, _) in text {
                    log.push_str(&line);
                }
                return iced::clipboard::write(format!("QuantumLauncher Log:\n{log}"));
            }
            Message::CoreTick => {
                self.tick_timer = self.tick_timer.wrapping_add(1);
                let mut commands = self.get_imgs_to_load();
                let command = self.tick();
                commands.push(command);
                return Task::batch(commands);
            }
            Message::UninstallLoaderForgeStart => {
                let instance = self.selected_instance.clone().unwrap();
                return Task::perform(
                    async move { loaders::forge::uninstall(instance).await.strerr() },
                    Message::UninstallLoaderEnd,
                );
            }
            Message::UninstallLoaderOptiFineStart => {
                let instance_name = self
                    .selected_instance
                    .as_ref()
                    .unwrap()
                    .get_name()
                    .to_owned();
                return Task::perform(
                    async { loaders::optifine::uninstall(instance_name).await.strerr() },
                    Message::UninstallLoaderEnd,
                );
            }
            Message::UninstallLoaderFabricStart => {
                let instance_name = self.selected_instance.clone().unwrap();
                return Task::perform(
                    async move { loaders::fabric::uninstall(instance_name).await.strerr() },
                    Message::UninstallLoaderEnd,
                );
            }
            Message::UninstallLoaderEnd(Ok(())) => {
                match self.go_to_edit_mods_menu_without_update_check() {
                    Ok(n) => return n,
                    Err(err) => self.set_error(err),
                }
            }
            Message::InstallForgeStart { is_neoforge } => {
                return self.install_forge(is_neoforge);
            }
            Message::InstallForgeEnd(Ok(())) => {
                return self.go_to_main_menu_with_message(Some("Installed Forge/NeoForge"));
            }
            Message::LaunchEndedLog(Ok((status, name))) => {
                info!("Game exited with status: {status}");
                self.set_game_crashed(status, &name);
            }
            Message::LaunchKill => return self.kill_selected_instance(),
            Message::LaunchCopyLog => {
                if let Some(log) = self
                    .client_logs
                    .get(self.selected_instance.as_ref().unwrap().get_name())
                {
                    return iced::clipboard::write(log.log.join(""));
                }
            }
            Message::UpdateCheckResult(Ok(info)) => match info {
                UpdateCheckInfo::UpToDate => {
                    info_no_log!("Launcher is latest version. No new updates");
                }
                UpdateCheckInfo::NewVersion { url } => {
                    self.state = State::UpdateFound(MenuLauncherUpdate {
                        url,
                        progress: None,
                    });
                }
            },
            Message::UpdateDownloadStart => return self.update_download_start(),
            Message::LauncherSettings(msg) => self.update_launcher_settings(msg),

            Message::InstallOptifine(msg) => return self.update_install_optifine(msg),
            Message::ServerManageOpen {
                selected_server,
                message,
            } => {
                self.selected_instance = selected_server.map(InstanceSelection::Server);
                return self.go_to_server_manage_menu(message);
            }
            Message::ServerCreateScreenOpen => {
                if let Some(cache) = &self.server_version_list_cache {
                    self.state = State::ServerCreate(MenuServerCreate::Loaded {
                        name: String::new(),
                        versions: Box::new(iced::widget::combo_box::State::new(cache.clone())),
                        selected_version: None,
                    });
                } else {
                    self.state = State::ServerCreate(MenuServerCreate::LoadingList);

                    return Task::perform(
                        async move { ql_servers::list().await.strerr() },
                        Message::ServerCreateVersionsLoaded,
                    );
                }
            }
            Message::ServerCreateNameInput(new_name) => {
                if let State::ServerCreate(MenuServerCreate::Loaded { name, .. }) = &mut self.state
                {
                    *name = new_name;
                }
            }
            Message::ServerCreateVersionSelected(list_entry) => {
                if let State::ServerCreate(MenuServerCreate::Loaded {
                    selected_version, ..
                }) = &mut self.state
                {
                    *selected_version = Some(list_entry);
                }
            }
            Message::ServerCreateStart => {
                if let State::ServerCreate(MenuServerCreate::Loaded {
                    name,
                    selected_version: Some(selected_version),
                    ..
                }) = &mut self.state
                {
                    let (sender, receiver) = std::sync::mpsc::channel();

                    let name = name.clone();
                    let selected_version = selected_version.clone();
                    self.state = State::ServerCreate(MenuServerCreate::Downloading {
                        progress: ProgressBar::with_recv(receiver),
                    });
                    return Task::perform(
                        async move {
                            let sender = sender;
                            ql_servers::create_server(name, selected_version, Some(&sender))
                                .await
                                .strerr()
                        },
                        Message::ServerCreateEnd,
                    );
                }
            }
            Message::ServerCreateEnd(Ok(name)) => {
                self.selected_instance = Some(InstanceSelection::Server(name));
                return self.go_to_server_manage_menu(Some("Created Server".to_owned()));
            }
            Message::ServerCreateVersionsLoaded(Ok(vec)) => {
                self.server_version_list_cache = Some(vec.clone());
                self.state = State::ServerCreate(MenuServerCreate::Loaded {
                    versions: Box::new(iced::widget::combo_box::State::new(vec)),
                    selected_version: None,
                    name: String::new(),
                });
            }
            Message::ServerManageStartServer(server) => {
                self.server_logs.remove(&server);
                let (sender, receiver) = std::sync::mpsc::channel();
                self.java_recv = Some(ProgressBar::with_recv(receiver));

                if self.server_processes.contains_key(&server) {
                    err!("Server is already running");
                } else {
                    return Task::perform(
                        async move { ql_servers::run(server, sender).await.strerr() },
                        Message::ServerManageStartServerFinish,
                    );
                }
            }
            Message::ServerManageStartServerFinish(Ok((child, is_classic_server))) => {
                self.java_recv = None;
                return self.add_server_to_processes(child, is_classic_server);
            }
            Message::ServerManageEndedLog(Ok((status, name))) => {
                if status.success() {
                    info!("Server {name} stopped.");
                } else {
                    info!("Server {name} crashed with status: {status}");
                }

                // TODO: Implement server crash handling
                if let Some(log) = self.server_logs.get_mut(&name) {
                    log.has_crashed = !status.success();
                }
            }
            Message::ServerManageKillServer(server) => {
                self.kill_selected_server(&server);
            }
            Message::ServerManageEditCommand(selected_server, command) => {
                if let Some(log) = self.server_logs.get_mut(&selected_server) {
                    log.command = command;
                }
            }
            Message::ServerManageSubmitCommand(selected_server) => {
                if let (
                    Some(log),
                    Some(ServerProcess {
                        stdin: Some(stdin), ..
                    }),
                ) = (
                    self.server_logs.get_mut(&selected_server),
                    self.server_processes.get_mut(&selected_server),
                ) {
                    let log_cloned = format!("{}\n", log.command);
                    let future = stdin.write_all(log_cloned.as_bytes());
                    // Make the input command visible in the log
                    log.log.push(format!("> {}", log.command));

                    log.command.clear();
                    _ = block_on(future);
                }
            }
            Message::ServerManageCopyLog => {
                let name = self.selected_instance.as_ref().unwrap().get_name();
                if let Some(logs) = self.server_logs.get(name) {
                    return iced::clipboard::write(
                        logs.log.iter().fold(String::new(), |n, v| n + v + "\n"),
                    );
                }
            }
            Message::InstallPaperStart => {
                self.state = State::InstallPaper;
                let instance_name = self
                    .selected_instance
                    .as_ref()
                    .unwrap()
                    .get_name()
                    .to_owned();
                return Task::perform(
                    async move { loaders::paper::install(instance_name).await.strerr() },
                    Message::InstallPaperEnd,
                );
            }
            Message::InstallPaperEnd(result) => {
                if let Err(err) = result {
                    self.set_error(err);
                } else {
                    return self.go_to_server_manage_menu(Some("Installed Paper".to_owned()));
                }
            }
            Message::UninstallLoaderPaperStart => {
                let get_name = self
                    .selected_instance
                    .as_ref()
                    .unwrap()
                    .get_name()
                    .to_owned();
                return Task::perform(
                    async move { loaders::paper::uninstall(get_name).await.strerr() },
                    Message::UninstallLoaderEnd,
                );
            }
            Message::CoreListLoaded(Ok((list, is_server))) => {
                if is_server {
                    self.server_list = Some(list);
                } else {
                    self.client_list = Some(list);
                }
            }
            Message::CoreCopyText(txt) => {
                return iced::clipboard::write(txt);
            }
            Message::InstallMods(msg) => return self.update_install_mods(msg),
            Message::CoreOpenChangeLog => {
                self.state = State::ChangeLog;
            }
            Message::CoreOpenIntro => {
                self.state = State::Welcome(MenuWelcome::P1InitialScreen);
            }
            Message::EditPresets(msg) => match self.update_edit_presets(msg) {
                Ok(n) => return n,
                Err(err) => self.set_error(err),
            },
            Message::UninstallLoaderConfirm(msg, name) => {
                self.state = State::ConfirmAction {
                    msg1: format!("uninstall {name}?"),
                    msg2: "This should be fine, you can always reinstall it later".to_owned(),
                    yes: (*msg).clone(),
                    no: Message::ManageMods(ManageModsMessage::ScreenOpenWithoutUpdate),
                }
            }
            Message::CoreEvent(event, status) => return self.iced_event(event, status),
            Message::LaunchChangeTab(launch_tab_id) => {
                if let LaunchTabId::Edit = launch_tab_id {
                    if let Err(err) = self.edit_instance() {
                        err!("Could not open edit instance menu: {err}");
                        if let State::Launch(MenuLaunch { edit_instance, .. }) = &mut self.state {
                            *edit_instance = None;
                        }
                    }
                }
                if let State::Launch(MenuLaunch { tab, .. }) = &mut self.state {
                    *tab = launch_tab_id;
                }
            }
            Message::CoreLogToggle => {
                self.is_log_open = !self.is_log_open;
            }
            Message::CoreLogScroll(lines) => {
                let new_scroll = self.log_scroll - lines;
                if new_scroll >= 0 {
                    self.log_scroll = new_scroll;
                }
            }
            Message::CoreLogScrollAbsolute(lines) => {
                self.log_scroll = lines;
            }
            Message::LaunchLogScroll(lines) => {
                if let State::Launch(MenuLaunch { log_scroll, .. }) = &mut self.state {
                    let new_scroll = *log_scroll - lines;
                    if new_scroll >= 0 {
                        *log_scroll = new_scroll;
                    }
                }
            }
            Message::LaunchLogScrollAbsolute(lines) => {
                if let State::Launch(MenuLaunch { log_scroll, .. }) = &mut self.state {
                    *log_scroll = lines;
                }
            }
            Message::LaunchScrollSidebar(total) => {
                if let State::Launch(MenuLaunch { sidebar_height, .. }) = &mut self.state {
                    *sidebar_height = total;
                }
            }

            Message::ExportInstanceOpen => {
                self.state = State::ExportInstance(MenuExportInstance {
                    entries: None,
                    progress: None,
                });
                return Task::perform(
                    ql_core::file_utils::read_filenames_from_dir_ext(
                        self.selected_instance
                            .clone()
                            .unwrap()
                            .get_dot_minecraft_path(),
                    ),
                    |n| Message::ExportInstanceLoaded(n.strerr()),
                );
            }
            Message::ExportInstanceLoaded(res) => {
                let mut entries: Vec<(DirItem, bool)> = match res {
                    Ok(n) => n
                        .into_iter()
                        .map(|n| {
                            let enabled = !(n.name == ".fabric"
                                || n.name == "logs"
                                || n.name == "command_history.txt"
                                || n.name == "realms_persistence.json"
                                || n.name == "debug"
                                || n.name == ".cache"
                                // Common mods...
                                || n.name == "authlib-injector.log"
                                || n.name == "easy_npc"
                                || n.name == "CustomSkinLoader"
                                || n.name == ".bobby");
                            (n, enabled)
                        })
                        .filter(|(n, _)| {
                            !(n.name == "mod_index.json" || n.name == "launcher_profiles.json")
                        })
                        .collect(),
                    Err(err) => {
                        self.set_error(err);
                        return Task::none();
                    }
                };
                entries.sort_by(|(a, _), (b, _)| {
                    // Folders before files, and then sorted alphabetically
                    a.is_file.cmp(&b.is_file).then_with(|| a.name.cmp(&b.name))
                });
                if let State::ExportInstance(menu) = &mut self.state {
                    menu.entries = Some(entries);
                }
            }
            Message::ExportInstanceToggleItem(idx, t) => {
                if let State::ExportInstance(MenuExportInstance {
                    entries: Some(entries),
                    ..
                }) = &mut self.state
                {
                    if let Some((_, b)) = entries.get_mut(idx) {
                        *b = t;
                    }
                }
            }
            Message::ExportInstanceStart => {
                if let State::ExportInstance(MenuExportInstance {
                    entries: Some(entries),
                    progress,
                }) = &mut self.state
                {
                    let (send, recv) = std::sync::mpsc::channel();
                    *progress = Some(ProgressBar::with_recv(recv));

                    let exceptions = entries
                        .iter()
                        .filter_map(|(n, b)| (!b).then_some(format!(".minecraft/{}", n.name)))
                        .collect();

                    return Task::perform(
                        ql_packager::export_instance(
                            self.selected_instance.clone().unwrap(),
                            exceptions,
                            Some(send),
                        ),
                        |n| Message::ExportInstanceFinished(n.strerr()),
                    );
                }
            }
            Message::ExportInstanceFinished(res) => match res {
                Ok(bytes) => {
                    if let Some(path) = rfd::FileDialog::new().save_file() {
                        if let Err(err) = std::fs::write(&path, bytes).path(path) {
                            self.set_error(err);
                        } else {
                            return self.go_to_launch_screen(None::<String>);
                        }
                    }
                }
                Err(err) => self.set_error(err),
            },
        }
        Task::none()
    }
}
