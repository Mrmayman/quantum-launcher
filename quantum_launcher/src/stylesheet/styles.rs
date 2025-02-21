use iced::widget;

use super::color::{Color, BROWN, PURPLE, SKY_BLUE};

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

    fn get(&self, color: Color, invert: bool) -> iced::Color {
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

    fn get_bg(&self, color: Color, invert: bool) -> iced::Background {
        let (palette, color) = self.get_base(invert, color);
        palette.get_bg(color)
    }

    fn get_border(&self, color: Color, invert: bool) -> iced::Border {
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
}

#[derive(Default)]
pub enum StyleFlatness {
    #[default]
    Round,
    Flat,
}

#[derive(Default)]
#[allow(unused)]
pub enum StyleButton {
    #[default]
    Round,
    Flat,
    FlatDark,
    FlatExtraDark,
}

#[derive(Default)]
pub enum StyleContainer {
    #[default]
    Box,
    SelectedFlatButton,
    SharpBox(Color, f32),
}

trait IsFlat {
    fn is_flat(&self) -> bool;
}

impl IsFlat for StyleButton {
    fn is_flat(&self) -> bool {
        match self {
            StyleButton::Round => false,
            StyleButton::Flat | StyleButton::FlatDark | StyleButton::FlatExtraDark => true,
        }
    }
}

impl IsFlat for StyleFlatness {
    fn is_flat(&self) -> bool {
        match self {
            StyleFlatness::Round => false,
            StyleFlatness::Flat => true,
        }
    }
}

impl widget::container::StyleSheet for LauncherTheme {
    type Style = StyleContainer;

    fn appearance(&self, style: &Self::Style) -> widget::container::Appearance {
        match style {
            StyleContainer::Box => widget::container::Appearance {
                border: self.get_border(Color::SecondDark, true),
                ..Default::default()
            },
            StyleContainer::SelectedFlatButton => widget::container::Appearance {
                border: self.get_border_sharp(Color::Mid, true),
                background: Some(self.get_bg(Color::Mid, true)),
                text_color: Some(self.get(Color::Dark, true)),
                ..Default::default()
            },
            StyleContainer::SharpBox(color, width) => widget::container::Appearance {
                border: {
                    let (palette, color) = self.get_base(true, Color::Mid);
                    iced::Border {
                        color: palette.get(color),
                        width: *width,
                        radius: 0.0.into(),
                    }
                },
                background: Some(self.get_bg(*color, true)),
                ..Default::default()
            },
        }
    }
}

// Uncomment this and comment the other impl below this
// to have a gradient skeumorphic look for the buttons
//
// I disabled this because even though it looks decent
// it doesnt fit with the rest of the launcher, and
// all the other widgets look bad with this skeumorphic
// aesthetic.
/*
impl widget::button::StyleSheet for LauncherTheme {
    type Style = StyleButton;

    fn active(&self, style: &Self::Style) -> widget::button::Appearance {
        let color = match style {
            StyleButton::Round | StyleButton::Flat => Color::SecondDark,
            StyleButton::FlatDark => Color::Dark,
            StyleButton::FlatExtraDark => Color::Black,
        };
        widget::button::Appearance {
            background: Some(if let StyleButton::Round = style {
                iced::Background::Gradient(iced::Gradient::Linear(
                    iced::gradient::Linear::new(0.0)
                        .add_stop(0.0, self.get(Color::SecondDark, true))
                        .add_stop(1.0, self.get(Color::Mid, true)),
                ))
            } else {
                self.get_bg(color, true)
            }),
            text_color: self.get(Color::White, true),
            border: self.get_border_style(style, color, true),
            ..Default::default()
        }
    }

    fn hovered(&self, style: &Self::Style) -> widget::button::Appearance {
        let color = match style {
            StyleButton::Round | StyleButton::Flat => Color::Mid,
            StyleButton::FlatDark => Color::Mid,
            StyleButton::FlatExtraDark => Color::SecondDark,
        };
        widget::button::Appearance {
            background: Some(if let StyleButton::Round = style {
                iced::Background::Gradient(iced::Gradient::Linear(
                    iced::gradient::Linear::new(0.0)
                        .add_stop(0.0, self.get(Color::Mid, true))
                        .add_stop(1.0, self.get(Color::SecondLight, true)),
                ))
            } else {
                self.get_bg(color, true)
            }),
            text_color: self.get(
                match style {
                    StyleButton::Round | StyleButton::Flat => Color::Dark,
                    StyleButton::FlatDark | StyleButton::FlatExtraDark => Color::White,
                },
                true,
            ),
            border: self.get_border_style(style, color, true),
            ..Default::default()
        }
    }

    fn pressed(&self, style: &Self::Style) -> widget::button::Appearance {
        widget::button::Appearance {
            background: Some(if let StyleButton::Round = style {
                iced::Background::Gradient(iced::Gradient::Linear(
                    iced::gradient::Linear::new(0.0)
                        .add_stop(0.0, self.get(Color::SecondLight, true))
                        .add_stop(1.0, self.get(Color::Mid, true)),
                ))
            } else {
                self.get_bg(Color::White, true)
            }),
            text_color: self.get(Color::Dark, true),
            border: self.get_border_style(style, Color::White, true),
            ..Default::default()
        }
    }

    fn disabled(&self, style: &Self::Style) -> widget::button::Appearance {
        widget::button::Appearance {
            background: Some(self.get_bg(
                match style {
                    StyleButton::Round | StyleButton::Flat => Color::SecondDark,
                    StyleButton::FlatDark => Color::Dark,
                    StyleButton::FlatExtraDark => Color::Black,
                },
                true,
            )),
            text_color: self.get(Color::SecondLight, true),
            border: self.get_border_style(style, Color::SecondDark, true),
            ..Default::default()
        }
    }
}
*/

impl widget::button::StyleSheet for LauncherTheme {
    type Style = StyleButton;

    fn active(&self, style: &Self::Style) -> widget::button::Appearance {
        let color = match style {
            StyleButton::Round | StyleButton::Flat => Color::SecondDark,
            StyleButton::FlatDark => Color::Dark,
            StyleButton::FlatExtraDark => Color::Black,
        };
        widget::button::Appearance {
            background: Some(self.get_bg(color, true)),
            text_color: self.get(Color::White, true),
            border: self.get_border_style(style, color, true),
            ..Default::default()
        }
    }

    fn hovered(&self, style: &Self::Style) -> widget::button::Appearance {
        let color = match style {
            StyleButton::Round | StyleButton::Flat => Color::Mid,
            StyleButton::FlatDark => Color::Mid,
            StyleButton::FlatExtraDark => Color::SecondDark,
        };
        widget::button::Appearance {
            background: Some(self.get_bg(color, true)),
            text_color: self.get(
                match style {
                    StyleButton::Round | StyleButton::Flat => Color::Dark,
                    StyleButton::FlatDark | StyleButton::FlatExtraDark => Color::White,
                },
                true,
            ),
            border: self.get_border_style(style, color, true),
            ..Default::default()
        }
    }

    fn pressed(&self, style: &Self::Style) -> widget::button::Appearance {
        widget::button::Appearance {
            background: Some(self.get_bg(Color::White, true)),
            text_color: self.get(Color::Dark, true),
            border: self.get_border_style(style, Color::White, true),
            ..Default::default()
        }
    }

    fn disabled(&self, style: &Self::Style) -> widget::button::Appearance {
        widget::button::Appearance {
            background: Some(self.get_bg(
                match style {
                    StyleButton::Round | StyleButton::Flat => Color::SecondDark,
                    StyleButton::FlatDark => Color::Dark,
                    StyleButton::FlatExtraDark => Color::Black,
                },
                true,
            )),
            text_color: self.get(Color::SecondLight, true),
            border: self.get_border_style(style, Color::SecondDark, true),
            ..Default::default()
        }
    }
}

impl widget::text::StyleSheet for LauncherTheme {
    type Style = ();

    fn appearance(&self, _style: Self::Style) -> widget::text::Appearance {
        widget::text::Appearance { color: None }
    }
}

impl widget::pick_list::StyleSheet for LauncherTheme {
    type Style = ();

    fn active(&self, (): &Self::Style) -> widget::pick_list::Appearance {
        widget::pick_list::Appearance {
            text_color: self.get(Color::Dark, false),
            placeholder_color: self.get(Color::SecondDark, false),
            handle_color: self.get(Color::Dark, false),
            background: iced::Background::Color(self.get(Color::Light, false)),
            border: self.get_border(Color::Mid, false),
        }
    }

    fn hovered(&self, (): &Self::Style) -> widget::pick_list::Appearance {
        widget::pick_list::Appearance {
            text_color: self.get(Color::Dark, false),
            placeholder_color: self.get(Color::SecondDark, false),
            handle_color: self.get(Color::Dark, false),
            background: self.get_bg(Color::SecondLight, false),
            border: self.get_border(Color::SecondLight, false),
        }
    }
}

impl widget::overlay::menu::StyleSheet for LauncherTheme {
    type Style = ();

    fn appearance(&self, (): &Self::Style) -> iced::overlay::menu::Appearance {
        iced::overlay::menu::Appearance {
            text_color: self.get(Color::White, true),
            background: self.get_bg(Color::SecondDark, true),
            border: self.get_border(Color::Mid, true),
            selected_text_color: self.get(Color::Dark, true),
            selected_background: self.get_bg(Color::SecondLight, true),
        }
    }
}

impl widget::scrollable::StyleSheet for LauncherTheme {
    type Style = StyleFlatness;

    fn active(&self, style: &Self::Style) -> widget::scrollable::Appearance {
        let border = self.get_border_style(
            style,
            match style {
                StyleFlatness::Round => Color::SecondDark,
                StyleFlatness::Flat => Color::Dark,
            },
            true,
        );
        widget::scrollable::Appearance {
            container: widget::container::Appearance {
                text_color: None,
                background: match style {
                    StyleFlatness::Round => None,
                    StyleFlatness::Flat => Some(self.get_bg(Color::Dark, true)),
                },
                border,
                shadow: iced::Shadow::default(),
            },
            scrollbar: widget::scrollable::Scrollbar {
                background: Some(self.get_bg(Color::Dark, true)),
                border,
                scroller: widget::scrollable::Scroller {
                    color: self.get(Color::SecondDark, true),
                    border: self.get_border_style(style, Color::Mid, true),
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
        let border = self.get_border_style(
            style,
            match style {
                StyleFlatness::Round => Color::Mid,
                StyleFlatness::Flat => Color::Dark,
            },
            true,
        );
        widget::scrollable::Appearance {
            container: widget::container::Appearance {
                text_color: None,
                background: match style {
                    StyleFlatness::Round => None,
                    StyleFlatness::Flat => Some(self.get_bg(Color::Dark, true)),
                },
                border,
                shadow: iced::Shadow::default(),
            },
            scrollbar: widget::scrollable::Scrollbar {
                background: Some(self.get_bg(Color::Dark, true)),
                border,
                scroller: widget::scrollable::Scroller {
                    color: self.get(Color::White, true),
                    border: self.get_border_style(style, Color::Light, true),
                },
            },
            gap: None,
        }
    }
}

impl widget::text_input::StyleSheet for LauncherTheme {
    type Style = ();

    fn active(&self, (): &Self::Style) -> widget::text_input::Appearance {
        widget::text_input::Appearance {
            background: self.get_bg(Color::SecondDark, true),
            border: self.get_border(Color::Mid, true),
            icon_color: iced::Color::default(),
        }
    }

    fn focused(&self, (): &Self::Style) -> widget::text_input::Appearance {
        widget::text_input::Appearance {
            background: self.get_bg(Color::SecondDark, true),
            border: self.get_border(Color::Mid, true),
            icon_color: iced::Color::default(),
        }
    }

    fn placeholder_color(&self, (): &Self::Style) -> iced::Color {
        self.get(Color::SecondLight, true)
    }

    fn value_color(&self, (): &Self::Style) -> iced::Color {
        self.get(Color::White, true)
    }

    fn disabled_color(&self, (): &Self::Style) -> iced::Color {
        self.get(Color::SecondDark, true)
    }

    fn selection_color(&self, (): &Self::Style) -> iced::Color {
        self.get(Color::SecondLight, true)
    }

    fn disabled(&self, (): &Self::Style) -> widget::text_input::Appearance {
        widget::text_input::Appearance {
            background: self.get_bg(Color::Dark, true),
            border: self.get_border(Color::SecondDark, true),
            icon_color: iced::Color::default(),
        }
    }
}

impl widget::progress_bar::StyleSheet for LauncherTheme {
    type Style = ();

    fn appearance(&self, (): &Self::Style) -> widget::progress_bar::Appearance {
        widget::progress_bar::Appearance {
            background: self.get_bg(Color::SecondDark, true),
            bar: self.get_bg(Color::Light, true),
            border_radius: BORDER_RADIUS.into(),
        }
    }
}

impl widget::slider::StyleSheet for LauncherTheme {
    type Style = ();

    fn active(&self, (): &Self::Style) -> widget::slider::Appearance {
        widget::slider::Appearance {
            rail: widget::slider::Rail {
                colors: (
                    self.get(Color::Mid, true),
                    self.get(Color::SecondDark, true),
                ),
                width: 4.0,
                border_radius: BORDER_RADIUS.into(),
            },
            handle: widget::slider::Handle {
                shape: widget::slider::HandleShape::Circle { radius: 8.0 },
                color: self.get(Color::SecondLight, true),
                border_width: 2.0,
                border_color: self.get(Color::Light, true),
            },
        }
    }

    fn hovered(&self, (): &Self::Style) -> widget::slider::Appearance {
        widget::slider::Appearance {
            rail: widget::slider::Rail {
                colors: (self.get(Color::Light, true), self.get(Color::Mid, true)),
                width: 4.0,
                border_radius: BORDER_RADIUS.into(),
            },
            handle: widget::slider::Handle {
                shape: widget::slider::HandleShape::Circle { radius: 8.0 },
                color: self.get(Color::SecondLight, true),
                border_width: 2.0,
                border_color: self.get(Color::White, true),
            },
        }
    }

    fn dragging(&self, (): &Self::Style) -> widget::slider::Appearance {
        widget::slider::Appearance {
            rail: widget::slider::Rail {
                colors: (
                    self.get(Color::Mid, true),
                    self.get(Color::SecondDark, true),
                ),
                width: 6.0,
                border_radius: BORDER_RADIUS.into(),
            },
            handle: widget::slider::Handle {
                shape: widget::slider::HandleShape::Circle { radius: 12.0 },
                color: self.get(Color::White, true),
                border_width: 2.0,
                border_color: self.get(Color::White, true),
            },
        }
    }
}

impl iced::application::StyleSheet for LauncherTheme {
    type Style = ();

    fn appearance(&self, (): &Self::Style) -> iced::application::Appearance {
        iced::application::Appearance {
            background_color: self.get(Color::Black, true),
            text_color: self.get(Color::Light, true),
        }
    }
}

impl widget::checkbox::StyleSheet for LauncherTheme {
    type Style = ();

    fn active(&self, (): &Self::Style, is_checked: bool) -> widget::checkbox::Appearance {
        widget::checkbox::Appearance {
            background: if is_checked {
                self.get_bg(Color::Light, true)
            } else {
                self.get_bg(Color::Dark, true)
            },
            icon_color: if is_checked {
                self.get(Color::Dark, true)
            } else {
                self.get(Color::Light, true)
            },
            border: self.get_border(Color::SecondLight, true),
            text_color: None,
        }
    }

    fn hovered(&self, (): &Self::Style, is_checked: bool) -> widget::checkbox::Appearance {
        widget::checkbox::Appearance {
            background: if is_checked {
                self.get_bg(Color::White, true)
            } else {
                self.get_bg(Color::SecondDark, true)
            },
            icon_color: if is_checked {
                self.get(Color::SecondDark, true)
            } else {
                self.get(Color::White, true)
            },
            border: self.get_border(Color::Light, true),
            text_color: None,
        }
    }
}

impl widget::text_editor::StyleSheet for LauncherTheme {
    type Style = ();

    fn active(&self, (): &Self::Style) -> widget::text_editor::Appearance {
        widget::text_editor::Appearance {
            background: self.get_bg(Color::Dark, true),
            border: self.get_border(Color::SecondDark, true),
        }
    }

    fn focused(&self, (): &Self::Style) -> widget::text_editor::Appearance {
        widget::text_editor::Appearance {
            background: self.get_bg(Color::SecondDark, true),
            border: self.get_border(Color::Mid, true),
        }
    }

    fn placeholder_color(&self, (): &Self::Style) -> iced::Color {
        self.get(Color::Light, true)
    }

    fn value_color(&self, (): &Self::Style) -> iced::Color {
        self.get(Color::White, true)
    }

    fn disabled_color(&self, (): &Self::Style) -> iced::Color {
        self.get(Color::Dark, true)
    }

    fn selection_color(&self, (): &Self::Style) -> iced::Color {
        self.get(Color::Dark, true)
    }

    fn disabled(&self, (): &Self::Style) -> widget::text_editor::Appearance {
        widget::text_editor::Appearance {
            background: self.get_bg(Color::Mid, true),
            border: self.get_border(Color::SecondLight, true),
        }
    }
}

impl widget::svg::StyleSheet for LauncherTheme {
    type Style = ();

    fn appearance(&self, _: &Self::Style) -> widget::svg::Appearance {
        widget::svg::Appearance { color: None }
    }
}

impl widget::radio::StyleSheet for LauncherTheme {
    type Style = ();

    fn active(&self, (): &Self::Style, is_selected: bool) -> widget::radio::Appearance {
        widget::radio::Appearance {
            background: self.get_bg(Color::Dark, true),
            dot_color: self.get(
                if is_selected {
                    Color::Light
                } else {
                    Color::Dark
                },
                true,
            ),
            border_width: BORDER_WIDTH,
            border_color: self.get(Color::SecondLight, true),
            text_color: None,
        }
    }

    fn hovered(&self, (): &Self::Style, is_selected: bool) -> widget::radio::Appearance {
        widget::radio::Appearance {
            background: self.get_bg(Color::Dark, true),
            dot_color: self.get(
                if is_selected {
                    Color::White
                } else {
                    Color::SecondDark
                },
                true,
            ),
            border_width: BORDER_WIDTH,
            border_color: self.get(Color::SecondLight, true),
            text_color: None,
        }
    }
}

pub struct StyleRule {
    pub thickness: u16,
    pub color: Color,
}

impl Default for StyleRule {
    fn default() -> Self {
        Self {
            thickness: 2,
            color: Color::Mid,
        }
    }
}

impl widget::rule::StyleSheet for LauncherTheme {
    type Style = StyleRule;

    fn appearance(&self, style: &Self::Style) -> widget::rule::Appearance {
        widget::rule::Appearance {
            color: self.get(style.color, true),
            width: style.thickness,
            radius: 0.into(),
            fill_mode: widget::rule::FillMode::Full,
        }
    }
}
