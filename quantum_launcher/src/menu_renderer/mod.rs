use iced::{widget, Length};
use ql_core::{InstanceSelection, Progress};

use crate::{
    config::LauncherConfig,
    icon_manager,
    launcher_state::{
        CreateInstanceMessage, EditInstanceMessage, InstallFabricMessage, InstallModsMessage,
        InstallOptifineMessage, LauncherSettingsMessage, ManageJarModsMessage, ManageModsMessage,
        MenuCreateInstance, MenuCurseforgeManualDownload, MenuEditInstance, MenuEditJarMods,
        MenuInstallFabric, MenuInstallForge, MenuInstallOptifine, MenuLauncherSettings,
        MenuLauncherUpdate, Message, ProgressBar, SelectedState,
    },
    stylesheet::{color::Color, styles::LauncherTheme},
};

pub mod changelog;
mod launch;
mod log;
mod mods_manage;
mod mods_store;
mod presets;
mod server_manager;

pub const DISCORD: &str = "https://discord.gg/bWqRaSXar5";

pub type Element<'a> = iced::Element<'a, Message, LauncherTheme, iced::Renderer>;

fn center_x<'a>(e: impl Into<Element<'a>>) -> Element<'a> {
    widget::row![
        widget::horizontal_space(),
        e.into(),
        widget::horizontal_space(),
    ]
    .into()
}

pub fn tooltip<'a>(e: impl Into<Element<'a>>, tooltip: impl Into<Element<'a>>) -> Element<'a> {
    widget::tooltip(e, tooltip, widget::tooltip::Position::Bottom)
        .style(|n: &LauncherTheme| n.style_container_sharp_box(0.0, Color::ExtraDark))
        .into()
}

pub fn button_with_icon<'element>(
    icon: Element<'element>,
    text: &'element str,
    size: u16,
) -> iced::widget::Button<'element, Message, LauncherTheme, iced::Renderer> {
    widget::button(
        widget::row![icon, widget::text(text).size(size)]
            .align_y(iced::alignment::Vertical::Center)
            .spacing(10)
            .padding(3),
    )
}

