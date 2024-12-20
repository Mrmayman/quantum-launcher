use std::{collections::HashMap, ops::RangeInclusive};

use iced::widget;
use ql_core::file_utils;
use ql_instances::LAUNCHER_VERSION_NAME;

use crate::{
    config::LauncherConfig,
    icon_manager,
    launcher_state::{
        CreateInstanceMessage, EditInstanceMessage, GameProcess, InstallFabricMessage, InstanceLog,
        Launcher, MenuCreateInstance, MenuEditInstance, MenuEditMods, MenuInstallFabric,
        MenuInstallForge, MenuInstallJava, MenuInstallOptifine, MenuLaunch, MenuLauncherSettings,
        MenuLauncherUpdate, Message, ModListEntry, SelectedMod, SelectedState,
    },
    stylesheet::styles::LauncherTheme,
};

mod html;
pub mod mods_store;

pub type Element<'a> =
    iced::Element<'a, Message, <Launcher as iced::Application>::Theme, iced::Renderer>;

pub fn button_with_icon<'element>(
    icon: Element<'element>,
    text: &'element str,
) -> iced::widget::Button<'element, Message, LauncherTheme> {
    widget::button(widget::row![icon, text].spacing(10).padding(5))
}

impl MenuLaunch {
    pub fn view<'element>(
        &'element self,
        config: Option<&'element LauncherConfig>,
        instances: Option<&'element [String]>,
        processes: &'element HashMap<String, GameProcess>,
        logs: &'element HashMap<String, InstanceLog>,
        selected_instance: Option<&'element String>,
    ) -> Element<'element> {
        let pick_list = get_instances_section(instances, selected_instance);
        let footer_text = self.get_footer_text();

        let left_elements =
            get_left_pane(config, pick_list, selected_instance, processes, footer_text);

        let log = Self::get_log_pane(logs, selected_instance);

        widget::row!(widget::scrollable(left_elements), log)
            .padding(10)
            .spacing(20)
            .into()
    }

    fn get_footer_text(&self) -> Element {
        let version_message = widget::text(format!(
            "QuantumLauncher v{LAUNCHER_VERSION_NAME}\nA Minecraft Launcher by Mrmayman"
        ))
        .size(12);

        if self.message.is_empty() {
            widget::column!(version_message)
        } else {
            widget::column!(
                widget::container(widget::text(&self.message).size(14))
                    .width(200)
                    .padding(10),
                version_message
            )
        }
        .spacing(10)
        .into()
    }

    fn get_log_pane<'element>(
        logs: &HashMap<String, InstanceLog>,
        selected_instance: Option<&'element String>,
    ) -> widget::Column<'element, Message, LauncherTheme> {
        const LOG_VIEW_LIMIT: usize = 10000;
        if let Some(Some(InstanceLog { log, has_crashed })) = selected_instance
            .as_ref()
            .map(|selection| logs.get(*selection))
        {
            let log_length = log.len();
            let slice = if log_length > LOG_VIEW_LIMIT {
                &log[log_length - LOG_VIEW_LIMIT..log_length]
            } else {
                log
            };
            widget::column!(
                "Having issues? Copy and send the game log for support",
                widget::button("Copy Log").on_press(Message::LaunchCopyLog),
                if *has_crashed {
                    widget::column!(
                        widget::text("The game has crashed!").size(14),
                        widget::text("Go to Edit -> Enable Logging (disable it) then launch the game again.").size(12),
                        widget::text("Then copy the text in the second terminal window for crash information").size(12)
                    )
                } else {
                    widget::column![]
                },
                widget::scrollable(
                    widget::text(slice)
                        .size(12)
                        .font(iced::Font::with_name("JetBrains Mono"))
                )
            )
        } else {
            widget::column!("Select an instance to view its logs")
        }
        .padding(10)
        .spacing(10)
    }
}

fn get_left_pane<'a>(
    config: Option<&'a LauncherConfig>,
    pick_list: Element<'a>,
    selected_instance: Option<&'a String>,
    processes: &'a HashMap<String, GameProcess>,
    footer_text: Element<'a>,
) -> Element<'a> {
    let username = &config.as_ref().unwrap().username;

    let play_button = get_play_button(username, selected_instance, processes);

    let files_button = get_files_button(selected_instance);

    widget::column![
        widget::column![
            "Username:",
            widget::text_input("Enter username...", username)
                .on_input(Message::LaunchUsernameSet)
                .width(200),
        ]
        .spacing(5),
        pick_list,
        widget::column!(
            widget::row![files_button, play_button].spacing(5),
            widget::row!(
                button_with_icon(icon_manager::settings(), "Settings & About...")
                    .width(200)
                    .on_press(Message::LauncherSettingsOpen),
            )
        )
        .spacing(5),
        footer_text
    ]
    .padding(10)
    .spacing(20)
    .into()
}

