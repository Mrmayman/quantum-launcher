//! A "log viewer" made with iced widgets.
//!
//! This is honestly complete and utter garbage.
//! I barely knew what I was doing when I made this,
//! and better solutions definitely might exist.
//!
//! It supports:
//! - Basic "word-wrapping" (cuts off words sometimes)
//! - Rough, per-line scrolling (because `iced::widget::scrollable`
//!   is magic I can't replicate).
//! - Click to copy a line to clipboard.
//!
//! `iced::widget::scrollable` renders the whole damn thing, not slices,
//! so I can't use it. I can't figure out how to make this as smooth as
//! `scrollable`.
//!
//! **NOTE: You have to handroll your own scrollbar**.
//! It's pretty simple through, the integration/hooks are already there,
//! you just need to place the final widget. See the places where
//! [`Launcher::view_launcher_log`] is called, and look for a
//! `widget::vertical_slider`, then copy the code.
//!
//! The way this works is,
//! it assumes logs are a big list of lines.
//! This renders a "subslice" of this big list.
//!
//! For example:
//!
//! ```txt
//! [info] Starting up launcher
//! [info] Installing fabric             <--|
//! - Downloading library: (1/7)            |
//! - Downloading library: (2/7)            |
//! - Downloading library: (3/7)            |
//! - Downloading library: (4/7)            |
//! - Downloading library: (5/7)         <--|
//! - Downloading library: (6/7)
//! - Downloading library: (7/7)
//! [info] Finished installing fabric
//! ```
//!
//! You can see uses of this in the following
//! (but not limited to) places:
//! - Instance log viewer
//! - Launcher debug log (the bottom bar)
//!
//! See [`Launcher::view_launcher_log`] for more info.

use iced::widget;
use ql_core::LogType;

use crate::{
    state::{Launcher, Message},
    stylesheet::{styles::LauncherTheme, widgets::StyleButton},
};

use super::Element;

impl Launcher {
    /// Renders the log. See the [`crate::menu_renderer::log`] module
    /// documentation for basic context.
    ///
    /// # Arguments
    /// - `text`: A list of log lines and their type
    ///   (info/error/point)
    /// - `text_size`: The size of the log text.
    ///   Recommended: `12.0`
    /// - `scroll`: The amount of lines scrolled down.
    ///   `0` for the beginning, add 1 to it as you scroll down.
    /// - `width_reduction`, `height_reduction`: How much the other
    ///   UI elements "eat into" the log space. I'll explain this in layouting.
    ///
    /// # Returns
    /// A tuple of:
    /// - `f64`: the total height of the log in lines
    ///   **including word wrapping, so it's not just `text.len()`**.
    ///   You can use this in your handrolled scrollbar, by representing scroll as
    ///   `(length - scroll) / length` (length is this number).
    /// - `Element`: The iced element of the log, put this in your GUI.
    ///
    /// # Layouting
    /// I made a deliberate (although questionable) design choice implementing this as
    /// "cutting-into"s of the total space, instead of a width and height of the log.
    ///
    /// This was mainly for ease of implementation and because it was ok for the use case
    /// of this log viewer. It's not exactly nice but it works.
    ///
    /// Essentially, lets say you have this GUI:
    ///
    /// ```txt
    /// + new| [info] Starting  |
    /// -----|   launcher...    |
    /// hello| [info] Installing|
    /// world|   fabric.        |
    /// blah | - Downloading    |
    /// foo  |   library (1/7)  |
    /// bar  | - Downloading    |
    /// -------------------------
    /// ```
    ///
    /// Here, the sidebar and bottom bar "cut into"
    /// the space of the log. In this terminology
    /// we assume the log is destined to occupy the
    /// entire screen but is unfortunately trespassed upon.
    ///
    /// So `width_reduction` would include the width of the
    /// sidebar, and `height_reduction` would include the
    /// height of the bottom bar.
    pub fn view_launcher_log(
        &self,
        text: &[(String, LogType)],
        text_size: f32,
        scroll: isize,
        width_reduction: f32,
        height_reduction: f32,
    ) -> (f64, Element) {
        let screen_width = (self.window_size.0 - width_reduction) / 7.4;
        if screen_width < 5.0 {
            return (0.0, widget::column![].into());
        }
        let screen_width = screen_width as usize - 5;
        let height_limit = (self.window_size.1 - height_reduction) - 50.0;

        let slice =
            Self::calculate_word_wrapping(text, text_size, scroll, screen_width, height_limit);

        let column = widget::column(slice.into_iter().map(|(msg, ty, i)| {
            // For copy pasting
            let msg_text = text
                .get(i)
                .map_or_else(|| msg.join(""), |(line, _)| line.clone());

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

        (
            text.iter()
                .map(|n| Self::split_string_len(&n.0, screen_width))
                .sum(),
            column.into(),
        )
    }

    fn split_string_len(input: &str, interval: usize) -> f64 {
        (input.len() as f64 / interval as f64).ceil()
    }

    fn calculate_word_wrapping(
        text: &[(String, LogType)],
        text_size: f32,
        scroll: isize,
        screen_width: usize,
        height_limit: f32,
    ) -> Vec<(Vec<String>, &LogType, usize)> {
        let mut slice = Vec::new();
        let mut start_pos = scroll as usize;
        let mut achieved_height = 0.0;

        for (i, (msg, ty)) in text.iter().enumerate() {
            let word_wrapped = Self::split_string_at_intervals(msg, screen_width);
            let len = word_wrapped.len();

            if len <= start_pos {
                start_pos -= len;
            } else {
                achieved_height += (len - start_pos) as f32 * text_size;
                slice.push((word_wrapped[start_pos..len].to_vec(), ty, i));
                start_pos = 0;
            }

            if achieved_height > height_limit {
                break;
            }
        }
        slice
    }

    fn split_string_at_intervals(input: &str, interval: usize) -> Vec<String> {
        input
            .chars()
            .collect::<Vec<char>>()
            .chunks(interval)
            .map(|chunk| chunk.iter().collect())
            .collect()
    }
}
