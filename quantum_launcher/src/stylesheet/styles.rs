use std::sync::{Arc, Mutex};

use iced::widget;
use lazy_static::lazy_static;

use super::color::{Color, BROWN, LIGHT_BLUE, PURPLE};

pub const BORDER_WIDTH: f32 = 2.0;
pub const BORDER_RADIUS: f32 = 8.0;

lazy_static! {
    pub static ref STYLE: Arc<Mutex<LauncherStyle>> = Arc::new(Mutex::new(LauncherStyle::Purple));
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub enum LauncherStyle {
    Brown,
    Purple,
    LightBlue,
}

impl Default for LauncherStyle {
    fn default() -> Self {
        STYLE.lock().unwrap().clone()
    }
}

#[derive(Clone, Default, Debug)]
pub enum LauncherTheme {
    #[default]
    Dark,
    Light,
}

impl LauncherTheme {
    fn get(&self, style: &LauncherStyle, color: Color, invert: bool) -> iced::Color {
        let (palette, color) = self.get_base(style, invert, color);
        palette.get(color)
    }

    fn get_base(
        &self,
        style: &LauncherStyle,
        invert: bool,
        color: Color,
    ) -> (&super::color::Pallete, Color) {
        let palette = match style {
            LauncherStyle::Brown => &BROWN,
            LauncherStyle::Purple => &PURPLE,
            LauncherStyle::LightBlue => &LIGHT_BLUE,
        };
        let color = if invert {
            match self {
                LauncherTheme::Dark => color,
                LauncherTheme::Light => color.invert(),
            }
        } else {
            match self {
                LauncherTheme::Dark => color.invert(),
                LauncherTheme::Light => color,
            }
        };
        (palette, color)
    }

    fn get_bg(&self, style: &LauncherStyle, color: Color, invert: bool) -> iced::Background {
        let (palette, color) = self.get_base(style, invert, color);
        palette.get_bg(color)
    }

    fn get_border(&self, style: &LauncherStyle, color: Color, invert: bool) -> iced::Border {
        let (palette, color) = self.get_base(style, invert, color);
        palette.get_border(color)
    }
}

impl widget::container::StyleSheet for LauncherTheme {
    type Style = LauncherStyle;

    fn appearance(&self, style: &Self::Style) -> widget::container::Appearance {
        widget::container::Appearance {
            text_color: Some(self.get(style, Color::Light, true)),
            background: Some(self.get_bg(style, Color::Dark, true)),
            border: self.get_border(style, Color::SecondDark, true),
            ..Default::default()
        }
    }
}

impl widget::button::StyleSheet for LauncherTheme {
    type Style = LauncherStyle;

    fn active(&self, style: &Self::Style) -> widget::button::Appearance {
        widget::button::Appearance {
            background: Some(self.get_bg(style, Color::SecondDark, true)),
            text_color: self.get(style, Color::White, true),
            border: self.get_border(style, Color::SecondDark, true),
            ..Default::default()
        }
    }

    fn hovered(&self, style: &Self::Style) -> widget::button::Appearance {
        widget::button::Appearance {
            background: Some(iced::Background::Color(self.get(style, Color::Mid, true))),
            text_color: self.get(style, Color::Dark, true),
            border: self.get_border(style, Color::Mid, true),
            ..Default::default()
        }
    }

    fn pressed(&self, style: &Self::Style) -> widget::button::Appearance {
        widget::button::Appearance {
            background: Some(iced::Background::Color(self.get(style, Color::White, true))),
            text_color: self.get(style, Color::Dark, true),
            border: self.get_border(style, Color::White, true),
            ..Default::default()
        }
    }

    fn disabled(&self, style: &Self::Style) -> widget::button::Appearance {
        widget::button::Appearance {
            background: Some(iced::Background::Color(self.get(
                style,
                Color::SecondDark,
                true,
            ))),
            text_color: self.get(style, Color::SecondLight, true),
            border: self.get_border(style, Color::SecondDark, true),
            ..Default::default()
        }
    }
}

impl widget::text::StyleSheet for LauncherTheme {
    type Style = LauncherStyle;

    fn appearance(&self, _style: Self::Style) -> widget::text::Appearance {
        widget::text::Appearance { color: None }
    }
}

impl widget::pick_list::StyleSheet for LauncherTheme {
    type Style = LauncherStyle;

    fn active(
        &self,
        style: &<Self as widget::pick_list::StyleSheet>::Style,
    ) -> widget::pick_list::Appearance {
        widget::pick_list::Appearance {
            text_color: self.get(style, Color::Dark, false),
            placeholder_color: self.get(style, Color::SecondDark, false),
            handle_color: self.get(style, Color::Dark, false),
            background: iced::Background::Color(self.get(style, Color::Light, false)),
            border: self.get_border(style, Color::Mid, false),
        }
    }

    fn hovered(
        &self,
        style: &<Self as widget::pick_list::StyleSheet>::Style,
    ) -> widget::pick_list::Appearance {
        widget::pick_list::Appearance {
            text_color: self.get(style, Color::Dark, false),
            placeholder_color: self.get(style, Color::SecondDark, false),
            handle_color: self.get(style, Color::Dark, false),
            background: self.get_bg(style, Color::SecondLight, false),
            border: self.get_border(style, Color::SecondLight, false),
        }
    }
}

impl widget::overlay::menu::StyleSheet for LauncherTheme {
    type Style = LauncherStyle;

    fn appearance(&self, style: &Self::Style) -> iced::overlay::menu::Appearance {
        iced::overlay::menu::Appearance {
            text_color: self.get(style, Color::White, true),
            background: self.get_bg(style, Color::SecondDark, true),
            border: self.get_border(style, Color::Mid, true),
            selected_text_color: self.get(style, Color::Dark, true),
            selected_background: self.get_bg(style, Color::SecondLight, true),
        }
    }
}

impl widget::scrollable::StyleSheet for LauncherTheme {
    type Style = LauncherStyle;

    fn active(&self, style: &Self::Style) -> widget::scrollable::Appearance {
        widget::scrollable::Appearance {
            container: widget::container::Appearance {
                text_color: None,
                background: None,
                border: {
                    let color = Color::SecondDark;
                    let palette = match style {
                        LauncherStyle::Brown => &BROWN,
                        LauncherStyle::Purple => &PURPLE,
                        LauncherStyle::LightBlue => &LIGHT_BLUE,
                    };
                    let color = match self {
                        LauncherTheme::Dark => color,
                        LauncherTheme::Light => {
                            if matches!(style, LauncherStyle::Purple) {
                                Color::SecondDark
                            } else {
                                color.invert()
                            }
                        }
                    };
                    palette.get_border(color)
                },
                shadow: iced::Shadow::default(),
            },
            scrollbar: widget::scrollable::Scrollbar {
                background: Some(self.get_bg(style, Color::Dark, true)),
                border: self.get_border(style, Color::SecondDark, true),
                scroller: widget::scrollable::Scroller {
                    color: self.get(style, Color::White, true),
                    border: self.get_border(style, Color::Light, true),
                },
            },
            gap: None,
        }
    }

    fn hovered(
        &self,
        style: &Self::Style,
        _is_mouse_over_scrollbar: bool,
    ) -> widget::scrollable::Appearance {
        widget::scrollable::Appearance {
            container: widget::container::Appearance {
                text_color: None,
                background: None,
                border: self.get_border(style, Color::Mid, true),
                shadow: iced::Shadow::default(),
            },
            scrollbar: widget::scrollable::Scrollbar {
                background: Some(self.get_bg(style, Color::Dark, true)),
                border: self.get_border(style, Color::SecondDark, true),
                scroller: widget::scrollable::Scroller {
                    color: self.get(style, Color::White, true),
                    border: self.get_border(style, Color::Light, true),
                },
            },
            gap: None,
        }
    }
}

impl widget::text_input::StyleSheet for LauncherTheme {
    type Style = LauncherStyle;

    fn active(&self, style: &Self::Style) -> widget::text_input::Appearance {
        widget::text_input::Appearance {
            background: self.get_bg(style, Color::SecondDark, true),
            border: self.get_border(style, Color::Mid, true),
            icon_color: iced::Color::default(),
        }
    }

    fn focused(&self, style: &Self::Style) -> widget::text_input::Appearance {
        widget::text_input::Appearance {
            background: self.get_bg(style, Color::SecondDark, true),
            border: self.get_border(style, Color::Mid, true),
            icon_color: iced::Color::default(),
        }
    }

    fn placeholder_color(&self, style: &Self::Style) -> iced::Color {
        self.get(style, Color::SecondLight, true)
    }

    fn value_color(&self, style: &Self::Style) -> iced::Color {
        self.get(style, Color::White, true)
    }

    fn disabled_color(&self, style: &Self::Style) -> iced::Color {
        self.get(style, Color::SecondDark, true)
    }

    fn selection_color(&self, style: &Self::Style) -> iced::Color {
        self.get(style, Color::SecondLight, true)
    }

    fn disabled(&self, style: &Self::Style) -> widget::text_input::Appearance {
        widget::text_input::Appearance {
            background: self.get_bg(style, Color::Dark, true),
            border: self.get_border(style, Color::SecondDark, true),
            icon_color: iced::Color::default(),
        }
    }
}

impl widget::progress_bar::StyleSheet for LauncherTheme {
    type Style = LauncherStyle;

    fn appearance(&self, style: &Self::Style) -> widget::progress_bar::Appearance {
        widget::progress_bar::Appearance {
            background: self.get_bg(style, Color::SecondDark, true),
            bar: self.get_bg(style, Color::Light, true),
            border_radius: BORDER_RADIUS.into(),
        }
    }
}

impl widget::slider::StyleSheet for LauncherTheme {
    type Style = LauncherStyle;

    fn active(&self, style: &Self::Style) -> widget::slider::Appearance {
        widget::slider::Appearance {
            rail: widget::slider::Rail {
                colors: (
                    self.get(style, Color::Mid, true),
                    self.get(style, Color::SecondDark, true),
                ),
                width: 4.0,
                border_radius: BORDER_RADIUS.into(),
            },
            handle: widget::slider::Handle {
                shape: widget::slider::HandleShape::Circle { radius: 8.0 },
                color: self.get(style, Color::SecondLight, true),
                border_width: 2.0,
                border_color: self.get(style, Color::Light, true),
            },
        }
    }

    fn hovered(&self, style: &Self::Style) -> widget::slider::Appearance {
        widget::slider::Appearance {
            rail: widget::slider::Rail {
                colors: (
                    self.get(style, Color::Light, true),
                    self.get(style, Color::Mid, true),
                ),
                width: 4.0,
                border_radius: BORDER_RADIUS.into(),
            },
            handle: widget::slider::Handle {
                shape: widget::slider::HandleShape::Circle { radius: 8.0 },
                color: self.get(style, Color::SecondLight, true),
                border_width: 2.0,
                border_color: self.get(style, Color::White, true),
            },
        }
    }

    fn dragging(&self, style: &Self::Style) -> widget::slider::Appearance {
        widget::slider::Appearance {
            rail: widget::slider::Rail {
                colors: (
                    self.get(style, Color::Mid, true),
                    self.get(style, Color::SecondDark, true),
                ),
                width: 6.0,
                border_radius: BORDER_RADIUS.into(),
            },
            handle: widget::slider::Handle {
                shape: widget::slider::HandleShape::Circle { radius: 12.0 },
                color: self.get(style, Color::White, true),
                border_width: 2.0,
                border_color: self.get(style, Color::White, true),
            },
        }
    }
}

impl iced::application::StyleSheet for LauncherTheme {
    type Style = LauncherStyle;

    fn appearance(&self, style: &Self::Style) -> iced::application::Appearance {
        iced::application::Appearance {
            background_color: self.get(style, Color::Dark, true),
            text_color: self.get(style, Color::Light, true),
        }
    }
}

impl iced::widget::checkbox::StyleSheet for LauncherTheme {
    type Style = LauncherStyle;

    fn active(&self, style: &Self::Style, is_checked: bool) -> widget::checkbox::Appearance {
        iced::widget::checkbox::Appearance {
            background: if is_checked {
                self.get_bg(style, Color::Light, true)
            } else {
                self.get_bg(style, Color::Dark, true)
            },
            icon_color: if is_checked {
                self.get(style, Color::Dark, true)
            } else {
                self.get(style, Color::Light, true)
            },
            border: self.get_border(style, Color::SecondLight, true),
            text_color: None,
        }
    }

    fn hovered(&self, style: &Self::Style, is_checked: bool) -> widget::checkbox::Appearance {
        iced::widget::checkbox::Appearance {
            background: if is_checked {
                self.get_bg(style, Color::White, true)
            } else {
                self.get_bg(style, Color::SecondDark, true)
            },
            icon_color: if is_checked {
                self.get(style, Color::SecondDark, true)
            } else {
                self.get(style, Color::White, true)
            },
            border: self.get_border(style, Color::Light, true),
            text_color: None,
        }
    }
}

impl iced::widget::text_editor::StyleSheet for LauncherTheme {
    type Style = LauncherStyle;

    fn active(&self, style: &Self::Style) -> widget::text_editor::Appearance {
        widget::text_editor::Appearance {
            background: self.get_bg(style, Color::Dark, true),
            border: self.get_border(style, Color::SecondDark, true),
        }
    }

    fn focused(&self, style: &Self::Style) -> widget::text_editor::Appearance {
        widget::text_editor::Appearance {
            background: self.get_bg(style, Color::SecondDark, true),
            border: self.get_border(style, Color::Mid, true),
        }
    }

    fn placeholder_color(&self, style: &Self::Style) -> iced::Color {
        self.get(style, Color::Light, true)
    }

    fn value_color(&self, style: &Self::Style) -> iced::Color {
        self.get(style, Color::White, true)
    }

    fn disabled_color(&self, style: &Self::Style) -> iced::Color {
        self.get(style, Color::Dark, true)
    }

    fn selection_color(&self, style: &Self::Style) -> iced::Color {
        self.get(style, Color::Dark, true)
    }

    fn disabled(&self, style: &Self::Style) -> widget::text_editor::Appearance {
        widget::text_editor::Appearance {
            background: self.get_bg(style, Color::Mid, true),
            border: self.get_border(style, Color::SecondLight, true),
        }
    }
}

impl iced::widget::svg::StyleSheet for LauncherTheme {
    type Style = LauncherStyle;

    fn appearance(&self, _: &Self::Style) -> widget::svg::Appearance {
        widget::svg::Appearance { color: None }
    }
}
