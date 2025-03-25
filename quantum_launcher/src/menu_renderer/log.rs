use iced::widget;
use ql_core::LOGGER;

use crate::{
    launcher_state::{Launcher, Message},
    stylesheet::styles::LauncherTheme,
};

impl Launcher {
    pub fn view_launcher_log(&self) -> (f64, f32, widget::Column<'_, Message, LauncherTheme>) {
        let text = {
            if let Some(logger) = LOGGER.as_ref() {
                let logger = logger.lock().unwrap();
                logger.text.clone()
            } else {
                Vec::new()
            }
        };

        let screen_width = (self.window_size.0 / 7.3) as usize - 5;

        let text_len: f64 = text
            .iter()
            .map(|n| Self::split_string_len(&n.0, screen_width))
            .sum();

        let text_size = 12.0;
        let text_lines = (self.window_size.1 / (2.9 * text_size)).ceil() as usize;

        let mut slice = Vec::new();

        let mut start_pos = self.log_scroll as usize;
        let mut end_pos = start_pos + text_lines;

        for (msg, ty) in text {
            let word_wrapped = Self::split_string_at_intervals(&msg, screen_width);
            let len = word_wrapped.len();
            if len <= start_pos {
                start_pos -= len;
                end_pos -= len;
            } else if len <= end_pos {
                slice.push((word_wrapped[start_pos..len].to_vec(), ty));
                start_pos = 0;
                end_pos -= len;
            } else {
                slice.push((word_wrapped[start_pos..end_pos].to_vec(), ty));
                break;
            }
        }

        let column = widget::column(slice.into_iter().map(|(msg, ty)| {
            widget::row![
                widget::text(match ty {
                    ql_core::LogType::Info => ">",
                    ql_core::LogType::Error => "(!)",
                    ql_core::LogType::Point => "- ",
                })
                .size(text_size),
                widget::column(msg.into_iter().map(|line| {
                    widget::text(line)
                        .font(iced::Font::with_name("JetBrains Mono"))
                        .size(text_size)
                        .into()
                }))
            ]
            .spacing(5)
            .into()
        }))
        .push(widget::horizontal_space())
        .spacing(5);
        (text_len, text_size, column)
    }
}
