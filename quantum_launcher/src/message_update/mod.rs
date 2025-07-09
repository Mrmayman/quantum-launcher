use std::str::FromStr;

use iced::{
    widget::{image::Handle, scrollable::AbsoluteOffset},
    Task,
};
use ql_core::{err, info, InstanceSelection, IntoStringError, ModId, OptifineUniqueVersion};
use ql_mod_manager::{
    loaders,
    store::{get_description, QueryType},
};

mod accounts;
mod create_instance;
mod edit_instance;
mod manage_mods;
mod presets;

use crate::{
    state::{
        self, InstallFabricMessage, InstallModsMessage, InstallOptifineMessage, Launcher,
        LauncherSettingsMessage, MenuCurseforgeManualDownload, MenuInstallFabric,
        MenuInstallOptifine, Message, ProgressBar, State,
    },
    stylesheet::styles::{LauncherThemeColor, LauncherThemeLightness},
};

impl Launcher {
    pub fn update_install_fabric(&mut self, message: InstallFabricMessage) -> Task<Message> {
        match message {
            InstallFabricMessage::End(result) => match result {
                Ok(is_quilt) => {
                    return self.go_to_main_menu_with_message(Some(if is_quilt {
                        "Installed Quilt"
                    } else {
                        "Installed Fabric"
                    }));
                }
                Err(err) => self.set_error(err),
            },
            InstallFabricMessage::VersionSelected(selection) => {
                if let State::InstallFabric(MenuInstallFabric::Loaded { fabric_version, .. }) =
                    &mut self.state
                {
                    *fabric_version = selection;
                }
            }
            InstallFabricMessage::VersionsLoaded(result) => match result {
                Ok(list_of_versions) => {
                    if let State::InstallFabric(menu) = &mut self.state {
                        *menu = if let Some(first) = list_of_versions.first().cloned() {
                            MenuInstallFabric::Loaded {
                                is_quilt: menu.is_quilt(),
                                fabric_version: first.loader.version.clone(),
                                fabric_versions: list_of_versions
                                    .iter()
                                    .map(|ver| ver.loader.version.clone())
                                    .collect(),
                                progress: None,
                            }
                        } else {
                            MenuInstallFabric::Unsupported(menu.is_quilt())
                        };
                    }
                }
                Err(err) => self.set_error(err),
            },
            InstallFabricMessage::ButtonClicked => {
                if let State::InstallFabric(MenuInstallFabric::Loaded {
                    fabric_version,
                    progress,
                    is_quilt,
                    ..
                }) = &mut self.state
                {
                    let (sender, receiver) = std::sync::mpsc::channel();
                    *progress = Some(ProgressBar::with_recv(receiver));
                    let loader_version = fabric_version.clone();

                    let instance_name = self.selected_instance.clone().unwrap();
                    let is_quilt = *is_quilt;
                    return Task::perform(
                        async move {
                            loaders::fabric::install(
                                Some(loader_version),
                                instance_name,
                                Some(&sender),
                                is_quilt,
                            )
                            .await
                        },
                        |m| Message::InstallFabric(InstallFabricMessage::End(m.strerr())),
                    );
                }
            }
            InstallFabricMessage::ScreenOpen { is_quilt } => {
                let instance_name = self.selected_instance.clone().unwrap();
                let (task, handle) = Task::perform(
                    loaders::fabric::get_list_of_versions(instance_name, is_quilt),
                    |m| Message::InstallFabric(InstallFabricMessage::VersionsLoaded(m.strerr())),
                )
                .abortable();

                self.state = State::InstallFabric(MenuInstallFabric::Loading {
                    is_quilt,
                    _loading_handle: handle.abort_on_drop(),
                });

                return task;
            }
        }
        Task::none()
    }

