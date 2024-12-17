use crate::menu_renderer::Element;
use paste::paste;

const ICON_FONT: iced::Font = iced::Font::with_name("launcher-icons");

pub fn icon<'a>(codepoint: char) -> Element<'a> {
    iced::widget::text(codepoint).font(ICON_FONT).into()
}

pub fn icon_with_size<'a>(codepoint: char, size: u16) -> Element<'a> {
    iced::widget::text(codepoint)
        .font(ICON_FONT)
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

icon_define!(create, '\u{e804}');
icon_define!(delete, '\u{e801}');
icon_define!(back, '\u{e802}');
icon_define!(play, '\u{e803}');
icon_define!(folder, '\u{e800}');
icon_define!(download, '\u{e805}');
icon_define!(settings, '\u{e806}');
icon_define!(save, '\u{e807}');
icon_define!(tick, '\u{e808}');
icon_define!(toggle, '\u{f204}');
icon_define!(update, '\u{e809}');
icon_define!(page, '\u{e80a}');
icon_define!(github, '\u{f09b}');
icon_define!(chat, '\u{e80b}');
icon_define!(arrow_up, '\u{f102}');
icon_define!(arrow_down, '\u{f103}');
