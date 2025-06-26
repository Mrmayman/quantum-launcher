use super::styles::{BORDER_RADIUS, BORDER_WIDTH};

pub struct Pallete {
    extra_dark: [u8; 3],
    dark: [u8; 3],
    second_dark: [u8; 3],
    mid: [u8; 3],
    second_light: [u8; 3],
    light: [u8; 3],
    white: [u8; 3],
}

pub const PURPLE: Pallete = Pallete {
    extra_dark: [0x22, 0x19, 0x20],
    dark: [0x3a, 0x24, 0x36],
    second_dark: [0x66, 0x47, 0x69],
    mid: [0xcc, 0x76, 0xc5],
    second_light: [0xf9, 0xb1, 0xe6],
    light: [0xff, 0xc7, 0xf0],
    white: [0xff, 0xda, 0xf5],
};

pub const BROWN: Pallete = Pallete {
    extra_dark: [0x00, 0x00, 0x00],
    dark: [0x3d, 0x21, 0x1a],
    second_dark: [0x6f, 0x4d, 0x38],
    mid: [0xa0, 0x78, 0x56],
    second_light: [0xcb, 0xb7, 0x99],
    light: [0xf0, 0xf0, 0xcf],
    white: [0xff, 0xff, 0xff],
};

pub const SKY_BLUE: Pallete = Pallete {
    extra_dark: [0x1a, 0x1b, 0x26],
    dark: [0x1a, 0x2f, 0x41],
    second_dark: [0x0f, 0x51, 0x73],
    mid: [0x48, 0x85, 0xa4],
    second_light: [0xa3, 0xd3, 0xfa],
    light: [0xe6, 0xf2, 0xff],
    white: [0xf5, 0xf9, 0xfe],
};

pub const CATPPUCCIN: Pallete = Pallete {
    extra_dark: [0x11, 0x11, 0x1b],
    dark: [0x1e, 0x1e, 0x2e],
    second_dark: [0x57, 0x56, 0x67],
    mid: [0x76, 0x75, 0x88],
    second_light: [0xf2, 0xcd, 0xcd],
    light: [0xfc, 0xe0, 0xda],
    white: [0xf7, 0xea, 0xe6],
};

pub const TEAL: Pallete = Pallete {
    extra_dark: [0x1b, 0x30, 0x30],
    dark: [0x26, 0x43, 0x44],
    second_dark: [0x30, 0x56, 0x57],
    mid: [0x6e, 0xa0, 0x7f],
    second_light: [0xa4, 0xc0, 0x7e],
    light: [0xfa, 0xff, 0x95],
    white: [0xfc, 0xff, 0xc8],
};

#[derive(Clone, Copy)]
pub enum Color {
    ExtraDark,
    Dark,
    SecondDark,
    Light,
    SecondLight,
    Mid,
    White,
}

impl Color {
    pub fn invert(self) -> Color {
        match self {
            Color::ExtraDark => Color::Light,
            Color::Dark => Color::White,
            Color::SecondDark => Color::SecondLight,
            Color::Light => Color::Dark,
            Color::SecondLight => Color::SecondDark,
            Color::Mid => Color::Mid,
            Color::White => Color::ExtraDark,
        }
    }
}

pub trait IntoIced {
    fn into_color(self) -> iced::Color;
}
impl IntoIced for [u8; 3] {
    fn into_color(self) -> iced::Color {
        iced::Color::from_rgb8(self[0], self[1], self[2])
    }
}

impl Pallete {
    pub fn get(&self, color: Color) -> iced::Color {
        match color {
            Color::Dark => self.dark.into_color(),
            Color::SecondDark => self.second_dark.into_color(),
            Color::Light => self.light.into_color(),
            Color::SecondLight => self.second_light.into_color(),
            Color::Mid => self.mid.into_color(),
            Color::White => self.white.into_color(),
            Color::ExtraDark => self.extra_dark.into_color(),
        }
    }

    pub fn get_bg(&self, color: Color) -> iced::Background {
        iced::Background::Color(self.get(color))
    }

    pub fn get_border(&self, color: Color) -> iced::Border {
        iced::Border {
            color: self.get(color),
            width: BORDER_WIDTH,
            radius: BORDER_RADIUS.into(),
        }
    }
}
