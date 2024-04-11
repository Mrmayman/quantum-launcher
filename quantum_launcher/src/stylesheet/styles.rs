use iced::widget;

use super::color::{Color, DARK_PURPLE};

pub const BORDER_WIDTH: f32 = 2.0;
pub const BORDER_RADIUS: f32 = 8.0;

#[allow(dead_code)]
#[derive(Clone, Default)]
pub enum LauncherTheme {
    Light,
    #[default] // Highly opinionated, I know.
    Dark,
}

impl widget::container::StyleSheet for LauncherTheme {
    type Style = LauncherTheme;

    fn appearance(&self, style: &Self::Style) -> widget::container::Appearance {
        match style {
            LauncherTheme::Light => todo!(),
            LauncherTheme::Dark => widget::container::Appearance {
                text_color: Some(DARK_PURPLE.get(Color::Light)),
                background: Some(iced::Background::Color(DARK_PURPLE.get(Color::Dark))),
                ..Default::default()
            },
        }
    }
}

impl widget::button::StyleSheet for LauncherTheme {
    type Style = LauncherTheme;

    fn active(&self, style: &Self::Style) -> widget::button::Appearance {
        match style {
            LauncherTheme::Light => todo!(),
            LauncherTheme::Dark => widget::button::Appearance {
                background: Some(iced::Background::Color(DARK_PURPLE.get(Color::SecondDark))),
                text_color: DARK_PURPLE.get(Color::White),
                border: DARK_PURPLE.get_border(Color::SecondDark),
                ..Default::default()
            },
        }
    }

    fn hovered(&self, style: &Self::Style) -> widget::button::Appearance {
        match style {
            LauncherTheme::Light => todo!(),
            LauncherTheme::Dark => widget::button::Appearance {
                background: Some(iced::Background::Color(DARK_PURPLE.get(Color::Mid))),
                text_color: DARK_PURPLE.get(Color::Dark),
                border: DARK_PURPLE.get_border(Color::Mid),
                ..Default::default()
            },
        }
    }

    fn pressed(&self, style: &Self::Style) -> widget::button::Appearance {
        match style {
            LauncherTheme::Light => todo!(),
            LauncherTheme::Dark => widget::button::Appearance {
                background: Some(iced::Background::Color(DARK_PURPLE.get(Color::White))),
                text_color: DARK_PURPLE.get(Color::Dark),
                border: DARK_PURPLE.get_border(Color::White),
                ..Default::default()
            },
        }
    }

    fn disabled(&self, style: &Self::Style) -> widget::button::Appearance {
        match style {
            LauncherTheme::Light => todo!(),
            LauncherTheme::Dark => widget::button::Appearance {
                background: Some(iced::Background::Color(DARK_PURPLE.get(Color::SecondDark))),
                text_color: DARK_PURPLE.get(Color::SecondLight),
                border: DARK_PURPLE.get_border(Color::SecondDark),
                ..Default::default()
            },
        }
    }
}

impl widget::text::StyleSheet for LauncherTheme {
    type Style = LauncherTheme;

    fn appearance(&self, style: Self::Style) -> widget::text::Appearance {
        match style {
            LauncherTheme::Light => todo!(),
            LauncherTheme::Dark => widget::text::Appearance { color: None },
        }
    }
}

impl widget::pick_list::StyleSheet for LauncherTheme {
    type Style = LauncherTheme;

    fn active(
        &self,
        style: &<Self as widget::pick_list::StyleSheet>::Style,
    ) -> widget::pick_list::Appearance {
        match style {
            LauncherTheme::Light => todo!(),
            LauncherTheme::Dark => widget::pick_list::Appearance {
                text_color: DARK_PURPLE.get(Color::Dark),
                placeholder_color: DARK_PURPLE.get(Color::SecondDark),
                handle_color: DARK_PURPLE.get(Color::Dark),
                background: iced::Background::Color(DARK_PURPLE.get(Color::Light)),
                border: DARK_PURPLE.get_border(Color::Mid),
            },
        }
    }

