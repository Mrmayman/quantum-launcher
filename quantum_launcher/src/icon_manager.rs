use crate::menu_renderer::Element;
use paste::paste;

const ICON_FONT: iced::Font = iced::Font::with_name("launcher-icons");
const ICON_FONT2: iced::Font = iced::Font::with_name("QuantumLauncher");

pub fn icon<'a>(codepoint: char) -> Element<'a> {
    iced::widget::text(codepoint).font(ICON_FONT).into()
}
pub fn icon2<'a>(codepoint: char) -> Element<'a> {
    iced::widget::text(codepoint).font(ICON_FONT2).into()
}

pub fn icon_with_size<'a>(codepoint: char, size: u16) -> Element<'a> {
    iced::widget::text(codepoint)
        .font(ICON_FONT)
        .size(size)
        .into()
}
pub fn icon2_with_size<'a>(codepoint: char, size: u16) -> Element<'a> {
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
macro_rules! icon_define2 {
    ($name:ident, $unicode:expr) => {
        paste! {
            #[allow(dead_code)]
            pub fn $name<'a>() -> Element<'a> {
                icon2($unicode)
            }

            #[allow(dead_code)]
            pub fn [<$name _with_size>]<'a>(size: u16) -> Element<'a> {
                icon2_with_size($unicode, size)
            }
        }
    };
}

// # New icons, designed by Aurlt

icon_define2!(update, '\u{e901}');
icon_define2!(play, '\u{e902}');
icon_define2!(delete, '\u{e903}');
icon_define2!(filter, '\u{e904}');
icon_define2!(folder, '\u{e905}');
icon_define2!(github, '\u{e906}');
icon_define2!(create, '\u{e907}');
//
//
icon_define2!(chat, '\u{e90A}');
icon_define2!(tick, '\u{e90B}');
icon_define2!(tick2, '\u{e90C}');
icon_define2!(discord, '\u{e90D}');
icon_define2!(arrow_down, '\u{e90E}');
icon_define2!(download, '\u{e90F}');

icon_define2!(download_file, '\u{e910}');
icon_define2!(settings_file, '\u{e911}');
icon_define2!(text_file, '\u{e912}');
icon_define2!(jar_file, '\u{e913}');
icon_define2!(zip_file, '\u{e914}');
icon_define2!(blank_file, '\u{e915}');

icon_define2!(save, '\u{e916}');
icon_define2!(settings, '\u{e917}');
icon_define2!(globe, '\u{e918}');
icon_define2!(three_lines, '\u{e919}');
icon_define2!(logo, '\u{e91A}');
icon_define2!(tick3, '\u{e91B}');

icon_define2!(toggle_off, '\u{e91C}');
icon_define2!(toggle_on, '\u{e91D}');

icon_define2!(arrow_up, '\u{e91E}');
icon_define2!(refresh_clock, '\u{e91F}');

// # Old icons grabbed from fontello

// icon_define!(create, '\u{e804}');
// icon_define!(delete, '\u{e801}');
icon_define!(back, '\u{e802}');
// icon_define!(play, '\u{e803}');
// icon_define!(folder, '\u{e800}');
// icon_define!(download, '\u{e805}');
// icon_define!(settings, '\u{e806}');
// icon_define!(save, '\u{e807}');
// icon_define!(tick, '\u{e808}');
// icon_define!(toggle, '\u{f204}');
// icon_define!(update, '\u{e809}');
// icon_define!(page, '\u{e80a}');
// icon_define!(github, '\u{f09b}');
// icon_define!(chat, '\u{e80b}');
// icon_define!(arrow_up, '\u{f102}');
// icon_define!(arrow_down, '\u{f103}');