    pub fn update_install_mods(&mut self, message: InstallModsMessage) -> Task<Message> {
        let is_server = matches!(&self.selected_instance, Some(InstanceSelection::Server(_)));

        match message {
            InstallModsMessage::LoadData(Err(err))
            | InstallModsMessage::DownloadComplete(Err(err))
            | InstallModsMessage::SearchResult(Err(err))
            | InstallModsMessage::IndexUpdated(Err(err)) => {
                self.set_error(err);
            }

            InstallModsMessage::SearchResult(Ok(search)) => {
                if let State::ModsDownload(menu) = &mut self.state {
                    menu.is_loading_continuation = false;

                    if search.start_time > menu.latest_load {
                        menu.latest_load = search.start_time;

                        if let (Some(results), true) = (&mut menu.results, search.offset > 0) {
                            results.mods.extend(search.mods);
                        } else {
                            menu.results = Some(search);
                        }
                    }
                }
            }
            InstallModsMessage::Scrolled(viewport) => {
                let total_height =
                    viewport.content_bounds().height - (viewport.bounds().height * 2.0);
                let absolute_offset = viewport.absolute_offset();
                let scroll_px = absolute_offset.y;

                if let State::ModsDownload(menu) = &mut self.state {
                    menu.scroll_offset = absolute_offset;
                    if (scroll_px > total_height) && !menu.is_loading_continuation {
                        menu.is_loading_continuation = true;

                        let offset = if let Some(results) = &menu.results {
                            results.offset + results.mods.len()
                        } else {
                            0
                        };
                        return menu.search_store(is_server, offset);
                    }
                }
            }
            InstallModsMessage::Open => match self.open_mods_store() {
                Ok(command) => return command,
                Err(err) => self.set_error(err),
            },
            InstallModsMessage::SearchInput(input) => {
                if let State::ModsDownload(menu) = &mut self.state {
                    menu.query = input;
                    return menu.search_store(is_server, 0);
                }
            }
            InstallModsMessage::ImageDownloaded(image) => match image {
                Ok(image) => {
                    self.insert_image(image);
                }
                Err(err) => {
                    err!("Could not download image: {err}");
                }
            },
            InstallModsMessage::Click(i) => {
                if let State::ModsDownload(menu) = &mut self.state {
                    menu.opened_mod = Some(i);
                    if let Some(results) = &menu.results {
                        let hit = results.mods.get(i).unwrap();
                        if !menu
                            .mod_descriptions
                            .contains_key(&ModId::from_pair(&hit.id, results.backend))
                        {
                            let backend = menu.backend;
                            let id = ModId::from_pair(&hit.id, backend);

                            return Task::perform(get_description(id), |n| {
                                Message::InstallMods(InstallModsMessage::LoadData(n.strerr()))
                            });
                        }
                    }
                }
            }
            InstallModsMessage::BackToMainScreen => {
                if let State::ModsDownload(menu) = &mut self.state {
                    menu.opened_mod = None;
                    return iced::widget::scrollable::scroll_to(
                        iced::widget::scrollable::Id::new("MenuModsDownload:main:mods_list"),
                        menu.scroll_offset,
                    );
                }
            }
            InstallModsMessage::LoadData(Ok((id, description))) => {
                if let State::ModsDownload(menu) = &mut self.state {
                    menu.mod_descriptions.insert(id, description);
                }
            }
            InstallModsMessage::Download(index) => {
                if let Some(value) = self.mod_download(index) {
                    return value;
                }
            }
            InstallModsMessage::DownloadComplete(Ok((id, not_allowed))) => {
                let task = if let State::ModsDownload(menu) = &mut self.state {
                    menu.mods_download_in_progress.remove(&id);
                    Task::none()
                } else {
                    match self.open_mods_store() {
                        Ok(n) => n,
                        Err(err) => {
                            self.set_error(err);
                            Task::none()
                        }
                    }
                };

                if not_allowed.is_empty() {
                    return task;
                }
                self.state = State::CurseforgeManualDownload(MenuCurseforgeManualDownload {
                    unsupported: not_allowed,
                    is_store: true,
                });
            }
            InstallModsMessage::IndexUpdated(Ok(idx)) => {
                if let State::ModsDownload(menu) = &mut self.state {
                    menu.mod_index = idx;
                }
            }

            InstallModsMessage::ChangeBackend(backend) => {
                if let State::ModsDownload(menu) = &mut self.state {
                    menu.backend = backend;
                    menu.results = None;
                    menu.scroll_offset = AbsoluteOffset::default();
                    return menu.search_store(is_server, 0);
                }
            }
            InstallModsMessage::ChangeQueryType(query) => {
                if let State::ModsDownload(menu) = &mut self.state {
                    menu.query_type = query;
                    menu.results = None;
                    menu.scroll_offset = AbsoluteOffset::default();
                    return menu.search_store(is_server, 0);
                }
            }
            InstallModsMessage::InstallModpack(id) => {
                let (sender, receiver) = std::sync::mpsc::channel();
                self.state = State::ImportModpack(ProgressBar::with_recv(receiver));

                let selected_instance = self.selected_instance.clone().unwrap();

                return Task::perform(
                    async move {
                        ql_mod_manager::store::download_mod(&id, &selected_instance, Some(sender))
                            .await
                            .map(|not_allowed| (id, not_allowed))
                    },
                    |n| Message::InstallMods(InstallModsMessage::DownloadComplete(n.strerr())),
                );
            }
        }
        Task::none()
    }

