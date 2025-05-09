use std::str::FromStr;

use iced::{widget::image::Handle, Task};
use ql_core::{err, info, InstanceSelection, IntoStringError, ModId};
use ql_mod_manager::{loaders, store::get_description};

mod accounts;
mod create_instance;
mod edit_instance;
mod manage_mods;
mod presets;

use crate::{
    launcher_state::{
        self, InstallFabricMessage, InstallModsMessage, InstallOptifineMessage, Launcher,
        LauncherSettingsMessage, MenuInstallFabric, MenuInstallOptifine, Message, ProgressBar,
        State,
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
                    *fabric_version = Some(selection);
                }
            }
            InstallFabricMessage::VersionsLoaded(result) => match result {
                Ok(list_of_versions) => {
                    if let State::InstallFabric(menu) = &mut self.state {
                        if list_of_versions.is_empty() {
                            *menu = MenuInstallFabric::Unsupported(menu.is_quilt());
                        } else {
                            let first = list_of_versions.first().map(|n| n.loader.version.clone());
                            *menu = MenuInstallFabric::Loaded {
                                is_quilt: menu.is_quilt(),
                                fabric_version: first,
                                fabric_versions: list_of_versions
                                    .iter()
                                    .map(|ver| ver.loader.version.clone())
                                    .collect(),
                                progress: None,
                            };
                        }
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
                    let loader_version = fabric_version.clone().unwrap();

                    let instance_name = self.selected_instance.clone().unwrap();
                    let is_quilt = *is_quilt;
                    return Task::perform(
                        loaders::fabric::install(
                            loader_version,
                            instance_name,
                            Some(sender),
                            is_quilt,
                        ),
                        |m| Message::InstallFabric(InstallFabricMessage::End(m.strerr())),
                    );
                }
            }
            InstallFabricMessage::ScreenOpen { is_quilt } => {
                self.state = State::InstallFabric(MenuInstallFabric::Loading(is_quilt));

                let instance_name = self.selected_instance.clone().unwrap();
                return Task::perform(
                    loaders::fabric::get_list_of_versions(instance_name, is_quilt),
                    |m| Message::InstallFabric(InstallFabricMessage::VersionsLoaded(m.strerr())),
                );
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
                let scroll_px = viewport.absolute_offset().y;

                if let State::ModsDownload(menu) = &mut self.state {
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
            InstallModsMessage::Open => match self.open_mods_screen() {
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
            InstallModsMessage::DownloadComplete(Ok(id)) => {
                if let State::ModsDownload(menu) = &mut self.state {
                    menu.mods_download_in_progress.remove(&id);
                }
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
                    return menu.search_store(is_server, 0);
                }
            }
            InstallModsMessage::ChangeQueryType(query) => {
                if let State::ModsDownload(menu) = &mut self.state {
                    menu.query_type = query;
                    menu.results = None;
                    return menu.search_store(is_server, 0);
                }
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

        Some(Task::perform(
            async move {
                ql_mod_manager::store::download_mod(&id, &selected_instance)
                    .await
                    .map(|()| ModId::Modrinth(project_id))
            },
            |n| Message::InstallMods(InstallModsMessage::DownloadComplete(n.strerr())),
        ))
    }

    pub fn update_install_optifine(&mut self, message: InstallOptifineMessage) -> Task<Message> {
        match message {
            InstallOptifineMessage::ScreenOpen => {
                self.state = State::InstallOptifine(MenuInstallOptifine::default());
            }
            InstallOptifineMessage::SelectInstallerStart => {
                return Task::perform(
                    rfd::AsyncFileDialog::new()
                        .add_filter("jar", &["jar"])
                        .set_title("Select OptiFine Installer")
                        .pick_file(),
                    |n| Message::InstallOptifine(InstallOptifineMessage::SelectInstallerEnd(n)),
                )
            }
            InstallOptifineMessage::SelectInstallerEnd(handle) => {
                if let Some(handle) = handle {
                    let path = handle.path().to_owned();

                    let (p_sender, p_recv) = std::sync::mpsc::channel();
                    let (j_sender, j_recv) = std::sync::mpsc::channel();

                    self.state = State::InstallOptifine(MenuInstallOptifine {
                        optifine_install_progress: Some(ProgressBar::with_recv(p_recv)),
                        java_install_progress: Some(ProgressBar::with_recv(j_recv)),
                        is_java_being_installed: false,
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

    pub fn update_launcher_settings(&mut self, msg: LauncherSettingsMessage) {
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
                self.state = State::LauncherSettings(launcher_state::MenuLauncherSettings {
                    temp_scale: self.config.ui_scale.unwrap_or(1.0),
                });
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
                ql_instances::delete_java_installs();
            }
        }
    }
}