fn get_play_button<'a>(
    username: &'a str,
    selected_instance: Option<&'a String>,
    processes: &'a HashMap<String, GameProcess>,
) -> widget::Column<'a, Message, LauncherTheme> {
    let play_button = button_with_icon(icon_manager::play(), "Play").width(98);

    let play_button = if username.is_empty() {
        widget::column!(widget::tooltip(
            play_button,
            widget::text("Username is empty!").size(12),
            widget::tooltip::Position::FollowCursor,
        ))
    } else if username.contains(' ') {
        widget::column!(widget::tooltip(
            play_button,
            widget::text("Username contains spaces!").size(12),
            widget::tooltip::Position::FollowCursor,
        ))
    } else if let Some(selected_instance) = selected_instance {
        widget::column!(if processes.contains_key(selected_instance) {
            button_with_icon(icon_manager::play(), "Kill")
                .on_press(Message::LaunchKill)
                .width(98)
        } else {
            play_button.on_press(Message::LaunchStart)
        })
    } else {
        widget::column!(widget::tooltip(
            play_button,
            widget::text("Select an instance first!").size(12),
            widget::tooltip::Position::FollowCursor,
        ))
    };
    play_button
}

fn get_files_button(selected_instance: Option<&String>) -> widget::Button<Message, LauncherTheme> {
    button_with_icon(icon_manager::folder(), "Files")
        .on_press_maybe((selected_instance.is_some()).then(|| {
            let launcher_dir = file_utils::get_launcher_dir().unwrap();
            Message::CoreOpenDir(
                launcher_dir
                    .join("instances")
                    .join(selected_instance.as_ref().unwrap())
                    .join(".minecraft")
                    .to_str()
                    .unwrap()
                    .to_owned(),
            )
        }))
        .width(97)
}

fn get_instances_section<'a>(
    instances: Option<&'a [String]>,
    selected_instance: Option<&'a String>,
) -> Element<'a> {
    if let Some(instances) = instances {
        widget::column![
            "Instances:",
            widget::pick_list(
                instances,
                selected_instance,
                Message::LaunchInstanceSelected,
            )
            .width(200),
            widget::row![
                button_with_icon(icon_manager::create(), "New")
                    .on_press(Message::CreateInstance(CreateInstanceMessage::ScreenOpen))
                    .width(97),
                button_with_icon(icon_manager::delete(), "Delete")
                    .on_press_maybe(
                        (selected_instance.is_some()).then_some(Message::DeleteInstanceMenu)
                    )
                    .width(98),
            ]
            .spacing(5),
            widget::row![
                button_with_icon(icon_manager::settings(), "Edit")
                    .on_press_maybe(
                        (selected_instance.is_some())
                            .then_some(Message::EditInstance(EditInstanceMessage::MenuOpen))
                    )
                    .width(97),
                button_with_icon(icon_manager::download(), "Mods")
                    .on_press_maybe(
                        (selected_instance.is_some()).then_some(Message::ManageModsScreenOpen)
                    )
                    .width(98),
            ]
            .spacing(5),
        ]
    } else {
        widget::column!["Loading instances..."]
    }
    .spacing(5)
    .into()
}

