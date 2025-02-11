use super::styles::{BORDER_RADIUS, BORDER_WIDTH};

pub struct Pallete {
    black: [u8; 3],
    dark: [u8; 3],
    second_dark: [u8; 3],
    mid: [u8; 3],
    second_light: [u8; 3],
    light: [u8; 3],
    white: [u8; 3],
}

pub const PURPLE: Pallete = Pallete {
    black: [0x22, 0x19, 0x20],
    dark: [0x3a, 0x24, 0x36],
    second_dark: [0x66, 0x47, 0x69],
    mid: [0xcc, 0x76, 0xc5],
    light: [0xeb, 0xad, 0xda],
    second_light: [0xe6, 0xe4, 0xce],
    white: [0xff, 0xff, 0xff],
};

pub const BROWN: Pallete = Pallete {
    black: [0x00, 0x00, 0x00],
    dark: [0x3d, 0x21, 0x1a],
    second_dark: [0x6f, 0x4d, 0x38],
    mid: [0xa0, 0x78, 0x56],
    second_light: [0xcb, 0xb7, 0x99],
    light: [0xf0, 0xf0, 0xcf],
    white: [0xff, 0xff, 0xff],
};

pub const SKY_BLUE: Pallete = Pallete {
    black: [0x0, 0x0, 0x0],
    dark: [0x1f, 0x32, 0x42],
    second_dark: [0x0f, 0x51, 0x73],
    mid: [0x48, 0x85, 0xa4],
    second_light: [0xa3, 0xd3, 0xfa],
    light: [0xe6, 0xf2, 0xff],
    white: [0xff, 0xff, 0xff],
};

#[allow(dead_code)]
#[derive(Clone, Copy)]
pub enum Color {
    Black,
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
            Color::Black => Color::White,
            Color::Dark => Color::Light,
            Color::SecondDark => Color::SecondLight,
            Color::Light => Color::Dark,
            Color::SecondLight => Color::SecondDark,
            Color::Mid => Color::Mid,
            Color::White => Color::Black,
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
            Color::Black => self.black.into_color(),
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
