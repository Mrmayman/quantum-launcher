use std::collections::HashSet;

use iced::widget;
use ql_core::{InstanceSelection, Progress, SelectedMod};

use crate::{
    config::LauncherConfig,
    icon_manager,
    launcher_state::{
        CreateInstanceMessage, EditInstanceMessage, EditPresetsMessage, InstallFabricMessage,
        InstallOptifineMessage, ManageModsMessage, MenuCreateInstance, MenuEditInstance,
        MenuEditPresets, MenuEditPresetsInner, MenuInstallFabric, MenuInstallForge,
        MenuInstallOptifine, MenuLauncherSettings, MenuLauncherUpdate, Message, ModListEntry,
        ProgressBar, SelectedState,
    },
    stylesheet::{color::Color, styles::LauncherTheme},
};

pub mod changelog;
mod dynamic_box;
pub mod launch;
pub mod mods_manage;
pub mod mods_store;
pub mod server_manager;

pub use dynamic_box::dynamic_box;

pub const DISCORD: &str = "https://discord.gg/bWqRaSXar5";

pub type Element<'a> = iced::Element<'a, Message, LauncherTheme, iced::Renderer>;

pub fn button_with_icon<'element>(
    icon: Element<'element>,
    text: &'element str,
    size: u16,
) -> iced::widget::Button<'element, Message, LauncherTheme, iced::Renderer> {
    widget::button(
        widget::row![icon, widget::text(text).size(size)]
            .spacing(10)
            .padding(5),
    )
}

impl MenuEditInstance {
    pub fn view<'a>(&'a self, selected_instance: &InstanceSelection) -> Element<'a> {
        // 2 ^ 8 = 256 MB
        const MEM_256_MB_IN_TWOS_EXPONENT: f32 = 8.0;
        // 2 ^ 13 = 8192 MB
        const MEM_8192_MB_IN_TWOS_EXPONENT: f32 = 13.0;

        widget::scrollable(
            widget::column![
                widget::container(widget::column!(
                    widget::row!(
                        widget::button("Rename").on_press(Message::EditInstance(EditInstanceMessage::RenameApply)),
                        widget::text_input("Rename Instance", &self.instance_name).on_input(|n| Message::EditInstance(EditInstanceMessage::RenameEdit(n))),
                    ).spacing(5),
                    widget::text(
                        match selected_instance {
                            InstanceSelection::Instance(n) => format!("Editing {} instance: {n}", self.config.mod_type),
                            InstanceSelection::Server(n) => format!("Editing {} server: {n}", self.config.mod_type),
                        }
                    ),
                ).padding(10).spacing(10)).style(|n: &LauncherTheme| n.style_container_sharp_box(0.0, Color::Black)),
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
                ).style(|n: &LauncherTheme| n.style_container_sharp_box(0.0, Color::Dark)),
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
                ).style(|n: &LauncherTheme| n.style_container_sharp_box(0.0, Color::Black)),
                widget::container(
                    widget::column![
                        widget::checkbox("Enable logging", self.config.enable_logger.unwrap_or(true))
                            .on_toggle(|t| Message::EditInstance(EditInstanceMessage::LoggingToggle(t))),
                        widget::text("Enabled by default, disable if you want to see some advanced crash messages in the terminal.").size(12)
                    ]
                    .padding(10)
                    .spacing(10)
                ).style(|n: &LauncherTheme| n.style_container_sharp_box(0.0, Color::Dark)),
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
                            button_with_icon(icon_manager::create(), "Add", 16)
                                .on_press(Message::EditInstance(EditInstanceMessage::JavaArgsAdd))
                        ),
                        widget::horizontal_space(),
                    ).padding(10).spacing(10)
                ).style(|n: &LauncherTheme| n.style_container_sharp_box(0.0, Color::Black)),
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
                            button_with_icon(icon_manager::create(), "Add", 16)
                                .on_press(Message::EditInstance(EditInstanceMessage::GameArgsAdd))
                        )
                    ).padding(10).spacing(10)
                ).style(|n: &LauncherTheme| n.style_container_sharp_box(0.0, Color::Dark)),
                widget::container(widget::row!(
                    button_with_icon(icon_manager::delete(), "Delete Instance", 16)
                        .on_press(
                            Message::DeleteInstanceMenu
                        ),
                    widget::horizontal_space(),
                )).padding(10).style(|n: &LauncherTheme| n.style_container_sharp_box(0.0, Color::Black)),
            ]
        ).style(LauncherTheme::style_scrollable_flat).into()
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
                        .align_y(iced::Alignment::Center)
                        .padding(5)
                )
                .on_press(msg_delete(i)),
                widget::button(
                    widget::row![icon_manager::arrow_up_with_size(ITEM_SIZE)]
                        .align_y(iced::Alignment::Center)
                        .padding(5)
                )
                .on_press(msg_up(i)),
                widget::button(
                    widget::row![icon_manager::arrow_down_with_size(ITEM_SIZE)]
                        .align_y(iced::Alignment::Center)
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
        if let Some(optifine) = &self.optifine_install_progress {
            widget::column!(
                optifine.view(),
                if self.is_java_being_installed {
                    if let Some(java) = &self.java_install_progress {
                        widget::column!(widget::container(java.view()))
                    } else {
                        widget::column!()
                    }
                } else {
                    widget::column!()
                },
            )
        } else {
            Self::install_optifine_screen()
        }
        .padding(10)
        .spacing(10)
        .into()
    }

    pub fn install_optifine_screen<'a>(
    ) -> iced::widget::Column<'a, Message, LauncherTheme, iced::Renderer> {
        widget::column!(
            button_with_icon(icon_manager::back(), "Back", 16)
                .on_press(Message::ManageMods(ManageModsMessage::ScreenOpen)),
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
                    widget::button("Select File").on_press(Message::InstallOptifine(
                        InstallOptifineMessage::SelectInstallerStart
                    ))
                )
                .padding(10)
                .spacing(10)
            )
        )
    }
}