impl MenuEditInstance {
    pub fn view<'a>(&'a self, selected_instance: &'a str) -> Element<'a> {
        // 2 ^ 8 = 256 MB
        const MEM_256_MB_IN_TWOS_EXPONENT: f32 = 8.0;
        // 2 ^ 13 = 8192 MB
        const MEM_8192_MB_IN_TWOS_EXPONENT: f32 = 13.0;

        widget::scrollable(
            widget::column![
                widget::button(widget::row![icon_manager::back(), "Back"]
                    .spacing(10)
                    .padding(5)
                ).on_press(Message::LaunchScreenOpen(None)),
                widget::text(format!("Editing {} instance: {}", self.config.mod_type, selected_instance)),
                widget::container(
                    widget::column![
                        "Use a special Java install instead of the default one. (Enter path, leave blank if none)",
                        widget::text_input(
                            "Enter Java override path...",
                            self.config
                                .java_override
                                .as_deref()
                                .unwrap_or_default()
                        )
                        .on_input(|t| Message::EditInstance(EditInstanceMessage::JavaOverride(t)))
                    ]
                    .padding(10)
                    .spacing(10)
                ),
                widget::container(
                    widget::column![
                        "Allocated memory",
                        widget::text("For normal Minecraft, allocate 2 - 3 GB").size(12),
                        widget::text("For old versions, allocate 512 MB - 1 GB").size(12),
                        widget::text("For heavy modpacks or very high render distances, allocate 4 - 8 GB").size(12),
                        widget::slider(MEM_256_MB_IN_TWOS_EXPONENT..=MEM_8192_MB_IN_TWOS_EXPONENT, self.slider_value, |n| Message::EditInstance(EditInstanceMessage::MemoryChanged(n))).step(0.1),
                        widget::text(&self.slider_text),
                    ]
                    .padding(10)
                    .spacing(5),
                ),
                widget::container(
                    widget::column![
                        widget::checkbox("Enable logging", self.config.enable_logger.unwrap_or(true))
                            .on_toggle(|t| Message::EditInstance(EditInstanceMessage::LoggingToggle(t))),
                        widget::text("Enabled by default, disable if you want to see some advanced crash messages in the terminal.").size(12)
                    ]
                    .padding(10)
                    .spacing(10)
                ),
                widget::container(
                    widget::column!(
                        "Java arguments:",
                        widget::column!(
                            Self::get_java_args_list(
                                self.config.java_args.as_ref(),
                                |n| Message::EditInstance(EditInstanceMessage::JavaArgDelete(n)),
                                |n| Message::EditInstance(EditInstanceMessage::JavaArgShiftUp(n)),
                                |n| Message::EditInstance(EditInstanceMessage::JavaArgShiftDown(n)),
                                &|n, i| Message::EditInstance(EditInstanceMessage::JavaArgEdit(n, i))
                            ),
                            button_with_icon(icon_manager::create(), "Add")
                                .on_press(Message::EditInstance(EditInstanceMessage::JavaArgsAdd))
                        )
                    ).padding(10).spacing(10)
                ),
                widget::container(
                    widget::column!(
                        "Game arguments:",
                        widget::column!(
                            Self::get_java_args_list(
                                self.config.game_args.as_ref(),
                                |n| Message::EditInstance(EditInstanceMessage::GameArgDelete(n)),
                                |n| Message::EditInstance(EditInstanceMessage::GameArgShiftUp(n)),
                                |n| Message::EditInstance(EditInstanceMessage::GameArgShiftDown(n)),
                                &|n, i| Message::EditInstance(EditInstanceMessage::GameArgEdit(n, i))
                            ),
                            button_with_icon(icon_manager::create(), "Add")
                                .on_press(Message::EditInstance(EditInstanceMessage::GameArgsAdd))
                        )
                    ).padding(10).spacing(10)
                )
            ]
            .padding(10)
            .spacing(20)
        ).into()
    }

    fn get_java_args_list<'a>(
        args: Option<&'a Vec<String>>,
        mut msg_delete: impl FnMut(usize) -> Message,
        mut msg_up: impl FnMut(usize) -> Message,
        mut msg_down: impl FnMut(usize) -> Message,
        edit_msg: &'a dyn Fn(String, usize) -> Message,
    ) -> Element<'a> {
        const ITEM_SIZE: u16 = 10;

        let Some(args) = args else {
            return widget::column!().into();
        };
        widget::column(args.iter().enumerate().map(|(i, arg)| {
            widget::row!(
                widget::button(
                    widget::row![icon_manager::delete_with_size(ITEM_SIZE)]
                        .align_items(iced::Alignment::Center)
                        .padding(5)
                )
                .on_press(msg_delete(i)),
                widget::button(
                    widget::row![icon_manager::arrow_up_with_size(ITEM_SIZE)]
                        .align_items(iced::Alignment::Center)
                        .padding(5)
                )
                .on_press(msg_up(i)),
                widget::button(
                    widget::row![icon_manager::arrow_down_with_size(ITEM_SIZE)]
                        .align_items(iced::Alignment::Center)
                        .padding(5)
                )
                .on_press(msg_down(i)),
                widget::text_input("Enter argument...", arg)
                    .size(ITEM_SIZE + 8)
                    .on_input(move |n| edit_msg(n, i))
            )
            .into()
        }))
        .into()
    }
}

