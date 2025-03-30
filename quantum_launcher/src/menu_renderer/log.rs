use iced::widget;
use ql_core::LogType;

use crate::{
    launcher_state::{Launcher, Message},
    stylesheet::{styles::LauncherTheme, widgets::StyleButton},
};

use super::Element;

impl Launcher {
    pub fn view_launcher_log(
        &self,
        text: &[(String, LogType)],
        text_size: f32,
        scroll: i64,
        width_reduction: f32,
        height_reduction: f32,
    ) -> (f64, Element) {
        let screen_width = ((self.window_size.0 - width_reduction) / 7.4) as usize - 5;
        let height_limit = (self.window_size.1 - height_reduction) - 50.0;

        let text_len: f64 = text
            .iter()
            .map(|n| Self::split_string_len(&n.0, screen_width))
            .sum();

        let mut slice = Vec::new();
        let mut start_pos = scroll as usize;
        let mut achieved_height = 0.0;

        for (msg, ty) in text {
            let word_wrapped = Self::split_string_at_intervals(msg, screen_width);
            let len = word_wrapped.len();

            if len <= start_pos {
                start_pos -= len;
            } else {
                achieved_height += (len - start_pos) as f32 * text_size;
                slice.push((word_wrapped[start_pos..len].to_vec(), ty));
                start_pos = 0;
            }

            if achieved_height > height_limit {
                break;
            }
        }

        let column = widget::column(slice.into_iter().map(|(msg, ty)| {
            let msg_text: String = msg.iter().fold(String::new(), |n, v| n + v);

            widget::button(
                widget::row![
                    widget::text(match ty {
                        ql_core::LogType::Info => ">",
                        ql_core::LogType::Error => "(!)",
                        ql_core::LogType::Point => "",
                    })
                    .size(text_size),
                    widget::column(msg.into_iter().map(|line| {
                        widget::text(line)
                            .font(iced::Font::with_name("JetBrains Mono"))
                            .size(text_size)
                            .into()
                    })),
                    widget::horizontal_space()
                ]
                .spacing(5),
            )
            .padding(0)
            .style(|n: &LauncherTheme, status| n.style_button(status, StyleButton::FlatExtraDark))
            .on_press(Message::CoreCopyText(msg_text))
            .into()
        }))
        .push(widget::horizontal_space())
        .spacing(4);
        (text_len, column.into())
    }

    fn split_string_len(input: &str, interval: usize) -> f64 {
        (input.len() as f64 / interval as f64).ceil()
    }
}