    fn insert_image(&mut self, image: ql_mod_manager::store::ImageResult) {
        if image.is_svg {
            let handle = iced::widget::svg::Handle::from_memory(image.image);
            self.images.svg.insert(image.url, handle);
        } else {
            self.images
                .bitmap
                .insert(image.url, Handle::from_bytes(image.image));
        }
    }

    fn mod_download(&mut self, index: usize) -> Option<Task<Message>> {
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

        if let QueryType::ModPacks = menu.query_type {
            self.state = State::ConfirmAction {
                msg1: format!("install the modpack: {}", hit.title),
                msg2: "This might take a while, install many files, and use a lot of network..."
                    .to_owned(),
                yes: Message::InstallMods(InstallModsMessage::InstallModpack(id)),
                no: Message::InstallMods(InstallModsMessage::Open),
            };
            None
        } else {
            Some(Task::perform(
                async move {
                    ql_mod_manager::store::download_mod(&id, &selected_instance, None)
                        .await
                        .map(|not_allowed| (ModId::Modrinth(project_id), not_allowed))
                },
                |n| Message::InstallMods(InstallModsMessage::DownloadComplete(n.strerr())),
            ))
        }
    }

    pub fn update_install_optifine(&mut self, message: InstallOptifineMessage) -> Task<Message> {
        match message {
            InstallOptifineMessage::ScreenOpen => {
                let optifine_unique_version =
                    OptifineUniqueVersion::get(self.selected_instance.as_ref().unwrap());

                if let Some(version @ OptifineUniqueVersion::B1_7_3) = optifine_unique_version {
                    self.state = State::InstallOptifine(MenuInstallOptifine {
                        optifine_install_progress: None,
                        java_install_progress: None,
                        is_java_being_installed: false,
                        is_b173_being_installed: true,
                        optifine_unique_version: Some(version),
                    });

                    let selected_instance = self.selected_instance.clone().unwrap();
                    let url = version.get_url().0;
                    return Task::perform(
                        ql_mod_manager::loaders::optifine::install_b173(selected_instance, url),
                        |n| Message::InstallOptifine(InstallOptifineMessage::End(n.strerr())),
                    );
                }

                self.state = State::InstallOptifine(MenuInstallOptifine {
                    optifine_install_progress: None,
                    java_install_progress: None,
                    is_java_being_installed: false,
                    is_b173_being_installed: false,
                    optifine_unique_version,
                });
            }
            InstallOptifineMessage::SelectInstallerStart => {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("jar/zip", &["jar", "zip"])
                    .set_title("Select OptiFine Installer")
                    .pick_file()
                {
                    let (p_sender, p_recv) = std::sync::mpsc::channel();
                    let (j_sender, j_recv) = std::sync::mpsc::channel();

                    let instance = self.selected_instance.as_ref().unwrap();
                    let optifine_unique_version = OptifineUniqueVersion::get(instance);

                    if let Some(OptifineUniqueVersion::B1_7_3) = optifine_unique_version {}

                    self.state = State::InstallOptifine(MenuInstallOptifine {
                        optifine_install_progress: Some(ProgressBar::with_recv(p_recv)),
                        java_install_progress: Some(ProgressBar::with_recv(j_recv)),
                        is_java_being_installed: false,
                        is_b173_being_installed: false,
                        optifine_unique_version,
                    });

                    let get_name = self
                        .selected_instance
                        .as_ref()
                        .unwrap()
                        .get_name()
                        .to_owned();
                    return Task::perform(
                        // Note: OptiFine does not support servers
                        // so it's safe to assume we've selected an instance.
                        ql_mod_manager::loaders::optifine::install(
                            get_name,
                            path,
                            Some(p_sender),
                            Some(j_sender),
                            optifine_unique_version.is_some(),
                        ),
                        |n| Message::InstallOptifine(InstallOptifineMessage::End(n.strerr())),
                    );
                }
            }
            InstallOptifineMessage::End(result) => {
                if let Err(err) = result {
                    self.set_error(err);
                } else {
                    return self.go_to_launch_screen(Some("Installed OptiFine".to_owned()));
                }
            }
        }
        Task::none()
    }