impl MenuCreateInstance {
    pub fn view(&self) -> Element {
        match self {
            MenuCreateInstance::Loading {
                progress_number, ..
            } => widget::column![
                widget::text("Loading version list...").size(20),
                widget::progress_bar(0.0..=24.0, *progress_number),
                widget::text(if *progress_number >= 1.0 {
                    format!("Downloading Omniarchive list {progress_number} / 24")
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
                progress,
                download_assets,
                combo_state,
                ..
            } => {

                widget::scrollable(
                    widget::column![
                        widget::button(
                            widget::row![icon_manager::back(), "Back"]
                                .spacing(10)
                                .padding(5)
                        ).on_press_maybe((progress.is_none()).then_some(Message::LaunchScreenOpen {message: None, clear_selection: false})),
                            widget::combo_box(combo_state, "Select a version...", selected_version.as_ref(), |version| {
                                Message::CreateInstance(CreateInstanceMessage::VersionSelected(version))
                            }),
                        widget::text_input("Enter instance name...", instance_name)
                            .on_input(|n| Message::CreateInstance(CreateInstanceMessage::NameInput(n))),
                        widget::tooltip(
                            widget::checkbox("Download assets?", *download_assets).on_toggle(|t| Message::CreateInstance(CreateInstanceMessage::ChangeAssetToggle(t))),
                            widget::text("If disabled, creating instance will be MUCH faster, but no sound or music will play in-game").size(12),
                            widget::tooltip::Position::FollowCursor).style(|n: &LauncherTheme| n.style_container_sharp_box(0.0, Color::Black)),
                        widget::button(widget::row![icon_manager::create(), "Create Instance"]
                                .spacing(10)
                                .padding(5)
                        ).on_press_maybe((selected_version.is_some() && !instance_name.is_empty() && progress.is_none()).then(|| Message::CreateInstance(CreateInstanceMessage::Start))),
                        widget::text("To install Fabric/Forge/OptiFine/Quilt, click on Mods after installing the instance").size(12),
                        if let Some(progress) = progress {
                            progress.view()
                        } else {
                            widget::column![].into()
                        },
                    ]
                    .spacing(10)
                    .padding(10),
                )
                .into()
            }
        }
    }
}

impl MenuInstallFabric {
    pub fn view(&self, selected_instance: &InstanceSelection) -> Element {
        match self {
            MenuInstallFabric::Loading(is_quilt) => {
                widget::column![widget::text(if *is_quilt {
                    "Loading Quilt version list..."
                } else {
                    "Loading Fabric version list..."
                })
                .size(20)]
            }
            MenuInstallFabric::Loaded {
                is_quilt,
                fabric_version,
                fabric_versions,
                progress,
            } => {
                if let Some(progress) = progress {
                    widget::column!(
                        widget::text(if *is_quilt {
                            "Installing Quilt..."
                        } else {
                            "Installing Fabric..."
                        })
                        .size(20),
                        progress.view(),
                    )
                } else {
                    widget::column![
                        button_with_icon(icon_manager::back(), "Back", 16)
                            .on_press(back_to_launch_screen(selected_instance, None)),
                        widget::text!(
                            "Select {} Version for instance {}",
                            if *is_quilt { "Quilt" } else { "Fabric" },
                            selected_instance.get_name()
                        ),
                        widget::pick_list(
                            fabric_versions.as_slice(),
                            fabric_version.as_ref(),
                            |n| Message::InstallFabric(InstallFabricMessage::VersionSelected(n))
                        ),
                        widget::button(if *is_quilt {
                            "Install Quilt"
                        } else {
                            "Install Fabric"
                        })
                        .on_press_maybe(
                            fabric_version.is_some().then(|| Message::InstallFabric(
                                InstallFabricMessage::ButtonClicked
                            ))
                        ),
                    ]
                }
            }
            MenuInstallFabric::Unsupported(is_quilt) => {
                widget::column!(
                    button_with_icon(icon_manager::back(), "Back", 16)
                        .on_press(back_to_launch_screen(selected_instance, None)),
                    if *is_quilt {
                        "Quilt is unsupported for this Minecraft version."
                    } else {
                        "Fabric is unsupported for this Minecraft version."
                    }
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
            self.forge_progress.view()
        )
        .spacing(10);

        if self.is_java_getting_installed {
            widget::column!(main_block, self.java_progress.view())
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
        if let Some(progress) = &self.progress {
            widget::column!("Updating QuantumLauncher...", progress.view())
        } else {
            widget::column!(
                "A new launcher update has been found! Do you want to download it?",
                widget::row!(
                    button_with_icon(icon_manager::download(), "Download", 16)
                        .on_press(Message::UpdateDownloadStart),
                    button_with_icon(icon_manager::back(), "Back", 16).on_press(
                        Message::LaunchScreenOpen {
                            message: None,
                            clear_selection: false
                        }
                    )
                )
                .spacing(5),
                "Note: If you downloaded this from a package manager or store, update it from there, not here."
            )
        }
        .padding(10)
        .spacing(10)
        .into()
    }
}

impl MenuLauncherSettings {
    pub fn view(config: Option<&LauncherConfig>) -> Element {
        let themes = ["Dark".to_owned(), "Light".to_owned()];
        let styles = [
            "Brown".to_owned(),
            "Purple".to_owned(),
            "Sky Blue".to_owned(),
        ];

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
                button_with_icon(icon_manager::back(), "Back", 16).on_press(
                    Message::LaunchScreenOpen {
                        message: None,
                        clear_selection: false
                    }
                ),
                config_view,
                widget::container(
                    widget::column!(
                        button_with_icon(icon_manager::page(), "View Changelog", 16)
                            .on_press(Message::CoreOpenChangeLog),
                        button_with_icon(icon_manager::page(), "Open Website", 16).on_press(
                            Message::CoreOpenDir(
                                "https://mrmayman.github.io/quantumlauncher".to_owned()
                            )
                        ),
                        button_with_icon(icon_manager::github(), "Open Github Repo", 16).on_press(
                            Message::CoreOpenDir(
                                "https://github.com/Mrmayman/quantum-launcher".to_owned()
                            )
                        ),
                        button_with_icon(icon_manager::chat(), "Join our Discord", 16).on_press(
                            Message::CoreOpenDir(DISCORD.to_owned())
                        ),
                        "QuantumLauncher is free and open source software under the GNU GPL3 license.",
                        "If you like it, consider sharing it with your friends.",
                        "Every new user motivates me to keep working on this :)"
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

fn back_to_launch_screen(
    selected_instance: &InstanceSelection,
    message: Option<String>,
) -> Message {
    match selected_instance {
        InstanceSelection::Server(selected_server) => Message::ServerManageOpen {
            selected_server: Some(selected_server.clone()),
            message,
        },
        InstanceSelection::Instance(_) => Message::LaunchScreenOpen {
            message: None,
            clear_selection: false,
        },
    }
}

impl MenuEditPresets {
    pub fn view(&self) -> Element {
        if let Some(progress) = &self.progress {
            return widget::column!(widget::text("Installing mods").size(20), progress.view())
                .padding(10)
                .spacing(10)
                .into();
        }

        if let MenuEditPresetsInner::Build {
            is_building: true, ..
        } = &self.inner
        {
            return widget::column!(widget::text("Building Preset").size(20),)
                .padding(10)
                .spacing(10)
                .into();
        }

        widget::scrollable(
            widget::column!(
                widget::row!(
                    button_with_icon(icon_manager::back(), "Back", 16)
                        .on_press(Message::ManageMods(ManageModsMessage::ScreenOpen)),
                    widget::tooltip(button_with_icon(icon_manager::folder(), "Import Preset", 16)
                        .on_press(Message::EditPresets(EditPresetsMessage::Load)), widget::column!(
                            widget::text("Note: Sideloaded mods in imported presets (that anyone sends to you) could be untrusted (might have viruses)").size(12),
                                widget::text("To get rid of them after installing, remove all the mods in the list ending in \".jar\"").size(12)
                        ), widget::tooltip::Position::FollowCursor).style(|n: &LauncherTheme| n.style_container_sharp_box(0.0, Color::Black)),
                )
                .spacing(5),
                "Presets are small bundles of mods and their configuration that you can share with anyone. You can import presets, create them or download recommended mods (if you haven't installed any yet.",
                self.get_create_preset_page()
            )
            .padding(10)
            .spacing(10),
        )
        .into()
    }

    fn get_create_preset_page(&self) -> Element {
        match &self.inner {
            MenuEditPresetsInner::Build {
                mods,
                selected_state,
                selected_mods,
                ..
            } => widget::column!(
                widget::text("Create Preset").size(20),
                "Select Mods to keep",
                widget::button(if let SelectedState::All = selected_state {
                    "Unselect All"
                } else {
                    "Select All"
                })
                .on_press(Message::EditPresets(EditPresetsMessage::SelectAll)),
                widget::container(Self::get_mods_list(selected_mods, mods).padding(10)),
                button_with_icon(icon_manager::save(), "Build Preset", 16)
                    .on_press(Message::EditPresets(EditPresetsMessage::BuildYourOwn)),
            )
            .spacing(10)
            .into(),
            MenuEditPresetsInner::Recommended {
                mods,
                progress,
                error,
            } => {
                if let Some(error) = error {
                    widget::column!(
                        widget::text!("Error loading presets: {error}"),
                        widget::button("Copy Error").on_press(Message::CoreCopyText(error.clone()))
                    )
                    .spacing(10)
                    .into()
                } else if let Some(mods) = mods {
                    widget::column!(
                        button_with_icon(icon_manager::download(), "Download Recommended Mods", 16)
                            .on_press(Message::EditPresets(
                                EditPresetsMessage::RecommendedDownload
                            )),
                        widget::column(mods.iter().enumerate().map(|(i, (e, n))| {
                            let elem: Element = if n.enabled_by_default {
                                widget::text!("- {}", n.name).into()
                            } else {
                                widget::checkbox(n.name, *e)
                                    .on_toggle(move |n| {
                                        Message::EditPresets(EditPresetsMessage::RecommendedToggle(
                                            i, n,
                                        ))
                                    })
                                    .into()
                            };
                            widget::column!(elem, widget::text(n.description).size(12))
                                .spacing(5)
                                .into()
                        }))
                        .spacing(10)
                    )
                    .spacing(10)
                    .into()
                } else {
                    progress.view()
                }
            }
        }
    }

    fn get_mods_list<'a>(
        selected_mods: &'a HashSet<SelectedMod>,
        mods: &'a [ModListEntry],
    ) -> widget::Column<'a, Message, LauncherTheme, iced::Renderer> {
        widget::column(mods.iter().map(|entry| {
            if entry.is_manually_installed() {
                widget::checkbox(entry.name(), selected_mods.contains(&entry.id()))
                    .on_toggle(move |t| match entry {
                        ModListEntry::Downloaded { id, config } => {
                            Message::EditPresets(EditPresetsMessage::ToggleCheckbox(
                                (config.name.clone(), id.clone()),
                                t,
                            ))
                        }
                        ModListEntry::Local { file_name } => Message::EditPresets(
                            EditPresetsMessage::ToggleCheckboxLocal(file_name.clone(), t),
                        ),
                    })
                    .into()
            } else {
                widget::text!(" - (DEPENDENCY) {}", entry.name()).into()
            }
        }))
        .spacing(5)
    }
}

impl<T: Progress> ProgressBar<T> {
    pub fn view(&self) -> Element {
        let total = T::total();
        if let Some(message) = &self.message {
            widget::column!(
                widget::progress_bar(0.0..=total, self.num),
                widget::text(message)
            )
        } else {
            widget::column!(widget::progress_bar(0.0..=total, self.num),)
        }
        .spacing(10)
        .into()
    }
}

pub fn view_account_login<'a>(url: &'a str, code: &'a str) -> Element<'a> {
    widget::row!(
        widget::horizontal_space(),
        widget::column!(
            button_with_icon(icon_manager::back(), "Back", 16).on_press(
                Message::LaunchScreenOpen {
                    message: None,
                    clear_selection: false
                }
            ),
            widget::vertical_space(),
            widget::text("Login to Microsoft").size(20),
            "Open this link and enter the code:",
            widget::text(url),
            widget::button("Open").on_press(Message::CoreOpenDir(url.to_owned())),
            widget::text(code),
            widget::button("Copy").on_press(Message::CoreCopyText(code.to_owned())),
            widget::vertical_space(),
        )
        .spacing(5)
        .align_x(iced::Alignment::Center),
        widget::horizontal_space()
    )
    .into()
}
