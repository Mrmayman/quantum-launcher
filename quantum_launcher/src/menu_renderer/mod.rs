use std::{collections::HashMap, ops::RangeInclusive};

use iced::widget;
use ql_instances::{file_utils, LAUNCHER_VERSION_NAME};

use crate::{
    config::LauncherConfig,
    icon_manager,
    launcher_state::{
        GameProcess, InstanceLog, Launcher, MenuCreateInstance, MenuDeleteInstance,
        MenuEditInstance, MenuEditMods, MenuInstallFabric, MenuInstallForge, MenuInstallJava,
        MenuInstallOptifine, MenuLaunch, MenuLauncherSettings, MenuLauncherUpdate, Message,
        SelectedMod, SelectedState,
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
    config: Option<&LauncherConfig>,
    pick_list: Element<'a>,
    selected_instance: Option<&String>,
    processes: &HashMap<String, GameProcess>,
    footer_text: Element<'a>,
) -> Element<'a> {
    widget::column![
        widget::column![
            "Username:",
            widget::text_input("Enter username...", &config.as_ref().unwrap().username)
                .on_input(Message::LaunchUsernameSet)
                .width(200),
        ]
        .spacing(5),
        pick_list,
        widget::column!(
            widget::row![
                button_with_icon(icon_manager::folder(), "Files")
                    .on_press_maybe((selected_instance.is_some()).then(|| {
                        let launcher_dir = file_utils::get_launcher_dir().unwrap();
                        Message::OpenDir(
                            launcher_dir
                                .join("instances")
                                .join(selected_instance.as_ref().unwrap())
                                .join(".minecraft")
                                .to_str()
                                .unwrap()
                                .to_owned(),
                        )
                    }))
                    .width(97),
                if let Some(selected_instance) = selected_instance {
                    if processes.contains_key(selected_instance) {
                        button_with_icon(icon_manager::play(), "Kill").on_press(Message::LaunchKill)
                    } else {
                        button_with_icon(icon_manager::play(), "Play")
                            .on_press(Message::LaunchStart)
                    }
                } else {
                    button_with_icon(icon_manager::play(), "Play")
                }
                .width(98),
            ]
            .spacing(5),
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
                    .on_press(Message::CreateInstanceScreenOpen)
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
                    .on_press_maybe((selected_instance.is_some()).then_some(Message::EditInstance))
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
    pub fn view<'element>(&self, selected_instance: &str) -> Element<'element> {
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
                            "Enter Java override",
                            self.config
                                .java_override
                                .as_deref()
                                .unwrap_or_default()
                        )
                        .on_input(Message::EditInstanceJavaOverride)
                    ]
                    .padding(10)
                    .spacing(10)
                ),
                widget::container(
                    widget::column![
                        "Allocated memory",
                        "For normal Minecraft, allocate 2 - 3 GB",
                        "For old versions, allocate 512 MB - 1 GB",
                        "For heavy modpacks or very high render distances, allocate 4 - 8 GB",
                        widget::slider(MEM_256_MB_IN_TWOS_EXPONENT..=MEM_8192_MB_IN_TWOS_EXPONENT, self.slider_value, Message::EditInstanceMemoryChanged).step(0.1),
                        widget::text(&self.slider_text),
                    ]
                    .padding(10)
                    .spacing(5),
                ),
                widget::container(
                    widget::column![
                        widget::checkbox("Enable logging", self.config.enable_logger.unwrap_or(true))
                            .on_toggle(Message::EditInstanceLoggingToggle),
                        widget::text("Enabled by default, disable if you want to see some advanced crash messages in the terminal.").size(12)
                    ]
                    .padding(10)
                    .spacing(10)
                ),
            ]
            .padding(10)
            .spacing(20)
        ).into()
    }
}

const OPTIFINE_DOWNLOADS: &str = "https://optifine.net/downloads";