pub fn shortcut_ctrl<'a>(key: &str) -> Element<'a> {
    #[cfg(target_os = "macos")]
    return widget::text!("Command + {key}").size(12).into();

    #[cfg(not(target_os = "macos"))]
    return widget::text!("Control + {key}").size(12).into();
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
                ).padding(10).spacing(10)).style(|n: &LauncherTheme| n.style_container_sharp_box(0.0, Color::ExtraDark)),
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
                ).style(|n: &LauncherTheme| n.style_container_sharp_box(0.0, Color::ExtraDark)),
                widget::container(
                    widget::column![
                        widget::column![
                            widget::checkbox("Enable logging", self.config.enable_logger.unwrap_or(true))
                                .on_toggle(|t| Message::EditInstance(EditInstanceMessage::LoggingToggle(t))),
                            widget::text("- Recommended to enable this (default) for most cases").size(12),
                            widget::text("- Disable if logs aren't visible properly for some reason").size(12),
                            widget::text("- Once disabled, logs will be printed in launcher STDOUT. Run the launcher executable from the terminal/command prompt to see it").size(12),
                            widget::horizontal_space(),
                        ].spacing(5)
                    ].push_maybe((!selected_instance.is_server()).then_some(widget::column![
                        widget::checkbox("Close launcher after game opens", self.config.close_on_start.unwrap_or(false))
                            .on_toggle(|t| Message::EditInstance(EditInstanceMessage::CloseLauncherToggle(t))),
                        widget::text("Disabled by default, enable if you want the launcher to close when the game opens.").size(12),
                        widget::text("It's recommended you leave this off because:").size(12),
                        widget::text("- This prevents you from seeing logs or killing the process.").size(12),
                        widget::text("- Besides, closing the launcher won't make your game run faster (it's already super lightweight).").size(12),
                    ].spacing(5)))
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
                ).style(|n: &LauncherTheme| n.style_container_sharp_box(0.0, Color::ExtraDark)),
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
                        ),
                        widget::horizontal_space()
                    ).padding(10).spacing(10)
                ).style(|n: &LauncherTheme| n.style_container_sharp_box(0.0, Color::Dark)),
                widget::container(widget::row!(
                    button_with_icon(icon_manager::delete(), "Delete Instance", 16)
                        .on_press(
                            Message::DeleteInstanceMenu
                        ),
                    widget::horizontal_space(),
                )).padding(10).style(|n: &LauncherTheme| n.style_container_sharp_box(0.0, Color::ExtraDark)),
            ]
        ).style(LauncherTheme::style_scrollable_flat_extra_dark).into()
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
        } else if self.is_b173_being_installed {
            widget::column![widget::text("Installing OptiFine for Beta 1.7.3...").size(20)]
        } else {
            self.install_optifine_screen()
        }
        .padding(10)
        .spacing(10)
        .into()
    }

    pub fn install_optifine_screen<'a>(
        &self,
    ) -> iced::widget::Column<'a, Message, LauncherTheme, iced::Renderer> {
        widget::column!(
            button_with_icon(icon_manager::back_with_size(14), "Back", 14).on_press(
                Message::ManageMods(ManageModsMessage::ScreenOpenWithoutUpdate)
            ),
            widget::container(
                widget::column!(
                    widget::text("Install OptiFine").size(20),
                    "Step 1: Open the OptiFine download page and download the installer.",
                    "WARNING: Make sure to download the correct version.",
                    widget::button("Open download page")
                        .on_press(Message::CoreOpenDir(self.get_url().to_owned()))
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
                number: progress_number,
                ..
            } => widget::column![
                button_with_icon(icon_manager::back(), "Back", 16)
                    .on_press(Message::CreateInstance(CreateInstanceMessage::Cancel)),
                widget::text("Loading version list...").size(20),
                widget::progress_bar(0.0..=24.0, *progress_number),
                widget::text(if *progress_number >= 1.0 {
                    format!("Downloading Omniarchive list {progress_number} / 26")
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
                        tooltip(
                            widget::checkbox("Download assets?", *download_assets).on_toggle(|t| Message::CreateInstance(CreateInstanceMessage::ChangeAssetToggle(t))),
                            widget::text("If disabled, creating instance will be MUCH faster, but no sound or music will play in-game").size(12),
                        ),
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
                    ].push_maybe(
                        (cfg!(target_os = "linux") && cfg!(target_arch = "x86"))
                            .then_some(
                                widget::column![
                                    // WARN: Linux i686
                                    widget::text("Warning: On your platform (Linux 32 bit) only Minecraft 1.16.5 and below are supported.").size(20),
                                    "If your computer isn't outdated, you might have wanted to download QuantumLauncher 64 bit (x86_64)",
                                ]
                            ))
                    .spacing(10)
                    .padding(10),
                )
                .style(LauncherTheme::style_scrollable_flat_dark)
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
            }
        }
    }
}