    pub fn update_launcher_settings(&mut self, msg: LauncherSettingsMessage) -> Task<Message> {
        match msg {
            LauncherSettingsMessage::ThemePicked(theme) => {
                info!("Setting color mode {theme}");
                self.config.theme = Some(theme.clone());

                match theme.as_str() {
                    "Light" => self.theme.lightness = LauncherThemeLightness::Light,
                    "Dark" => self.theme.lightness = LauncherThemeLightness::Dark,
                    _ => err!("Invalid color mode {theme}"),
                }
            }
            LauncherSettingsMessage::Open => {
                self.go_to_launcher_settings();
            }
            LauncherSettingsMessage::StylePicked(style) => {
                info!("Setting color scheme {style}");
                self.config.style = Some(style.clone());
                self.theme.color = LauncherThemeColor::from_str(&style).unwrap_or_default();
            }
            LauncherSettingsMessage::UiScale(scale) => {
                if let State::LauncherSettings(menu) = &mut self.state {
                    menu.temp_scale = scale;
                }
            }
            LauncherSettingsMessage::UiScaleApply => {
                if let State::LauncherSettings(menu) = &self.state {
                    self.config.ui_scale = Some(menu.temp_scale);
                }
            }
            LauncherSettingsMessage::ClearJavaInstalls => {
                self.state = State::ConfirmAction {
                    msg1: "delete auto-installed Java files".to_owned(),
                    msg2: "They will get reinstalled automatically as needed".to_owned(),
                    yes: Message::LauncherSettings(
                        LauncherSettingsMessage::ClearJavaInstallsConfirm,
                    ),
                    no: Message::LauncherSettings(LauncherSettingsMessage::ChangeTab(
                        state::LauncherSettingsTab::Internal,
                    )),
                }
            }
            LauncherSettingsMessage::ClearJavaInstallsConfirm => {
                return Task::perform(ql_instances::delete_java_installs(), |()| Message::Nothing);
            }
            LauncherSettingsMessage::ChangeTab(tab) => {
                self.go_to_launcher_settings();
                if let State::LauncherSettings(menu) = &mut self.state {
                    menu.selected_tab = tab;
                }
            }
        }
        Task::none()
    }

    pub fn go_to_launcher_settings(&mut self) {
        if let State::LauncherSettings(_) = &self.state {
            return;
        }
        self.state = State::LauncherSettings(state::MenuLauncherSettings {
            temp_scale: self.config.ui_scale.unwrap_or(1.0),
            selected_tab: state::LauncherSettingsTab::UserInterface,
        });
    }
}