impl MenuInstallOptifine {
    pub fn view(&self) -> Element {
        widget::column!(
            button_with_icon(icon_manager::back(), "Back").on_press(Message::ManageModsScreenOpen),
            widget::container(
                widget::column!(
                    widget::text("Install OptiFine").size(20),
                    "Step 1: Open the OptiFine download page and download the installer.",
                    widget::button("Open download page")
                        .on_press(Message::OpenDir(OPTIFINE_DOWNLOADS.to_owned()))
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
        .padding(10)
        .spacing(10)
        .into()
    }
}

impl MenuEditMods {
    pub fn view(&self, selected_instance: &str) -> Element {
        let mod_installer = if self.config.mod_type == "Vanilla" {
            widget::column![
                widget::button("Install OptiFine").on_press(Message::InstallOptifineScreenOpen),
                widget::button("Install Fabric").on_press(Message::InstallFabricScreenOpen),
                widget::button("Install Forge").on_press(Message::InstallForgeStart),
                widget::button("Install Quilt"),
                widget::button("Install NeoForge"),
            ]
        } else {
            widget::column![
                widget::button(
                    widget::row![
                        icon_manager::delete(),
                        widget::text(format!("Uninstall {}", self.config.mod_type))
                    ]
                    .spacing(10)
                    .padding(5)
                )
                .on_press_maybe(
                    (self.config.mod_type == "Fabric" || self.config.mod_type == "Forge")
                        .then_some(Message::UninstallLoaderStart)
                ),
                button_with_icon(icon_manager::download(), "Download Mods")
                    .on_press(Message::InstallModsOpen)
            ]
        }
        .spacing(5);

        let side_pane = widget::column![
            widget::button(
                widget::row![icon_manager::back(), "Back"]
                    .spacing(10)
                    .padding(5)
            )
            .on_press(Message::LaunchScreenOpen(None)),
            mod_installer,
            self.open_mod_folder_button(selected_instance),
        ]
        .padding(10)
        .spacing(20);

        let mod_list = self.get_mod_list();

        widget::row!(side_pane, mod_list)
            .padding(10)
            .spacing(10)
            .into()
    }

    fn open_mod_folder_button(&self, selected_instance: &str) -> Element {
        button_with_icon(icon_manager::folder(), "Go to Mods Folder")
            .on_press({
                let launcher_dir = file_utils::get_launcher_dir().unwrap();
                Message::OpenDir(
                    launcher_dir
                        .join("instances")
                        .join(selected_instance)
                        .join(".minecraft/mods")
                        .to_str()
                        .unwrap()
                        .to_owned(),
                )
            })
            .into()
    }

    fn get_mod_list(&self) -> Element {
        if self.sorted_dependencies.is_empty() {
            widget::column!("Download some mods to get started")
        } else {
            widget::column!(
                "Select some mods to perform actions on them",
                widget::row!(
                    button_with_icon(icon_manager::delete(), "Delete")
                        .on_press(Message::ManageModsDeleteSelected),
                    button_with_icon(icon_manager::play(), "Toggle On/Off")
                        .on_press(Message::ManageModsToggleSelected),
                    button_with_icon(
                        icon_manager::play(),
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
                self.sorted_dependencies.iter().map(|(id, config)| {
                    // let config_name = config.name.clone();
                    widget::row!(
                        if config.manually_installed {
                            widget::row!(widget::checkbox(
                                format!(
                                    "{}{}",
                                    if config.enabled { "" } else { "(DISABLED) " },
                                    config.name
                                ),
                                self.selected_mods.contains(&SelectedMod {
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
                            widget::row!(widget::text(format!("- (DEPENDENCY) {}", config.name)))
                        },
                        widget::horizontal_space(),
                        widget::text(&config.installed_version).width(100).size(12),
                    )
                    .into()
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
        let progress_bar = if let Some(progress_number) = self.progress_number {
            if let Some(progress_text) = &self.progress_text {
                widget::column![
                    widget::progress_bar(RangeInclusive::new(0.0, 10.0), progress_number),
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
                ).on_press_maybe((self.progress_receiver.is_none()).then_some(Message::LaunchScreenOpen(None))),
                    widget::combo_box(&self.combo_state, "Select a version...", self.selected_version.as_ref(), |version| {
                        Message::CreateInstanceVersionSelected(version)
                    }),
                widget::text_input("Enter instance name...", &self.instance_name)
                    .on_input(Message::CreateInstanceNameInput),
                widget::tooltip(
                    widget::checkbox("Download assets?", self.download_assets).on_toggle(Message::CreateInstanceChangeAssetToggle),
                    widget::text("If disabled, creating instance will be MUCH faster, but no sound or music will play in-game").size(12),
                    widget::tooltip::Position::FollowCursor),
                widget::button(widget::row![icon_manager::create(), "Create Instance"]
                        .spacing(10)
                        .padding(5)
                ).on_press_maybe((self.selected_version.is_some() && !self.instance_name.is_empty() && self.progress_receiver.is_none()).then(|| Message::CreateInstanceStart)),
                widget::text("To install Fabric/Forge/OptiFine/Quilt, click on Manage Mods after installing the instance").size(12),
                progress_bar,
            ]
            .spacing(10)
            .padding(10),
        )
        .into()
    }
}

impl MenuDeleteInstance {
    pub fn view(&self, selected_instance: &str) -> Element {
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
}

impl MenuInstallFabric {
    pub fn view(&self, selected_instance: &str) -> Element {
        if self.progress_receiver.is_some() {
            widget::column!(
                "Installing Fabric...",
                widget::progress_bar(0.0..=1.0, self.progress_num)
            )
            .padding(10)
            .spacing(20)
            .into()
        } else {
            widget::column![
                widget::button(
                    widget::row![icon_manager::back(), "Back"]
                        .spacing(10)
                        .padding(5)
                )
                .on_press(Message::LaunchScreenOpen(None)),
                widget::text(format!(
                    "Select Fabric Version for instance {}",
                    &selected_instance
                )),
                widget::pick_list(
                    self.fabric_versions.as_slice(),
                    self.fabric_version.as_ref(),
                    Message::InstallFabricVersionSelected
                ),
                widget::button("Install Fabric").on_press_maybe(
                    self.fabric_version
                        .is_some()
                        .then(|| Message::InstallFabricClicked)
                ),
            ]
            .padding(10)
            .spacing(20)
            .into()
        }
    }
}

impl MenuInstallForge {
    pub fn view(&self) -> Element {
        let progress_bar = widget::column!(
            iced::widget::progress_bar(0.0..=4.0, self.forge_progress_num),
            widget::text(&self.forge_message)
        );

        if self.is_java_getting_installed {
            if let Some(message) = &self.java_message {
                widget::column!(
                    "Installing forge...",
                    progress_bar,
                    widget::progress_bar(0.0..=1.0, self.java_progress_num),
                    widget::text(message)
                )
            } else {
                widget::column!(
                    "Installing forge...",
                    progress_bar,
                    iced::widget::progress_bar(0.0..=1.0, self.java_progress_num),
                )
            }
        } else {
            widget::column!("Installing forge...", progress_bar)
        }
        .padding(20)
        .spacing(20)
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
                config_view
            )
            .padding(10)
            .spacing(10),
        )
        .into()
    }
}
