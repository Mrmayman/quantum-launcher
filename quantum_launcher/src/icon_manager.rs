//! All the icons to be shown in the launcher's UI.
//! For example, play, delete, etc.
//!
//! The icons are designed by [Aurlt](https://github.com/Aurlt).
//!
//! # How this works
//! Internally, the icons are stored as a font,
//! where each character is an icon. When showing an
//! icon, a `widget::text` object is made with the icon font
//! and the special character that corresponds to the icon.

use crate::menu_renderer::Element;
use paste::paste;

const ICON_FONT2: iced::Font = iced::Font::with_name("QuantumLauncher");

pub fn icon<'a>(codepoint: char) -> Element<'a> {
    iced::widget::text(codepoint).font(ICON_FONT2).into()
}

pub fn icon_with_size<'a>(codepoint: char, size: u16) -> Element<'a> {
    iced::widget::text(codepoint)
        .font(ICON_FONT2)
        .size(size)
        .into()
}

macro_rules! icon_define {
    ($name:ident, $unicode:expr) => {
        paste! {
            #[allow(dead_code)]
            pub fn $name<'a>() -> Element<'a> {
                icon($unicode)
            }

            #[allow(dead_code)]
            pub fn [<$name _with_size>]<'a>(size: u16) -> Element<'a> {
                icon_with_size($unicode, size)
            }
        }
    };
}

icon_define!(update, '\u{e901}');
icon_define!(play, '\u{e902}');
icon_define!(delete, '\u{e903}');
icon_define!(filter, '\u{e904}');
icon_define!(folder, '\u{e905}');
icon_define!(github, '\u{e906}');
icon_define!(create, '\u{e907}');
icon_define!(back, '\u{e908}');
// icon_define!(back, '\u{e909}');
icon_define!(chat, '\u{e90A}');
icon_define!(tick, '\u{e90B}');
icon_define!(tick2, '\u{e90C}');
icon_define!(discord, '\u{e90D}');
icon_define!(arrow_down, '\u{e90E}');
icon_define!(download, '\u{e90F}');
icon_define!(edit, '\u{e923}');
icon_define!(sort, '\u{e900}'); // A-Z, Version, Playtime, Date Created, etc...

icon_define!(download_file, '\u{e910}');
icon_define!(settings_file, '\u{e911}');
icon_define!(text_file, '\u{e912}');
icon_define!(jar_file, '\u{e913}');
icon_define!(zip_file, '\u{e914}');
icon_define!(blank_file, '\u{e915}');

icon_define!(save, '\u{e916}');
icon_define!(settings, '\u{e917}');
icon_define!(globe, '\u{e918}');
icon_define!(three_lines, '\u{e919}');
icon_define!(logo, '\u{e91A}');
icon_define!(tick3, '\u{e91B}');

icon_define!(toggle_off, '\u{e91C}');
icon_define!(toggle_on, '\u{e91D}');

icon_define!(arrow_up, '\u{e91E}');
icon_define!(refresh_clock, '\u{e91F}'); // This is should be an update icon

icon_define!(chatbox_alt, '\u{e920}'); // This is experimental i guess idk
icon_define!(mode_dark, '\u{e921}');
icon_define!(mode_light, '\u{e923}');
