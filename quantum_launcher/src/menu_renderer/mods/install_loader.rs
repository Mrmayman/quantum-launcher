use iced::widget;
use ql_core::InstanceSelection;

use crate::{
    icon_manager,
    menu_renderer::{back_button, button_with_icon, Element},
    state::{
        InstallFabricMessage, InstallOptifineMessage, ManageModsMessage, MenuInstallFabric,
        MenuInstallForge, MenuInstallOptifine, Message,
    },
    stylesheet::styles::LauncherTheme,
};

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
            back_button().on_press(Message::ManageMods(
                ManageModsMessage::ScreenOpenWithoutUpdate
            )),
            widget::container(
                widget::column!(
                    widget::text("Install OptiFine").size(20),
                    "Step 1: Open the OptiFine download page and download the installer.",
                    "WARNING: Make sure to download the correct version.",
                    widget::button("Open download page")
                        .on_press(Message::CoreOpenLink(self.get_url().to_owned()))
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

impl MenuInstallFabric {
    pub fn view(&self, selected_instance: &InstanceSelection, tick_timer: usize) -> Element {
        match self {
            MenuInstallFabric::Loading { is_quilt, .. } => {
                let loader_name = if *is_quilt { "Quilt" } else { "Fabric" };
                let dots = ".".repeat((tick_timer % 3) + 1);

                widget::column![
                    back_button().on_press(Message::ManageMods(
                        ManageModsMessage::ScreenOpenWithoutUpdate
                    )),
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
                        back_button().on_press(Message::ManageMods(
                            ManageModsMessage::ScreenOpenWithoutUpdate
                        )),
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
                    back_button().on_press(Message::ManageMods(
                        ManageModsMessage::ScreenOpenWithoutUpdate
                    )),
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
