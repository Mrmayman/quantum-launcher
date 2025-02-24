use iced::widget;

use super::{
    color::{Color, BROWN, PURPLE, SKY_BLUE},
    widgets::{IsFlat, StyleButton, StyleScrollable},
};

pub const BORDER_WIDTH: f32 = 2.0;
pub const BORDER_RADIUS: f32 = 8.0;

#[derive(Clone, Debug, Copy, Default)]
pub enum LauncherThemeColor {
    Brown,
    #[default]
    Purple,
    SkyBlue,
}

#[derive(Clone, Default, Debug)]
pub enum LauncherThemeLightness {
    #[default]
    Dark,
    Light,
}

#[derive(Clone, Default, Debug)]
pub struct LauncherTheme {
    pub lightness: LauncherThemeLightness,
    pub color: LauncherThemeColor,
}

impl LauncherTheme {
    pub fn from_vals(color: LauncherThemeColor, lightness: LauncherThemeLightness) -> Self {
        Self { color, lightness }
    }

    pub fn get(&self, color: Color, invert: bool) -> iced::Color {
        let (palette, color) = self.get_base(invert, color);
        palette.get(color)
    }

    fn get_base(&self, invert: bool, color: Color) -> (&super::color::Pallete, Color) {
        let palette = match self.color {
            LauncherThemeColor::Brown => &BROWN,
            LauncherThemeColor::Purple => &PURPLE,
            LauncherThemeColor::SkyBlue => &SKY_BLUE,
        };
        let color = if invert {
            match self.lightness {
                LauncherThemeLightness::Dark => color,
                LauncherThemeLightness::Light => color.invert(),
            }
        } else {
            match self.lightness {
                LauncherThemeLightness::Dark => color.invert(),
                LauncherThemeLightness::Light => color,
            }
        };
        (palette, color)
    }

    pub fn get_bg(&self, color: Color, invert: bool) -> iced::Background {
        let (palette, color) = self.get_base(invert, color);
        palette.get_bg(color)
    }

    pub fn get_border(&self, color: Color, invert: bool) -> iced::Border {
        let (palette, color) = self.get_base(invert, color);
        palette.get_border(color)
    }

    fn get_border_sharp(&self, color: Color, invert: bool) -> iced::Border {
        let (palette, color) = self.get_base(invert, color);
        iced::Border {
            color: palette.get(color),
            width: 1.0,
            radius: 0.0.into(),
        }
    }

    fn get_border_style(&self, style: &impl IsFlat, color: Color, invert: bool) -> iced::Border {
        if style.is_flat() {
            self.get_border_sharp(color, invert)
        } else {
            self.get_border(color, invert)
        }
    }

    fn style_scrollable_active(&self, style: StyleScrollable) -> widget::scrollable::Style {
        let border = self.get_border_style(
            &style,
            match style {
                StyleScrollable::Round => Color::SecondDark,
                StyleScrollable::Flat => Color::Dark,
            },
            true,
        );
        let rail = widget::scrollable::Rail {
            background: Some(self.get_bg(Color::Dark, true)),
            border,
            scroller: widget::scrollable::Scroller {
                color: self.get(Color::SecondDark, true),
                border: self.get_border_style(&style, Color::Mid, true),
            },
        };
        widget::scrollable::Style {
            container: widget::container::Style {
                text_color: None,
                background: match style {
                    StyleScrollable::Round => None,
                    StyleScrollable::Flat => Some(self.get_bg(Color::Dark, true)),
                },
                border,
                shadow: iced::Shadow::default(),
            },
            gap: None,
            vertical_rail: rail,
            horizontal_rail: rail,
        }
    }

    fn style_scrollable_hovered(
        &self,
        style: StyleScrollable,
        is_vertical_scrollbar_hovered: bool,
        is_horizontal_scrollbar_hovered: bool,
    ) -> widget::scrollable::Style {
        let border = self.get_border_style(
            &style,
            match style {
                StyleScrollable::Round => Color::Mid,
                StyleScrollable::Flat => Color::Dark,
            },
            true,
        );
        let rail_v = widget::scrollable::Rail {
            background: Some(self.get_bg(Color::Dark, true)),
            border,
            scroller: widget::scrollable::Scroller {
                color: self.get(
                    if is_vertical_scrollbar_hovered {
                        Color::Light
                    } else {
                        Color::Mid
                    },
                    true,
                ),
                border: self.get_border_style(&style, Color::Light, true),
            },
        };
        let rail_h = widget::scrollable::Rail {
            background: Some(self.get_bg(Color::Dark, true)),
            border,
            scroller: widget::scrollable::Scroller {
                color: self.get(
                    if is_horizontal_scrollbar_hovered {
                        Color::Light
                    } else {
                        Color::Mid
                    },
                    true,
                ),
                border: self.get_border_style(&style, Color::Light, true),
            },
        };
        widget::scrollable::Style {
            container: widget::container::Style {
                text_color: None,
                background: match style {
                    StyleScrollable::Round => None,
                    StyleScrollable::Flat => Some(self.get_bg(Color::Dark, true)),
                },
                border,
                shadow: iced::Shadow::default(),
            },
            vertical_rail: rail_v,
            horizontal_rail: rail_h,
            gap: None,
        }
    }