const OPTIFINE_DOWNLOADS: &str = "https://optifine.net/downloads";

impl MenuInstallOptifine {
    pub fn view(&self) -> Element {
        if let Some(progress) = &self.progress {
            widget::column!(
                widget::text("Installing OptiFine...").size(20),
                widget::progress_bar(0.0..=3.0, progress.optifine_install_num),
                widget::text(&progress.optifine_install_message),
                if progress.is_java_being_installed {
                    widget::column!(widget::container(
                        widget::column!(
                            "Installing Java",
                            widget::progress_bar(0.0..=1.0, progress.java_install_num),
                            widget::text(&progress.java_install_message),
                        )
                        .spacing(10)
                        .padding(10)
                    ))
                } else {
                    widget::column!()
                },
            )
        } else {
            widget::column!(
                button_with_icon(icon_manager::back(), "Back")
                    .on_press(Message::ManageModsScreenOpen),
                widget::container(
                    widget::column!(
                        widget::text("Install OptiFine").size(20),
                        "Step 1: Open the OptiFine download page and download the installer.",
                        "WARNING: Make sure to download the correct version.",
                        widget::button("Open download page")
                            .on_press(Message::CoreOpenDir(OPTIFINE_DOWNLOADS.to_owned()))
                    )
                    .padding(10)
                    .spacing(10)
                ),
                widget::container(
                    widget::column!(
                        "Step 2: Select the installer file",
                        widget::button("Select File")
                            .on_press(Message::InstallOptifineSelectInstallerStart)
                    )
                    .padding(10)
                    .spacing(10)
                )
            )
        }
        .padding(10)
        .spacing(10)
        .into()
    }
}

impl MenuEditMods {
    pub fn view<'a>(&'a self, selected_instance: &'a str) -> Element<'a> {
        if let Some(progress) = &self.mod_update_progress {
            return widget::column!(
                widget::text("Updating mods").size(20),
                widget::progress_bar(0.0..=1.0, progress.num),
                widget::text(&progress.message),
            )
            .padding(10)
            .spacing(10)
            .into();
        }

        let mod_installer = self.get_mod_installer_buttons();

        let mod_update_pane = if self.available_updates.is_empty() {
            widget::column!()
        } else {
            widget::column!(
                "Mod Updates Available!",
                widget::column(self.available_updates.iter().enumerate().map(
                    |(i, (_, name, is_enabled))| {
                        widget::checkbox(name, *is_enabled)
                            .on_toggle(move |b| Message::ManageModsUpdateCheckToggle(i, b))
                            .text_size(12)
                            .into()
                    }
                ))
                .spacing(10),
                button_with_icon(icon_manager::update(), "Update")
                    .on_press(Message::ManageModsUpdateMods),
            )
            .padding(10)
            .spacing(10)
            .width(200)
        };

        let side_pane = widget::scrollable(
            widget::column!(
                widget::button(
                    widget::row![icon_manager::back(), "Back"]
                        .spacing(10)
                        .padding(5)
                )
                .on_press(Message::LaunchScreenOpen(None)),
                mod_installer,
                Self::open_mod_folder_button(selected_instance),
                widget::container(mod_update_pane),
            )
            .padding(10)
            .spacing(20),
        );

        let mod_list = self.get_mod_list();

        widget::row!(side_pane, mod_list)
            .padding(10)
            .spacing(10)
            .into()
    }

