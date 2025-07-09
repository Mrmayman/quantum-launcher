use iced::{widget, Length};
use ql_core::{InstanceSelection, Progress};

use crate::{
    config::LauncherConfig,
    icon_manager,
    state::{
        CreateInstanceMessage, InstallModsMessage, LauncherSettingsMessage, ManageModsMessage,
        MenuCreateInstance, MenuCurseforgeManualDownload, MenuLauncherUpdate, MenuServerCreate,
        Message, ProgressBar,
    },
    stylesheet::{color::Color, styles::LauncherTheme},
};

pub mod changelog;
mod edit_instance;
mod launch;
mod log;
mod login;
mod mods;
mod settings;

pub const DISCORD: &str = "https://discord.gg/bWqRaSXar5";
pub const GITHUB: &str = "https://github.com/Mrmayman/quantumlauncher";

pub type Element<'a> = iced::Element<'a, Message, LauncherTheme, iced::Renderer>;

pub fn center_x<'a>(e: impl Into<Element<'a>>) -> Element<'a> {
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

pub fn back_button<'a>() -> widget::Button<'a, Message, LauncherTheme, iced::Renderer> {
    button_with_icon(icon_manager::back_with_size(14), "Back", 14)
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

impl MenuCreateInstance {
    pub fn view(&self) -> Element {
        match self {
            MenuCreateInstance::LoadingList { .. } => widget::column![
                widget::row![
                    back_button().on_press(Message::CreateInstance(CreateInstanceMessage::Cancel)),
                    button_with_icon(icon_manager::folder(), "Import Instance", 16)
                        .on_press(Message::CreateInstance(CreateInstanceMessage::Import)),
                ]
                .spacing(5),
                widget::text("Loading version list...").size(20),
            ]
            .padding(10)
            .spacing(10)
            .into(),
            MenuCreateInstance::Choosing {
                instance_name,
                selected_version,
                download_assets,
                combo_state,
                ..
            } => {
                widget::scrollable(
                    widget::column![
                        widget::row![
                            back_button()
                                .on_press(
                                    Message::LaunchScreenOpen {
                                        message: None,
                                        clear_selection: false
                                }),
                            button_with_icon(icon_manager::folder(), "Import Instance", 16)
                                .on_press(Message::CreateInstance(CreateInstanceMessage::Import)),
                        ]
                        .spacing(5),
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
                        ).on_press_maybe((selected_version.is_some() && !instance_name.is_empty()).then(|| Message::CreateInstance(CreateInstanceMessage::Start))),
                        widget::text("To install Fabric/Forge/OptiFine/Quilt, click on Mods after installing the instance").size(12),
                    ].push_maybe(
                        {
                            let real_platform = if cfg!(target_arch = "x86") { "x86_64" } else { "aarch64" };
                            (cfg!(target_os = "linux") && (cfg!(target_arch = "x86") || cfg!(target_arch = "arm")))
                            .then_some(
                                widget::column![
                                    // WARN: Linux i686 and arm32
                                    widget::text("Warning: On your platform (Linux 32 bit) only Minecraft 1.16.5 and below are supported.").size(20),
                                    widget::text!("If your computer isn't outdated, you might have wanted to download QuantumLauncher 64 bit ({real_platform})"),
                                ]
                            )})
                    .spacing(10)
                    .padding(10),
                )
                .style(LauncherTheme::style_scrollable_flat_dark)
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
            }
            MenuCreateInstance::DownloadingInstance(progress) => widget::column![
                widget::text("Downloading Instance..").size(20),
                progress.view()
            ]
            .padding(10)
            .spacing(5)
            .into(),
            MenuCreateInstance::ImportingInstance(progress) => widget::column![
                widget::text("Importing Instance..").size(20),
                progress.view()
            ]
            .padding(10)
            .spacing(5)
            .into(),
        }
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
                    back_button().on_press(
                        Message::LaunchScreenOpen {
                            message: None,
                            clear_selection: false
                        }
                    ),
                    button_with_icon(icon_manager::page(), "Open Website", 16)
                        .on_press(Message::CoreOpenLink("https://mrmayman.github.com/quantumlauncher".to_owned())),
                ).push_maybe((cfg!(target_os = "linux")).then_some(
                    widget::column!(
                        // WARN: Package manager
                        "Note: If you installed this launcher from a package manager (flatpak/apt/dnf/pacman/..) it's recommended to update from there",
                        "If you just downloaded it from the website then continue from here."
                    )
                )).push_maybe((cfg!(target_os = "macos")).then_some(
                    // WARN: macOS updater
                    "Note: The updater may be broken on macOS, so download the new version from the website"
                ))
                .spacing(5),
            )
        }
        .padding(10)
        .spacing(10)
        .into()
    }
}

pub fn get_theme_selector(config: &LauncherConfig) -> (Element, Element) {
    const PADDING: iced::Padding = iced::Padding {
        top: 5.0,
        bottom: 5.0,
        right: 10.0,
        left: 10.0,
    };

    let theme = config.theme.as_deref().unwrap_or("Dark");
    let (light, dark): (Element, Element) = if theme == "Dark" {
        (
            widget::button(widget::text("Light").size(14))
                .on_press(Message::LauncherSettings(
                    LauncherSettingsMessage::ThemePicked("Light".to_owned()),
                ))
                .into(),
            widget::container(widget::text("Dark").size(14))
                .padding(PADDING)
                .into(),
        )
    } else {
        (
            widget::container(widget::text("Light").size(14))
                .padding(PADDING)
                .into(),
            widget::button(widget::text("Dark").size(14))
                .on_press(Message::LauncherSettings(
                    LauncherSettingsMessage::ThemePicked("Dark".to_owned()),
                ))
                .into(),
        )
    };
    (light, dark)
}

fn get_color_schemes(config: &LauncherConfig) -> Element {
    // HOOK: Add more themes
    let styles = [
        "Brown".to_owned(),
        "Purple".to_owned(),
        "Sky Blue".to_owned(),
        "Catppuccin".to_owned(),
        "Teal".to_owned(),
    ];

    widget::pick_list(styles, config.style.clone(), |n| {
        Message::LauncherSettings(LauncherSettingsMessage::StylePicked(n))
    })
    .into()
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
                        widget::button(widget::text("Open link").size(14)).on_press(Message::CoreOpenLink(url)),
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

impl MenuServerCreate {
    pub fn view(&self) -> Element {
        match self {
            MenuServerCreate::LoadingList => {
                widget::column!(widget::text("Loading version list...").size(20),)
            }
            MenuServerCreate::Loaded {
                name,
                versions,
                selected_version,
                ..
            } => {
                widget::column!(
                    back_button().on_press(Message::ServerManageOpen {
                        selected_server: None,
                        message: None
                    }),
                    widget::text("Create new server").size(20),
                    widget::combo_box(
                        versions,
                        "Select a version...",
                        selected_version.as_ref(),
                        Message::ServerCreateVersionSelected
                    ),
                    widget::text_input("Enter server name...", name)
                        .on_input(Message::ServerCreateNameInput),
                    widget::button("Create Server").on_press_maybe(
                        (selected_version.is_some() && !name.is_empty())
                            .then(|| Message::ServerCreateStart)
                    ),
                )
            }
            MenuServerCreate::Downloading { progress } => {
                widget::column!(widget::text("Creating Server...").size(20), progress.view())
            }
        }
        .padding(10)
        .spacing(10)
        .into()
    }
}