impl MenuInstallFabric {
    pub fn view(&self, selected_instance: &InstanceSelection, tick_timer: usize) -> Element {
        match self {
            MenuInstallFabric::Loading { is_quilt, .. } => {
                let loader_name = if *is_quilt { "Quilt" } else { "Fabric" };
                let dots = ".".repeat((tick_timer % 3) + 1);

                widget::column![
                    button_with_icon(icon_manager::back_with_size(14), "Back", 14).on_press(
                        Message::ManageMods(ManageModsMessage::ScreenOpenWithoutUpdate)
                    ),
                    widget::text!("Loading {loader_name} version list{dots}",).size(20)
                ]
            }
            MenuInstallFabric::Loaded {
                is_quilt,
                fabric_version,
                fabric_versions,
                progress,
            } => {
                let loader_name = if *is_quilt { "Quilt" } else { "Fabric" };

                if let Some(progress) = progress {
                    widget::column!(
                        widget::text!("Installing {loader_name}...").size(20),
                        progress.view(),
                    )
                } else {
                    widget::column![
                        button_with_icon(icon_manager::back_with_size(14), "Back", 14).on_press(
                            Message::ManageMods(ManageModsMessage::ScreenOpenWithoutUpdate)
                        ),
                        widget::text!(
                            "Install {loader_name} (instance: {})",
                            selected_instance.get_name()
                        )
                        .size(20),
                        widget::column![
                            widget::text!("{loader_name} version: (Ignore if you aren't sure)"),
                            widget::pick_list(
                                fabric_versions.as_slice(),
                                Some(fabric_version),
                                |n| Message::InstallFabric(InstallFabricMessage::VersionSelected(
                                    n
                                ))
                            ),
                        ]
                        .spacing(5),
                        button_with_icon(icon_manager::download(), "Install", 16)
                            .on_press(Message::InstallFabric(InstallFabricMessage::ButtonClicked)),
                    ]
                }
            }
            MenuInstallFabric::Unsupported(is_quilt) => {
                widget::column!(
                    button_with_icon(icon_manager::back_with_size(14), "Back", 14).on_press(
                        Message::ManageMods(ManageModsMessage::ScreenOpenWithoutUpdate)
                    ),
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
            widget::text("Installing Forge/NeoForge...").size(20),
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
                    ),
                    button_with_icon(icon_manager::page(), "Open Website", 16)
                        .on_press(Message::CoreOpenDir("https://mrmayman.github.com/quantumlauncher".to_owned())),
                ).push_maybe((cfg!(target_os = "linux")).then_some(
                    widget::column!(
                        // WARN: Package manager
                        "Note: If you installed this launcher from a package manager (apt/dnf/pacman/..) it's recommended to update from there",
                        "If you just downloaded it from the website then it's fine."
                    )
                )).push_maybe((cfg!(target_os = "macos")).then_some(
                    // WARN: macOS updater
                    "Note: The updater may be broken on macOS so download the new version from the website"
                ))
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
    pub fn view<'a>(&'a self, config: &'a LauncherConfig) -> Element<'a> {
        let (theme_list, style_list) = get_themes_and_styles(config);

        let config_view = widget::row!(
            widget::container(
                widget::column!(
                    "Select theme:",
                    theme_list,
                )
                .padding(10)
                .spacing(10)
            ),
            widget::container(
                widget::column!(
                    "Select style:",
                    style_list
                )
                .padding(10)
                .spacing(10)
            ),
            widget::container(
                widget::column![
                    "Change UI Scaling: (warning: slightly buggy)",
                    widget::slider(0.5..=2.0, self.temp_scale, |n| Message::LauncherSettings(
                        LauncherSettingsMessage::UiScale(n)
                    ))
                    .step(0.1),
                    widget::text!("Scale: {:.2}x", self.temp_scale),
                    widget::button("Apply").on_press(Message::LauncherSettings(
                        LauncherSettingsMessage::UiScaleApply
                    ))
                ]
                .padding(10)
                .spacing(10)
            ),
            widget::container(widget::column![
                button_with_icon(icon_manager::delete(), "Clear Java installs", 16)
                    .on_press(Message::LauncherSettings(LauncherSettingsMessage::ClearJavaInstalls)),
                widget::text("Might fix any problems with Java. Should be safe, you just need to redownload the Java Runtime").size(12),
            ].padding(10).spacing(10))
        )
        .spacing(10)
        .wrap();

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
                        widget::row![
                            button_with_icon(icon_manager::page(), "View Changelog", 16)
                                .on_press(Message::CoreOpenChangeLog),
                            button_with_icon(icon_manager::page(), "View Intro", 16)
                                .on_press(Message::CoreOpenIntro),
                        ].spacing(5).wrap(),
                        widget::row![
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
                        ].spacing(5).wrap(),
                        widget::column![
                            widget::text("QuantumLauncher is free and open source software under the GNU GPLv3 license.").size(12),
                            widget::text("No warranty is provided for this software.").size(12),
                            widget::text("You're free to share, modify, and redistribute it under the same license.").size(12),
                            widget::button("View License").on_press(
                                Message::CoreOpenDir("https://www.gnu.org/licenses/gpl-3.0.en.html".to_owned())
                            ),
                        ].spacing(5),
                        "If you like this launcher, consider sharing it with your friends.",
                        "Every new user motivates me to keep working on this :)"
                    )
                    .padding(10)
                    .spacing(10)
                ),
            )
            .padding(10)
            .spacing(10),
        )
        .style(LauncherTheme::style_scrollable_flat_extra_dark)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}