    fn get_mod_installer_buttons(&self) -> widget::Column<'_, Message, LauncherTheme> {
        let mod_installer = match self.config.mod_type.as_str() {
            "Vanilla" => {
                widget::column![
                    widget::button("Install OptiFine").on_press(Message::InstallOptifineScreenOpen),
                    widget::button("Install Fabric")
                        .on_press(Message::InstallFabric(InstallFabricMessage::ScreenOpen)),
                    widget::button("Install Forge").on_press(Message::InstallForgeStart),
                    widget::button("Install Quilt"),
                    widget::button("Install NeoForge"),
                ]
            }
            "Forge" => {
                widget::column!(
                    widget::button("Install OptiFine"),
                    Self::get_uninstall_panel(
                        &self.config.mod_type,
                        Message::UninstallLoaderForgeStart,
                        true
                    )
                )
            }
            "OptiFine" => {
                widget::column!(
                    widget::button("Install Forge"),
                    Self::get_uninstall_panel(
                        &self.config.mod_type,
                        Message::UninstallLoaderOptiFineStart,
                        false
                    ),
                )
            }
            "Fabric" => Self::get_uninstall_panel(
                &self.config.mod_type,
                Message::UninstallLoaderFabricStart,
                true,
            ),
            _ => {
                widget::column!(widget::text(format!(
                    "Unknown mod type: {}",
                    self.config.mod_type
                )))
            }
        }
        .spacing(5);
        mod_installer
    }

    fn get_uninstall_panel(
        mod_type: &str,
        uninstall_loader_message: Message,
        download_mods: bool,
    ) -> iced::widget::Column<'_, Message, LauncherTheme> {
        widget::column!(
            widget::button(
                widget::row!(
                    icon_manager::delete(),
                    widget::text(format!("Uninstall {mod_type}"))
                )
                .spacing(10)
                .padding(5)
            )
            .on_press(uninstall_loader_message),
            if download_mods {
                widget::column!(
                    button_with_icon(icon_manager::download(), "Download Mods")
                        .on_press(Message::InstallModsOpen),
                    widget::text("Warning: the mod store is\nexperimental and may have bugs")
                        .size(12)
                )
                .spacing(5)
            } else {
                widget::column!()
            },
        )
        .spacing(5)
    }

    fn open_mod_folder_button(selected_instance: &str) -> Element {
        let path = {
            if let Ok(launcher_dir) = file_utils::get_launcher_dir() {
                let path = launcher_dir
                    .join("instances")
                    .join(selected_instance)
                    .join(".minecraft/mods");
                path.exists().then_some(path.to_str().unwrap().to_owned())
            } else {
                None
            }
        };

        button_with_icon(icon_manager::folder(), "Go to Mods Folder")
            .on_press_maybe(path.map(Message::CoreOpenDir))
            .into()
    }

    fn get_mod_list(&self) -> Element {
        if self.sorted_mods_list.is_empty() {
            widget::column!("Download some mods to get started")
        } else {
            widget::column!(
                "Select some mods to perform actions on them",
                widget::row!(
                    button_with_icon(icon_manager::delete(), "Delete")
                        .on_press(Message::ManageModsDeleteSelected),
                    button_with_icon(icon_manager::toggle(), "Toggle On/Off")
                        .on_press(Message::ManageModsToggleSelected),
                    button_with_icon(
                        icon_manager::tick(),
                        if matches!(self.selected_state, SelectedState::All) {
                            "Unselect All"
                        } else {
                            "Select All"
                        }
                    )
                    .on_press(Message::ManageModsSelectAll)
                )
                .spacing(5),
                self.get_mod_list_contents(),
            )
        }
        .spacing(10)
        .into()
    }

    fn get_mod_list_contents(&self) -> Element {
        widget::scrollable(
            widget::column({
                self.sorted_mods_list
                    .iter()
                    .map(|mod_list_entry| match mod_list_entry {
                        ModListEntry::Downloaded { id, config } => widget::row!(
                            if config.manually_installed {
                                widget::row!(widget::checkbox(
                                    format!(
                                        "{}{}",
                                        if config.enabled { "" } else { "(DISABLED) " },
                                        config.name
                                    ),
                                    self.selected_mods.contains(&SelectedMod::Downloaded {
                                        name: config.name.clone(),
                                        id: (*id).clone()
                                    })
                                )
                                .on_toggle(move |t| {
                                    Message::ManageModsToggleCheckbox(
                                        (config.name.clone(), id.clone()),
                                        t,
                                    )
                                }))
                            } else {
                                widget::row!(widget::text(format!(
                                    "- (DEPENDENCY) {}",
                                    config.name
                                )))
                            },
                            widget::horizontal_space(),
                            widget::text(&config.installed_version).width(100).size(12),
                        )
                        .into(),
                        ModListEntry::Local { file_name } => widget::row!(widget::checkbox(
                            file_name.clone(),
                            self.selected_mods.contains(&SelectedMod::Local {
                                file_name: file_name.clone()
                            })
                        )
                        .on_toggle(move |t| {
                            Message::ManageModsToggleCheckboxLocal(file_name.clone(), t)
                        }))
                        .into(),
                    })
            })
            .padding(10)
            .spacing(10),
        )
        .into()
    }
}

