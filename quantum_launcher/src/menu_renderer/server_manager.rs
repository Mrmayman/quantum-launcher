use iced::widget;

use crate::{
    icon_manager,
    launcher_state::{MenuServerCreate, Message},
};

use super::{button_with_icon, Element};

impl MenuServerCreate {
    pub fn view(&self) -> Element {
        match self {
            MenuServerCreate::LoadingList {
                progress_number, ..
            } => {
                widget::column!(
                    widget::text("Loading version list...").size(20),
                    widget::progress_bar(0.0..=16.0, *progress_number),
                    widget::text(if *progress_number >= 1.0 {
                        format!("Downloading Omniarchive list {progress_number} / 17")
                    } else {
                        "Downloading official version list".to_owned()
                    })
                )
            }
            MenuServerCreate::Loaded {
                name,
                versions,
                selected_version,
                ..
            } => {
                widget::column!(
                    button_with_icon(icon_manager::back(), "Back", 16).on_press(
                        Message::ServerManageOpen {
                            selected_server: None,
                            message: None
                        }
                    ),
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