fn get_themes_and_styles(config: &LauncherConfig) -> (Element, Element) {
    // HOOK: Add more themes
    let themes = ["Dark".to_owned(), "Light".to_owned()];
    let styles = [
        "Brown".to_owned(),
        "Purple".to_owned(),
        "Sky Blue".to_owned(),
        "Catppuccin".to_owned(),
    ];

    let theme_list = widget::pick_list(themes, config.theme.clone(), |n| {
        Message::LauncherSettings(LauncherSettingsMessage::ThemePicked(n))
    })
    .into();

    let style_list = widget::pick_list(styles, config.style.clone(), |n| {
        Message::LauncherSettings(LauncherSettingsMessage::StylePicked(n))
    })
    .into();
    (theme_list, style_list)
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
            widget::text!("Code: {code}"),
            widget::button("Copy").on_press(Message::CoreCopyText(code.to_owned())),
            widget::text!("Link: {url}"),
            widget::button("Open").on_press(Message::CoreOpenDir(url.to_owned())),
            widget::vertical_space(),
        )
        .spacing(5)
        .align_x(iced::Alignment::Center),
        widget::horizontal_space()
    )
    .into()
}

impl MenuEditJarMods {
    pub fn view(&self, selected_instance: &InstanceSelection) -> Element {
        let menu_main = widget::row!(
            widget::container(
                widget::scrollable(
                    widget::column!(
                        button_with_icon(icon_manager::back(), "Back", 15)
                            .on_press(Message::ManageMods(ManageModsMessage::ScreenOpen)),
                        widget::column![
                            {
                                let path = {
                                    let path =
                                        selected_instance.get_instance_path().join("jarmods");
                                    path.exists().then_some(path.to_str().unwrap().to_owned())
                                };

                                button_with_icon(
                                    icon_manager::folder_with_size(14),
                                    "Open Folder",
                                    15,
                                )
                                .on_press_maybe(path.map(Message::CoreOpenDir))
                            },
                            button_with_icon(icon_manager::create(), "Add file", 15)
                                .on_press(Message::ManageJarMods(ManageJarModsMessage::AddFile)),
                        ]
                        .spacing(5),
                        widget::row![
                            "You can find some good jar mods at McArchive",
                            widget::button("Open")
                                .on_press(Message::CoreOpenDir("https://mcarchive.net".to_owned()))
                        ]
                        .spacing(5)
                        .wrap(),
                        widget::column![
                            "WARNING: Jarmods are mainly for OLD Minecraft versions.",
                            widget::text(
                                "This is easier than copying .class files into Minecraft's jar"
                            )
                            .size(12),
                            widget::text(
                                "If you just want some mods (for newer Minecraft), click Back"
                            )
                            .size(12),
                        ],
                    )
                    .padding(10)
                    .spacing(10)
                )
                .style(LauncherTheme::style_scrollable_flat_dark)
                .height(Length::Fill)
            )
            .width(250)
            .style(|n| n.style_container_sharp_box(0.0, Color::Dark)),
            self.get_mod_list()
        );

        if self.drag_and_drop_hovered {
            widget::stack!(
                menu_main,
                widget::center(widget::button(
                    widget::text("Drag and drop jarmod files to add them").size(20)
                ))
            )
            .into()
        } else {
            menu_main.into()
        }
    }