impl MenuCreateInstance {
    pub fn view(&self) -> Element {
        match self {
            MenuCreateInstance::Loading {
                progress_number, ..
            } => widget::column![
                widget::text("Loading version list...").size(20),
                widget::progress_bar(0.0..=21.0, *progress_number),
                widget::text(if *progress_number >= 1.0 {
                    format!("Downloading OmniArchive list {progress_number} / 20")
                } else {
                    "Downloading official version list".to_owned()
                })
            ]
            .padding(10)
            .spacing(10)
            .into(),
            MenuCreateInstance::Loaded {
                instance_name,
                selected_version,
                progress_receiver,
                progress_number,
                progress_text,
                download_assets,
                combo_state,
                ..
            } => {
                let progress_bar = if let Some(progress_number) = progress_number {
                    if let Some(progress_text) = progress_text {
                        widget::column![
                            widget::progress_bar(RangeInclusive::new(0.0, 10.0), *progress_number),
                            widget::text(progress_text),
                        ]
                    } else {
                        widget::column![]
                    }
                } else {
                    widget::column![]
                };

                widget::scrollable(
                    widget::column![
                        widget::button(
                            widget::row![icon_manager::back(), "Back"]
                                .spacing(10)
                                .padding(5)
                        ).on_press_maybe((progress_receiver.is_none()).then_some(Message::LaunchScreenOpen(None))),
                            widget::combo_box(combo_state, "Select a version...", selected_version.as_ref(), |version| {
                                Message::CreateInstance(CreateInstanceMessage::VersionSelected(version))
                            }),
                        widget::text_input("Enter instance name...", instance_name)
                            .on_input(|n| Message::CreateInstance(CreateInstanceMessage::NameInput(n))),
                        widget::tooltip(
                            widget::checkbox("Download assets?", *download_assets).on_toggle(|t| Message::CreateInstance(CreateInstanceMessage::ChangeAssetToggle(t))),
                            widget::text("If disabled, creating instance will be MUCH faster, but no sound or music will play in-game").size(12),
                            widget::tooltip::Position::FollowCursor),
                        widget::button(widget::row![icon_manager::create(), "Create Instance"]
                                .spacing(10)
                                .padding(5)
                        ).on_press_maybe((selected_version.is_some() && !instance_name.is_empty() && progress_receiver.is_none()).then(|| Message::CreateInstance(CreateInstanceMessage::Start))),
                        widget::text("To install Fabric/Forge/OptiFine/Quilt, click on Manage Mods after installing the instance").size(12),
                        progress_bar,
                    ]
                    .spacing(10)
                    .padding(10),
                )
                .into()
            }
        }
    }
}

pub fn menu_delete_instance_view(selected_instance: &str) -> Element {
    widget::column![
        widget::text(format!(
            "Are you SURE you want to DELETE the Instance: {}?",
            &selected_instance
        )),
        "All your data, including worlds will be lost.",
        widget::button("Yes, delete my data").on_press(Message::DeleteInstance),
        widget::button("No").on_press(Message::LaunchScreenOpen(None)),
    ]
    .padding(10)
    .spacing(10)
    .into()
}

impl MenuInstallFabric {
    pub fn view(&self, selected_instance: &str) -> Element {
        match self {
            MenuInstallFabric::Loading => {
                widget::column![widget::text("Loading Fabric version list...").size(20)]
            }
            MenuInstallFabric::Loaded {
                fabric_version,
                fabric_versions,
                progress_receiver,
                progress_num,
                progress_message,
            } => {
                if progress_receiver.is_some() {
                    widget::column!(
                        widget::text("Installing Fabric...").size(20),
                        widget::progress_bar(0.0..=1.0, *progress_num),
                        widget::text(progress_message),
                    )
                } else {
                    widget::column![
                        button_with_icon(icon_manager::back(), "Back")
                            .on_press(Message::LaunchScreenOpen(None)),
                        widget::text(format!(
                            "Select Fabric Version for instance {}",
                            &selected_instance
                        )),
                        widget::pick_list(
                            fabric_versions.as_slice(),
                            fabric_version.as_ref(),
                            |n| Message::InstallFabric(InstallFabricMessage::VersionSelected(n))
                        ),
                        widget::button("Install Fabric").on_press_maybe(
                            fabric_version.is_some().then(|| Message::InstallFabric(
                                InstallFabricMessage::ButtonClicked
                            ))
                        ),
                    ]
                }
            }
            MenuInstallFabric::Unsupported => {
                widget::column!(
                    button_with_icon(icon_manager::back(), "Back")
                        .on_press(Message::LaunchScreenOpen(None)),
                    "Fabric is unsupported for this Minecraft version."
                )
            }
        }
        .padding(10)
        .spacing(10)
        .into()
    }
}