    fn hovered(
        &self,
        style: &<Self as widget::pick_list::StyleSheet>::Style,
    ) -> widget::pick_list::Appearance {
        match style {
            LauncherTheme::Light => todo!(),
            LauncherTheme::Dark => widget::pick_list::Appearance {
                text_color: DARK_PURPLE.get(Color::Dark),
                placeholder_color: DARK_PURPLE.get(Color::SecondDark),
                handle_color: DARK_PURPLE.get(Color::Dark),
                background: DARK_PURPLE.get_bg(Color::SecondLight),
                border: DARK_PURPLE.get_border(Color::SecondLight),
            },
        }
    }
}

impl widget::overlay::menu::StyleSheet for LauncherTheme {
    type Style = LauncherTheme;

    fn appearance(&self, style: &Self::Style) -> iced::overlay::menu::Appearance {
        match style {
            LauncherTheme::Light => todo!(),
            LauncherTheme::Dark => iced::overlay::menu::Appearance {
                text_color: DARK_PURPLE.get(Color::White),
                background: DARK_PURPLE.get_bg(Color::SecondDark),
                border: DARK_PURPLE.get_border(Color::Mid),
                selected_text_color: DARK_PURPLE.get(Color::Dark),
                selected_background: DARK_PURPLE.get_bg(Color::SecondLight),
            },
        }
    }
}

impl widget::scrollable::StyleSheet for LauncherTheme {
    type Style = LauncherTheme;

    fn active(&self, style: &Self::Style) -> widget::scrollable::Appearance {
        match style {
            LauncherTheme::Light => todo!(),
            LauncherTheme::Dark => widget::scrollable::Appearance {
                container: widget::container::Appearance {
                    text_color: None,
                    background: None,
                    border: DARK_PURPLE.get_border(Color::SecondDark),
                    shadow: Default::default(),
                },
                scrollbar: widget::scrollable::Scrollbar {
                    background: Some(DARK_PURPLE.get_bg(Color::Dark)),
                    border: DARK_PURPLE.get_border(Color::SecondDark),
                    scroller: widget::scrollable::Scroller {
                        color: DARK_PURPLE.get(Color::White),
                        border: DARK_PURPLE.get_border(Color::Light),
                    },
                },
                gap: None,
            },
        }
    }

    fn hovered(
        &self,
        style: &Self::Style,
        _is_mouse_over_scrollbar: bool,
    ) -> widget::scrollable::Appearance {
        match style {
            LauncherTheme::Light => todo!(),
            LauncherTheme::Dark => widget::scrollable::Appearance {
                container: widget::container::Appearance {
                    text_color: None,
                    background: None,
                    border: DARK_PURPLE.get_border(Color::Mid),
                    shadow: Default::default(),
                },
                scrollbar: widget::scrollable::Scrollbar {
                    background: Some(DARK_PURPLE.get_bg(Color::Dark)),
                    border: DARK_PURPLE.get_border(Color::SecondDark),
                    scroller: widget::scrollable::Scroller {
                        color: DARK_PURPLE.get(Color::White),
                        border: DARK_PURPLE.get_border(Color::Light),
                    },
                },
                gap: None,
            },
        }
    }
}

impl widget::text_input::StyleSheet for LauncherTheme {
    type Style = LauncherTheme;

    fn active(&self, style: &Self::Style) -> widget::text_input::Appearance {
        match style {
            LauncherTheme::Light => todo!(),
            LauncherTheme::Dark => widget::text_input::Appearance {
                background: DARK_PURPLE.get_bg(Color::SecondDark),
                border: DARK_PURPLE.get_border(Color::Mid),
                icon_color: Default::default(),
            },
        }
    }

    fn focused(&self, style: &Self::Style) -> widget::text_input::Appearance {
        match style {
            LauncherTheme::Light => todo!(),
            LauncherTheme::Dark => widget::text_input::Appearance {
                background: DARK_PURPLE.get_bg(Color::SecondDark),
                border: DARK_PURPLE.get_border(Color::Mid),
                icon_color: Default::default(),
            },
        }
    }

    fn placeholder_color(&self, style: &Self::Style) -> iced::Color {
        match style {
            LauncherTheme::Light => todo!(),
            LauncherTheme::Dark => DARK_PURPLE.get(Color::SecondLight),
        }
    }

    fn value_color(&self, style: &Self::Style) -> iced::Color {
        match style {
            LauncherTheme::Light => todo!(),
            LauncherTheme::Dark => DARK_PURPLE.get(Color::White),
        }
    }

