use crate::menu_renderer::Element;

const ICON_FONT: iced::Font = iced::Font::with_name("launcher-icons");

pub fn icon<'a>(codepoint: char) -> Element<'a> {
    iced::widget::text(codepoint).font(ICON_FONT).into()
}

macro_rules! icon_define {
    ($name:ident, $unicode:expr) => {
        pub fn $name<'a>() -> Element<'a> {
            icon($unicode)
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
// icon_define!(save, '\u{e807}');