impl MenuInstallForge {
    pub fn view(&self) -> Element {
        let main_block = widget::column!(
            widget::text("Installing forge...").size(20),
            iced::widget::progress_bar(0.0..=4.0, self.forge_progress_num),
            widget::text(&self.forge_message)
        )
        .spacing(10);

        if self.is_java_getting_installed {
            if let Some(message) = &self.java_message {
                widget::column!(
                    main_block,
                    widget::progress_bar(0.0..=1.0, self.java_progress_num),
                    widget::text(message)
                )
            } else {
                widget::column!(
                    main_block,
                    iced::widget::progress_bar(0.0..=1.0, self.java_progress_num),
                )
            }
        } else {
            main_block
        }
        .padding(20)
        .spacing(10)
        .into()
    }
}

impl MenuLauncherUpdate {
    pub fn view(&self) -> Element {
        if let Some(message) = &self.progress_message {
            widget::column!(
                "Updating QuantumLauncher...",
                widget::progress_bar(0.0..=4.0, self.progress),
                widget::text(message)
            )
        } else {
            widget::column!(
                "A new launcher update has been found! Do you want to download it?",
                widget::row!(
                    button_with_icon(icon_manager::download(), "Download")
                        .on_press(Message::UpdateDownloadStart),
                    button_with_icon(icon_manager::back(), "Back")
                        .on_press(Message::LaunchScreenOpen(None))
                )
                .spacing(5),
            )
        }
        .padding(10)
        .spacing(10)
        .into()
    }
}

impl MenuInstallJava {
    pub fn view(&self) -> Element {
        widget::column!(
            widget::text("Downloading Java").size(20),
            widget::progress_bar(0.0..=1.0, self.num),
            widget::text(&self.message)
        )
        .padding(10)
        .spacing(10)
        .into()
    }
}

impl MenuLauncherSettings {
    pub fn view(config: Option<&LauncherConfig>) -> Element {
        let themes = ["Dark".to_owned(), "Light".to_owned()];
        let styles = ["Brown".to_owned(), "Purple".to_owned()];

        let config_view = if let Some(config) = config {
            widget::column!(
                widget::container(
                    widget::column!(
                        "Select theme:",
                        widget::pick_list(
                            themes,
                            config.theme.clone(),
                            Message::LauncherSettingsThemePicked
                        ),
                    )
                    .padding(10)
                    .spacing(10)
                ),
                widget::container(
                    widget::column!(
                        "Select style:",
                        widget::pick_list(
                            styles,
                            config.style.clone(),
                            Message::LauncherSettingsStylePicked
                        )
                    )
                    .padding(10)
                    .spacing(10)
                ),
            )
        } else {
            widget::column!()
        }
        .spacing(10);

        widget::scrollable(
            widget::column!(
                button_with_icon(icon_manager::back(), "Back")
                    .on_press(Message::LaunchScreenOpen(None)),
                config_view,
                widget::container(
                    widget::column!(
                        button_with_icon(icon_manager::page(), "Open Website").on_press(
                            Message::CoreOpenDir(
                                "https://mrmayman.github.io/quantumlauncher".to_owned()
                            )
                        ),
                        button_with_icon(icon_manager::github(), "Open Github Repo").on_press(
                            Message::CoreOpenDir(
                                "https://github.com/Mrmayman/quantum-launcher".to_owned()
                            )
                        ),
                        button_with_icon(icon_manager::chat(), "Join our Discord").on_press(
                            Message::CoreOpenDir("https://discord.gg/bWqRaSXar5".to_owned())
                        ),
                    )
                    .padding(10)
                    .spacing(10)
                ),
            )
            .padding(10)
            .spacing(10),
        )
        .into()
    }
}