    fn style_scrollable_dragged(
        &self,
        style: StyleScrollable,
        is_vertical_scrollbar_dragged: bool,
        is_horizontal_scrollbar_dragged: bool,
    ) -> widget::scrollable::Style {
        let border = self.get_border_style(
            &style,
            match style {
                StyleScrollable::Round => Color::Mid,
                StyleScrollable::Flat => Color::Dark,
            },
            true,
        );
        let rail_v = widget::scrollable::Rail {
            background: Some(self.get_bg(Color::Dark, true)),
            border,
            scroller: widget::scrollable::Scroller {
                color: self.get(
                    if is_vertical_scrollbar_dragged {
                        Color::White
                    } else {
                        Color::Mid
                    },
                    true,
                ),
                border: self.get_border_style(&style, Color::Light, true),
            },
        };
        let rail_h = widget::scrollable::Rail {
            background: Some(self.get_bg(Color::Dark, true)),
            border,
            scroller: widget::scrollable::Scroller {
                color: self.get(
                    if is_horizontal_scrollbar_dragged {
                        Color::White
                    } else {
                        Color::Mid
                    },
                    true,
                ),
                border: self.get_border_style(&style, Color::Light, true),
            },
        };
        widget::scrollable::Style {
            container: widget::container::Style {
                text_color: None,
                background: match style {
                    StyleScrollable::Round => None,
                    StyleScrollable::Flat => Some(self.get_bg(Color::Dark, true)),
                },
                border,
                shadow: iced::Shadow::default(),
            },
            vertical_rail: rail_v,
            horizontal_rail: rail_h,
            gap: None,
        }
    }

    pub fn style_rule(&self, color: Color, thickness: u16) -> widget::rule::Style {
        widget::rule::Style {
            color: self.get(color, true),
            width: thickness,
            radius: 0.into(),
            fill_mode: widget::rule::FillMode::Full,
        }
    }

    pub fn style_container_box(&self) -> widget::container::Style {
        widget::container::Style {
            border: self.get_border(Color::SecondDark, true),
            ..Default::default()
        }
    }

    pub fn style_container_selected_flat_button(&self) -> widget::container::Style {
        widget::container::Style {
            border: self.get_border_sharp(Color::Mid, true),
            background: Some(self.get_bg(Color::Mid, true)),
            text_color: Some(self.get(Color::Dark, true)),
            ..Default::default()
        }
    }

    pub fn style_container_sharp_box(&self, width: f32, color: Color) -> widget::container::Style {
        widget::container::Style {
            border: {
                let (palette, color) = self.get_base(true, Color::Mid);
                iced::Border {
                    color: palette.get(color),
                    width,
                    radius: 0.0.into(),
                }
            },
            background: Some(self.get_bg(color, true)),
            ..Default::default()
        }
    }

    pub fn style_scrollable_round(
        &self,
        status: widget::scrollable::Status,
    ) -> widget::scrollable::Style {
        self.style_scrollable(status, StyleScrollable::Round)
    }

    pub fn style_scrollable_flat(
        &self,
        status: widget::scrollable::Status,
    ) -> widget::scrollable::Style {
        self.style_scrollable(status, StyleScrollable::Flat)
    }

    fn style_scrollable(
        &self,
        status: widget::scrollable::Status,
        style: StyleScrollable,
    ) -> widget::scrollable::Style {
        match status {
            widget::scrollable::Status::Active => self.style_scrollable_active(style),
            widget::scrollable::Status::Hovered {
                is_horizontal_scrollbar_hovered,
                is_vertical_scrollbar_hovered,
            } => self.style_scrollable_hovered(
                style,
                is_vertical_scrollbar_hovered,
                is_horizontal_scrollbar_hovered,
            ),
            widget::scrollable::Status::Dragged {
                is_horizontal_scrollbar_dragged,
                is_vertical_scrollbar_dragged,
            } => self.style_scrollable_dragged(
                style,
                is_vertical_scrollbar_dragged,
                is_horizontal_scrollbar_dragged,
            ),
        }
    }

    pub fn style_rule_default(&self) -> widget::rule::Style {
        self.style_rule(Color::Mid, 2)
    }

    pub fn style_button(
        &self,
        status: widget::button::Status,
        style: StyleButton,
    ) -> widget::button::Style {
        match status {
            widget::button::Status::Active => {
                let color = match style {
                    StyleButton::Round | StyleButton::Flat => Color::SecondDark,
                    StyleButton::FlatDark => Color::Dark,
                    StyleButton::FlatExtraDark => Color::Black,
                };
                widget::button::Style {
                    background: Some(self.get_bg(color, true)),
                    text_color: self.get(Color::White, true),
                    border: self.get_border_style(&style, color, true),
                    ..Default::default()
                }
            }
            widget::button::Status::Hovered => {
                let color = match style {
                    StyleButton::Round | StyleButton::Flat | StyleButton::FlatDark => Color::Mid,
                    StyleButton::FlatExtraDark => Color::SecondDark,
                };
                widget::button::Style {
                    background: Some(self.get_bg(color, true)),
                    text_color: self.get(
                        match style {
                            StyleButton::Round | StyleButton::Flat => Color::Dark,
                            StyleButton::FlatDark | StyleButton::FlatExtraDark => Color::White,
                        },
                        true,
                    ),
                    border: self.get_border_style(&style, color, true),
                    ..Default::default()
                }
            }
            widget::button::Status::Pressed => widget::button::Style {
                background: Some(self.get_bg(Color::White, true)),
                text_color: self.get(Color::Dark, true),
                border: self.get_border_style(&style, Color::White, true),
                ..Default::default()
            },
            widget::button::Status::Disabled => widget::button::Style {
                background: Some(self.get_bg(
                    match style {
                        StyleButton::Round | StyleButton::Flat => Color::SecondDark,
                        StyleButton::FlatDark => Color::Dark,
                        StyleButton::FlatExtraDark => Color::Black,
                    },
                    true,
                )),
                text_color: self.get(Color::SecondLight, true),
                border: self.get_border_style(&style, Color::SecondDark, true),
                ..Default::default()
            },
        }
    }
}