    fn get_mod_list(&self) -> Element {
        if self.jarmods.mods.is_empty() {
            return widget::column!("Add some mods to get started")
                .spacing(10)
                .padding(10)
                .width(Length::Fill)
                .into();
        }

        widget::container(
            widget::column!(
                widget::column![
                    widget::text("Select some jarmods to perform actions on them").size(14),
                    widget::row![
                        widget::button("Delete")
                            .on_press(Message::ManageJarMods(ManageJarModsMessage::DeleteSelected)),
                        widget::button("Toggle")
                            .on_press(Message::ManageJarMods(ManageJarModsMessage::ToggleSelected)),
                        widget::button(if matches!(self.selected_state, SelectedState::All) {
                            "Unselect All"
                        } else {
                            "Select All"
                        })
                        .on_press(Message::ManageJarMods(ManageJarModsMessage::SelectAll)),
                        widget::button(icon_manager::arrow_up())
                            .on_press(Message::ManageJarMods(ManageJarModsMessage::MoveUp)),
                        widget::button(icon_manager::arrow_down())
                            .on_press(Message::ManageJarMods(ManageJarModsMessage::MoveDown)),
                    ]
                    .spacing(5)
                    .wrap()
                ]
                .padding(10)
                .spacing(5),
                self.get_mod_list_contents(),
            )
            .spacing(10),
        )
        .style(|n| n.style_container_sharp_box(0.0, Color::ExtraDark))
        .into()
    }

    fn get_mod_list_contents(&self) -> Element {
        widget::scrollable(
            widget::column({
                self.jarmods.mods.iter().map(|jarmod| {
                    widget::checkbox(
                        format!(
                            "{}{}",
                            if jarmod.enabled { "" } else { "(DISABLED) " },
                            jarmod.filename
                        ),
                        self.selected_mods.contains(&jarmod.filename),
                    )
                    .on_toggle(move |t| {
                        Message::ManageJarMods(ManageJarModsMessage::ToggleCheckbox(
                            jarmod.filename.clone(),
                            t,
                        ))
                    })
                    .into()
                })
            })
            .padding(10)
            .spacing(10),
        )
        .style(LauncherTheme::style_scrollable_flat_extra_dark)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}

impl MenuCurseforgeManualDownload {
    pub fn view(&self) -> Element {
        widget::column![
            "Some Curseforge mods have blocked this launcher!\nYou need to manually download the files and add them to your mods",

            widget::scrollable(
                widget::column(self.unsupported.iter().map(|entry| {
                    let url = format!(
                        "https://www.curseforge.com/minecraft/{}/{}/download/{}",
                        entry.project_type,
                        entry.slug,
                        entry.file_id
                    );

                    widget::row![
                        widget::button(widget::text("Open link").size(14)).on_press(Message::CoreOpenDir(url)),
                        widget::text(&entry.name)
                    ]
                    .align_y(iced::Alignment::Center)
                    .spacing(10)
                    .into()
                }))
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .style(LauncherTheme::style_scrollable_flat_extra_dark),

            "Warning: Ignoring this may lead to crashes!",
            widget::row![
                widget::button("+ Select above downloaded files").on_press(Message::ManageMods(ManageModsMessage::AddFile)),
                widget::button("Continue").on_press(if self.is_store {
                    Message::InstallMods(InstallModsMessage::Open)
                } else {
                    Message::ManageMods(ManageModsMessage::ScreenOpenWithoutUpdate)
                }),
            ].spacing(5)
        ]
        .padding(10)
        .spacing(10)
        .into()
    }
}