    fn disabled_color(&self, style: &Self::Style) -> iced::Color {
        match style {
            LauncherTheme::Light => todo!(),
            LauncherTheme::Dark => DARK_PURPLE.get(Color::SecondDark),
        }
    }

    fn selection_color(&self, style: &Self::Style) -> iced::Color {
        match style {
            LauncherTheme::Light => todo!(),
            LauncherTheme::Dark => DARK_PURPLE.get(Color::SecondLight),
        }
    }

    fn disabled(&self, style: &Self::Style) -> widget::text_input::Appearance {
        match style {
            LauncherTheme::Light => todo!(),
            LauncherTheme::Dark => widget::text_input::Appearance {
                background: DARK_PURPLE.get_bg(Color::Dark),
                border: DARK_PURPLE.get_border(Color::SecondDark),
                icon_color: Default::default(),
            },
        }
    }
}

impl widget::progress_bar::StyleSheet for LauncherTheme {
    type Style = LauncherTheme;

    fn appearance(&self, style: &Self::Style) -> widget::progress_bar::Appearance {
        match style {
            LauncherTheme::Light => todo!(),
            LauncherTheme::Dark => widget::progress_bar::Appearance {
                background: DARK_PURPLE.get_bg(Color::SecondDark),
                bar: DARK_PURPLE.get_bg(Color::Light),
                border_radius: BORDER_RADIUS.into(),
            },
        }
    }
}

impl widget::slider::StyleSheet for LauncherTheme {
    type Style = LauncherTheme;

    fn active(&self, style: &Self::Style) -> widget::slider::Appearance {
        match style {
            LauncherTheme::Light => todo!(),
            LauncherTheme::Dark => widget::slider::Appearance {
                rail: widget::slider::Rail {
                    colors: (
                        DARK_PURPLE.get(Color::Mid),
                        DARK_PURPLE.get(Color::SecondDark),
                    ),
                    width: 4.0,
                    border_radius: BORDER_RADIUS.into(),
                },
                handle: widget::slider::Handle {
                    shape: widget::slider::HandleShape::Circle { radius: 8.0 },
                    color: DARK_PURPLE.get(Color::SecondLight),
                    border_width: 2.0,
                    border_color: DARK_PURPLE.get(Color::Light),
                },
            },
        }
    }

    fn hovered(&self, style: &Self::Style) -> widget::slider::Appearance {
        match style {
            LauncherTheme::Light => todo!(),
            LauncherTheme::Dark => widget::slider::Appearance {
                rail: widget::slider::Rail {
                    colors: (DARK_PURPLE.get(Color::Light), DARK_PURPLE.get(Color::Mid)),
                    width: 4.0,
                    border_radius: BORDER_RADIUS.into(),
                },
                handle: widget::slider::Handle {
                    shape: widget::slider::HandleShape::Circle { radius: 8.0 },
                    color: DARK_PURPLE.get(Color::SecondLight),
                    border_width: 2.0,
                    border_color: DARK_PURPLE.get(Color::White),
                },
            },
        }
    }

    fn dragging(&self, style: &Self::Style) -> widget::slider::Appearance {
        match style {
            LauncherTheme::Light => todo!(),
            LauncherTheme::Dark => widget::slider::Appearance {
                rail: widget::slider::Rail {
                    colors: (
                        DARK_PURPLE.get(Color::Mid),
                        DARK_PURPLE.get(Color::SecondDark),
                    ),
                    width: 6.0,
                    border_radius: BORDER_RADIUS.into(),
                },
                handle: widget::slider::Handle {
                    shape: widget::slider::HandleShape::Circle { radius: 12.0 },
                    color: DARK_PURPLE.get(Color::White),
                    border_width: 2.0,
                    border_color: DARK_PURPLE.get(Color::White),
                },
            },
        }
    }
}

impl iced::application::StyleSheet for LauncherTheme {
    type Style = LauncherTheme;

    fn appearance(&self, style: &Self::Style) -> iced::application::Appearance {
        match style {
            LauncherTheme::Light => todo!(),
            LauncherTheme::Dark => iced::application::Appearance {
                background_color: DARK_PURPLE.get(Color::Dark),
                text_color: DARK_PURPLE.get(Color::Light),
            },
        }
    }
}
