use iced::widget;

use super::{
    color::Color,
    styles::{LauncherTheme, BORDER_WIDTH},
};

#[derive(Default, Clone, Copy)]
pub enum StyleScrollable {
    #[default]
    Round,
    Flat,
}

#[derive(Default, Clone, Copy)]
#[allow(unused)]
pub enum StyleButton {
    #[default]
    Round,
    Flat,
    FlatDark,
    FlatExtraDark,
}

pub trait IsFlat {
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

impl IsFlat for StyleScrollable {
    fn is_flat(&self) -> bool {
        match self {
            StyleScrollable::Round => false,
            StyleScrollable::Flat => true,
        }
    }
}

impl widget::container::Catalog for LauncherTheme {
    type Class<'a> = widget::container::StyleFn<'a, LauncherTheme>;

    fn default<'a>() -> <Self as widget::container::Catalog>::Class<'a> {
        Box::new(Self::style_container_box)
    }

    fn style(
        &self,
        style: &widget::container::StyleFn<'_, LauncherTheme>,
    ) -> widget::container::Style {
        style(self)
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
impl widget::button::Catalog for LauncherTheme {
    type Class<'a> = StyleButton;

    fn active(&self, style: &Self::Class) -> widget::button::Style {
        let color = match style {
            StyleButton::Round | StyleButton::Flat => Color::SecondDark,
            StyleButton::FlatDark => Color::Dark,
            StyleButton::FlatExtraDark => Color::Black,
        };
        widget::button::Style {
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

    fn hovered(&self, style: &Self::Class) -> widget::button::Style {
        let color = match style {
            StyleButton::Round | StyleButton::Flat => Color::Mid,
            StyleButton::FlatDark => Color::Mid,
            StyleButton::FlatExtraDark => Color::SecondDark,
        };
        widget::button::Style {
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

    fn pressed(&self, style: &Self::Class) -> widget::button::Style {
        widget::button::Style {
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

    fn disabled(&self, style: &Self::Class) -> widget::button::Style {
        widget::button::Style {
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

impl widget::button::Catalog for LauncherTheme {
    type Class<'a> = widget::button::StyleFn<'a, LauncherTheme>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(|n, status| n.style_button(status, StyleButton::default()))
    }

    fn style(
        &self,
        style: &widget::button::StyleFn<'_, LauncherTheme>,
        status: widget::button::Status,
    ) -> widget::button::Style {
        style(self, status)
    }
}

impl widget::text::Catalog for LauncherTheme {
    type Class<'a> = ();

    fn default<'a>() -> Self::Class<'a> {}

    fn style(&self, _: &()) -> widget::text::Style {
        widget::text::Style { color: None }
    }
}

impl widget::pick_list::Catalog for LauncherTheme {
    type Class<'a> = ();

    fn default<'a>() -> <Self as widget::pick_list::Catalog>::Class<'a> {}

    fn style(&self, (): &(), status: iced::widget::pick_list::Status) -> widget::pick_list::Style {
        match status {
            widget::pick_list::Status::Active => widget::pick_list::Style {
                text_color: self.get(Color::Dark, false),
                placeholder_color: self.get(Color::SecondDark, false),
                handle_color: self.get(Color::Dark, false),
                background: iced::Background::Color(self.get(Color::Light, false)),
                border: self.get_border(Color::Mid, false),
            },
            widget::pick_list::Status::Hovered => widget::pick_list::Style {
                text_color: self.get(Color::Dark, false),
                placeholder_color: self.get(Color::SecondDark, false),
                handle_color: self.get(Color::Dark, false),
                background: self.get_bg(Color::SecondLight, false),
                border: self.get_border(Color::SecondLight, false),
            },
            widget::pick_list::Status::Opened => widget::pick_list::Style {
                text_color: self.get(Color::Dark, false),
                placeholder_color: self.get(Color::SecondDark, false),
                handle_color: self.get(Color::Dark, false),
                background: self.get_bg(Color::Light, false),
                border: self.get_border(Color::SecondLight, false),
            },
        }
    }
}

impl widget::overlay::menu::Catalog for LauncherTheme {
    type Class<'a> = ();

    fn default<'a>() -> <Self as widget::overlay::menu::Catalog>::Class<'a> {}

    fn style(&self, (): &()) -> iced::overlay::menu::Style {
        iced::overlay::menu::Style {
            text_color: self.get(Color::White, true),
            background: self.get_bg(Color::SecondDark, true),
            border: self.get_border(Color::Mid, true),
            selected_text_color: self.get(Color::Dark, true),
            selected_background: self.get_bg(Color::SecondLight, true),
        }
    }
}

impl widget::scrollable::Catalog for LauncherTheme {
    type Class<'a> = widget::scrollable::StyleFn<'a, LauncherTheme>;

    fn default<'a>() -> <Self as widget::scrollable::Catalog>::Class<'a> {
        Box::new(Self::style_scrollable_round)
    }

    fn style(
        &self,
        style: &widget::scrollable::StyleFn<'_, LauncherTheme>,
        status: widget::scrollable::Status,
    ) -> widget::scrollable::Style {
        style(self, status)
    }
}

impl widget::text_input::Catalog for LauncherTheme {
    type Class<'a> = ();

    fn default<'a>() -> <Self as widget::text_input::Catalog>::Class<'a> {}

    fn style(&self, (): &(), status: widget::text_input::Status) -> widget::text_input::Style {
        match status {
            widget::text_input::Status::Active => widget::text_input::Style {
                background: self.get_bg(Color::Dark, true),
                border: self.get_border(Color::Mid, true),
                icon: self.get(Color::Light, true),
                placeholder: self.get(Color::SecondLight, true),
                value: self.get(Color::White, true),
                selection: self.get(Color::Light, true),
            },
            widget::text_input::Status::Hovered => widget::text_input::Style {
                background: self.get_bg(Color::SecondDark, true),
                border: self.get_border(Color::Mid, true),
                icon: self.get(Color::Light, true),
                placeholder: self.get(Color::SecondLight, true),
                value: self.get(Color::White, true),
                selection: self.get(Color::Light, true),
            },
            widget::text_input::Status::Focused => widget::text_input::Style {
                background: self.get_bg(Color::SecondDark, true),
                border: self.get_border(Color::Light, true),
                icon: self.get(Color::Light, true),
                placeholder: self.get(Color::SecondLight, true),
                value: self.get(Color::White, true),
                selection: self.get(Color::Light, true),
            },
            widget::text_input::Status::Disabled => widget::text_input::Style {
                background: self.get_bg(Color::Black, true),
                border: self.get_border(Color::Dark, true),
                icon: self.get(Color::Light, true),
                placeholder: self.get(Color::SecondLight, true),
                value: self.get(Color::White, true),
                selection: self.get(Color::Light, true),
            },
        }
    }
}

impl widget::progress_bar::Catalog for LauncherTheme {
    type Class<'a> = ();

    fn default<'a>() -> <Self as widget::progress_bar::Catalog>::Class<'a> {}

    fn style(&self, (): &()) -> widget::progress_bar::Style {
        widget::progress_bar::Style {
            background: self.get_bg(Color::SecondDark, true),
            bar: self.get_bg(Color::Light, true),
            border: self.get_border(Color::Mid, true),
        }
    }
}

impl widget::slider::Catalog for LauncherTheme {
    type Class<'a> = ();

    fn default<'a>() -> <Self as widget::slider::Catalog>::Class<'a> {}

    fn style(&self, (): &(), status: widget::slider::Status) -> widget::slider::Style {
        match status {
            widget::slider::Status::Active => widget::slider::Style {
                rail: widget::slider::Rail {
                    backgrounds: (
                        self.get_bg(Color::Mid, true),
                        self.get_bg(Color::SecondDark, true),
                    ),
                    width: 4.0,
                    border: self.get_border(Color::SecondDark, true),
                },
                handle: widget::slider::Handle {
                    shape: widget::slider::HandleShape::Circle { radius: 8.0 },
                    background: self.get_bg(Color::SecondLight, true),
                    border_width: 2.0,
                    border_color: self.get(Color::Light, true),
                },
            },
            widget::slider::Status::Hovered => widget::slider::Style {
                rail: widget::slider::Rail {
                    backgrounds: (
                        self.get_bg(Color::Light, true),
                        self.get_bg(Color::Mid, true),
                    ),
                    width: 4.0,
                    border: self.get_border(Color::Mid, true),
                },
                handle: widget::slider::Handle {
                    shape: widget::slider::HandleShape::Circle { radius: 8.0 },
                    background: self.get_bg(Color::SecondLight, true),
                    border_width: 2.0,
                    border_color: self.get(Color::White, true),
                },
            },
            widget::slider::Status::Dragged => widget::slider::Style {
                rail: widget::slider::Rail {
                    backgrounds: (
                        self.get_bg(Color::White, true),
                        self.get_bg(Color::SecondDark, true),
                    ),
                    width: 6.0,
                    border: self.get_border(Color::Mid, true),
                },
                handle: widget::slider::Handle {
                    shape: widget::slider::HandleShape::Circle { radius: 12.0 },
                    background: self.get_bg(Color::White, true),
                    border_width: 2.0,
                    border_color: self.get(Color::White, true),
                },
            },
        }
    }
}

impl iced::application::DefaultStyle for LauncherTheme {
    fn default_style(&self) -> iced::application::Appearance {
        iced::application::Appearance {
            background_color: self.get(Color::Black, true),
            text_color: self.get(Color::Light, true),
        }
    }
}

impl widget::checkbox::Catalog for LauncherTheme {
    type Class<'a> = ();

    fn default<'a>() -> <Self as widget::checkbox::Catalog>::Class<'a> {}

    fn style(&self, (): &(), status: widget::checkbox::Status) -> widget::checkbox::Style {
        match status {
            widget::checkbox::Status::Active { is_checked } => widget::checkbox::Style {
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
            },
            widget::checkbox::Status::Hovered { is_checked } => widget::checkbox::Style {
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
            },
            widget::checkbox::Status::Disabled { is_checked } => widget::checkbox::Style {
                background: if is_checked {
                    self.get_bg(Color::SecondLight, true)
                } else {
                    self.get_bg(Color::Black, true)
                },
                icon_color: if is_checked {
                    self.get(Color::Black, true)
                } else {
                    self.get(Color::SecondLight, true)
                },
                border: self.get_border(Color::Mid, true),
                text_color: None,
            },
        }
    }
}

impl widget::text_editor::Catalog for LauncherTheme {
    type Class<'a> = ();

    fn default<'a>() -> <Self as widget::text_editor::Catalog>::Class<'a> {}

    fn style(&self, (): &(), status: widget::text_editor::Status) -> widget::text_editor::Style {
        match status {
            widget::text_editor::Status::Active => widget::text_editor::Style {
                background: self.get_bg(Color::Black, true),
                border: self.get_border(Color::SecondDark, true),
                icon: self.get(Color::Light, true),
                placeholder: self.get(Color::Light, true),
                value: self.get(Color::White, true),
                selection: self.get(Color::Dark, true),
            },
            widget::text_editor::Status::Hovered => widget::text_editor::Style {
                background: self.get_bg(Color::Black, true),
                border: self.get_border(Color::Mid, true),
                icon: self.get(Color::Light, true),
                placeholder: self.get(Color::Light, true),
                value: self.get(Color::White, true),
                selection: self.get(Color::Dark, true),
            },
            widget::text_editor::Status::Focused => widget::text_editor::Style {
                background: self.get_bg(Color::Dark, true),
                border: self.get_border(Color::Mid, true),
                icon: self.get(Color::Light, true),
                placeholder: self.get(Color::Light, true),
                value: self.get(Color::White, true),
                selection: self.get(Color::SecondDark, true),
            },
            widget::text_editor::Status::Disabled => widget::text_editor::Style {
                background: self.get_bg(Color::SecondDark, true),
                border: self.get_border(Color::Mid, true),
                icon: self.get(Color::Light, true),
                placeholder: self.get(Color::Light, true),
                value: self.get(Color::White, true),
                selection: self.get(Color::Dark, true),
            },
        }
    }
}

impl widget::svg::Catalog for LauncherTheme {
    type Class<'a> = ();

    fn default<'a>() -> <Self as widget::svg::Catalog>::Class<'a> {}

    fn style(&self, (): &(), _: widget::svg::Status) -> widget::svg::Style {
        // Who hovers on an svg image huh?
        widget::svg::Style { color: None }
    }
}

impl widget::radio::Catalog for LauncherTheme {
    type Class<'a> = ();

    fn default<'a>() -> <Self as widget::radio::Catalog>::Class<'a> {}

    fn style(&self, (): &(), status: widget::radio::Status) -> widget::radio::Style {
        match status {
            widget::radio::Status::Active { is_selected } => widget::radio::Style {
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
            },
            widget::radio::Status::Hovered { is_selected } => widget::radio::Style {
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
            },
        }
    }
}

impl widget::rule::Catalog for LauncherTheme {
    type Class<'a> = widget::rule::StyleFn<'a, LauncherTheme>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(LauncherTheme::style_rule_default)
    }

    fn style(&self, style: &widget::rule::StyleFn<'_, LauncherTheme>) -> widget::rule::Style {
        style(self)
    }
}

impl widget::combo_box::Catalog for LauncherTheme {}
