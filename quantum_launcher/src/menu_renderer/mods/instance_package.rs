use iced::{widget, Length};

use crate::{
    icon_manager,
    menu_renderer::{back_button, button_with_icon, Element},
    state::{MenuExportInstance, Message},
};

impl MenuExportInstance {
    pub fn view(&self, tick_timer: usize) -> Element {
        widget::column![
            back_button().on_press(Message::LaunchScreenOpen {
                message: None,
                clear_selection: false
            }),
            "Select the contents of \".minecraft\" folder you want to keep",
            widget::scrollable(if let Some(entries) = &self.entries {
                widget::column(entries.iter().enumerate().map(|(i, (entry, enabled))| {
                    let name = if entry.is_file {
                        entry.name.clone()
                    } else {
                        format!("{}/", entry.name)
                    };
                    widget::checkbox(name, *enabled)
                        .on_toggle(move |t| Message::ExportInstanceToggleItem(i, t))
                        .into()
                }))
                .padding(5)
            } else {
                let dots = ".".repeat((tick_timer % 3) + 1);
                widget::column!(widget::text!("Loading{dots}"))
            })
            .width(Length::Fill)
            .height(Length::Fill),
            widget::column![
                widget::text("Format:").size(12),
                widget::row![
                    widget::pick_list(["QuantumLauncher"], Some("QuantumLauncher"), |_| {
                        Message::Nothing
                    })
                    .text_line_height(1.68),
                    button_with_icon(icon_manager::save(), "Export", 16)
                        .on_press(Message::ExportInstanceStart),
                ]
                .spacing(5)
                .wrap()
            ]
            .spacing(2),
        ]
        .padding(10)
        .spacing(10)
        .into()
    }
}
