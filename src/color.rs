use core::fmt::Display;

use embedded_graphics::{
    pixelcolor::raw::RawU8,
    prelude::{PixelColor, RawData, RgbColor},
};

pub fn rgb_from_byte(color: u8) -> (u8, u8, u8) {
    let color: u16 = color as u16;
    let shifted = color << 1;
    let highest = shifted >> 7;
    let rgb = shifted + highest;

    let red3 = (rgb & 0b0_0000_0111) as u8;
    let green3 = ((rgb >> 3) & 0b0_0000_0111) as u8;
    let blue3 = ((rgb >> 6) & 0b0_0000_0111) as u8;

    (
        color3_to_byte(red3),
        color3_to_byte(green3),
        color3_to_byte(blue3),
    )
}

pub fn byte_from_rgb(red: u8, green: u8, blue: u8) -> u8 {
    let r3 = red / 36;
    let g3 = green / 36;
    let b3 = blue / 36;

    let r: u16 = (r3 & 0b00000111).into();
    let g: u16 = (g3 as u16 & 0b00000111_u16) << 3;
    let b: u16 = (b3 as u16 & 0b00000111_u16) << 6;
    let rgb3: u16 = r + g + b;
    // Convert to vgaterm color byte by rshift 1
    (rgb3 >> 1) as u8
}

#[inline(always)]
pub fn rgb3_from_rgb(red: u8, green: u8, blue: u8) -> u16 {
    let r3 = red / 36;
    let g3 = green / 36;
    let b3 = blue / 36;

    let r: u16 = (r3 & 0b00000111).into();
    let g: u16 = (g3 as u16 & 0b00000111_u16) << 3;
    let b: u16 = (b3 as u16 & 0b00000111_u16) << 6;
    r + g + b
}

#[inline(always)]
pub const fn rgb3_to_byte(rgb3: u16) -> u8 {
    (rgb3 >> 1) as u8
}

pub fn byte_to_rgb3(b: u8) -> (u8, u8, u8) {
    let shifted: u16 = u16::from(b) << 1;
    // Grab the high bit of b, and add it to shifted
    let tail = u16::from((b & 0b10000000) >> 7);
    let rgb_bits = shifted + tail;

    (
        (rgb_bits & 0b000000111) as u8,
        ((rgb_bits & 0b000111000) >> 3) as u8,
        ((rgb_bits & 0b111000000) >> 6) as u8,
    )
}

pub fn color3_to_byte(color: u8) -> u8 {
    let color = color % 8;
    match color {
        0 => 0,
        1 => 36,
        2 => 72,
        3 => 109,
        4 => 145,
        5 => 182,
        6 => 218,
        7 => 255,
        _ => 0,
    }
}

pub fn color2_to_color3(color2: u8) -> u8 {
    // Set to 2 bit value
    let color2 = color2 & 0b00000011;
    match color2 {
        0 => 0,
        1 => 3,
        2 => 6,
        3 => 7,
        _ => unreachable!(),
    }
}

pub fn color3_to_color2(color3: u8) -> u8 {
    let color3 = color3 & 0b00000111;
    match color3 {
        0 => 0,
        1 => 0,
        2 => 1,
        3 => 1,
        4 => 2,
        5 => 2,
        6 => 2,
        7 => 3,
        _ => unreachable!(),
    }
}

/// A 9 bit (3 bits per color channel) RGB color
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Rgb3 {
    red: u8,
    green: u8,
    blue: u8,
    brightness: u8,
}

impl Rgb3 {
    pub const fn new(r: u8, g: u8, b: u8) -> Rgb3 {
        Rgb3 {
            red: r % 8,
            green: g % 8,
            blue: b % 8,
            brightness: 8,
        }
    }

    pub const fn from_rgb(r: u8, g: u8, b: u8) -> Rgb3 {
        let r3 = r / 36;
        let g3 = g / 36;
        let b3 = b / 36;
        Rgb3::new(r3, g3, b3)
    }

    pub fn brightness(&self, b: u8) -> Rgb3 {
        Rgb3 {
            red: self.red,
            green: self.green,
            blue: self.blue,
            brightness: b,
        }
    }

    pub const fn render(&self) -> Rgb3 {
        if self.brightness <= 8 {
            Rgb3 {
                red: self.red * self.brightness / 8,
                green: self.green * self.brightness / 8,
                blue: self.blue * self.brightness / 8,
                brightness: 8,
            }
        } else {
            let bright = self.brightness - 8;
            Rgb3 {
                red: ((8 - self.red) * bright) / 8 + self.red,
                green: ((8 - self.green) * bright) / 8 + self.green,
                blue: ((8 - self.blue) * bright) / 8 + self.blue,
                brightness: 8,
            }
        }
    }

    pub fn from_rgb2(r: u8, g: u8, b: u8) -> Rgb3 {
        Rgb3::new(
            color2_to_color3(r),
            color2_to_color3(g),
            color2_to_color3(b),
        )
    }

    pub fn rgb2(&self) -> (u8, u8, u8) {
        (
            color3_to_color2(self.r()),
            color3_to_color2(self.g()),
            color3_to_color2(self.b()),
        )
    }

    #[inline(always)]
    pub const fn to_byte(&self) -> u8 {
        // RGB3 to byte goes like:
        //     2  1  0
        // R: b1 b0 b7
        // G: b4 b3 b2
        // B: b7 b6 b5
        // 7-2 6-1 5-0 4-2 3-1 2-0 1-0 0-1
        // B2, B1, B0, G2, G1, G0, R2, R1
        // Each color channel goes from 0-7, bits 0-2
        let rendered = self.render();
        rendered.blue << 5 | rendered.green << 2 | rendered.red >> 1
    }
}

impl Display for Rgb3 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Rgb3({}, {}, {})", self.red, self.green, self.blue)
    }
}

impl PixelColor for Rgb3 {
    type Raw = RawU8;
}

impl RgbColor for Rgb3 {
    fn r(&self) -> u8 {
        self.red
    }

    fn g(&self) -> u8 {
        self.green
    }

    fn b(&self) -> u8 {
        self.blue
    }

    const MAX_R: u8 = 7;
    const MAX_G: u8 = 7;
    const MAX_B: u8 = 7;

    const BLACK: Self = Rgb3::new(0, 0, 0);
    const WHITE: Self = Rgb3::new(Self::MAX_R, Self::MAX_G, Self::MAX_B);
    const RED: Self = Rgb3::new(Self::MAX_R, 0, 0);
    const GREEN: Self = Rgb3::new(0, Self::MAX_G, 0);
    const BLUE: Self = Rgb3::new(0, 0, Self::MAX_B);
    const YELLOW: Self = Rgb3::new(Self::MAX_R, Self::MAX_G, 0);
    const CYAN: Self = Rgb3::new(0, Self::MAX_G, Self::MAX_B);
    const MAGENTA: Self = Rgb3::new(Self::MAX_R, 0, Self::MAX_B);
}

impl From<RawU8> for Rgb3 {
    fn from(value: RawU8) -> Rgb3 {
        let (r, g, b) = byte_to_rgb3(value.into_inner());
        Rgb3::new(r, g, b)
    }
}

impl From<Rgb3> for RawU8 {
    fn from(value: Rgb3) -> RawU8 {
        let mut v: u16 = value.r().into();
        v += u16::from(value.g()) << 3;
        v += u16::from(value.b()) << 6;

        RawU8::new(rgb3_to_byte(v))
    }
}

pub const ANSI_BASE_FG_LOW_START: u8 = 30;
pub const ANSI_BASE_FG_LOW_STOP: u8 = ANSI_BASE_FG_LOW_START + 7;
pub const ANSI_BASE_FG_HIGH_START: u8 = 90;
pub const ANSI_BASE_FG_HIGH_STOP: u8 = ANSI_BASE_FG_HIGH_START + 7;
pub const ANSI_BASE_BG_LOW_START: u8 = 40;
pub const ANSI_BASE_BG_LOW_STOP: u8 = ANSI_BASE_BG_LOW_START + 7;
pub const ANSI_BASE_BG_HIGH_START: u8 = 100;
pub const ANSI_BASE_BG_HIGH_STOP: u8 = ANSI_BASE_BG_HIGH_START + 7;

pub static ANSI_BASE_LOW_COLORS: [Rgb3; 8] = [
    Rgb3::BLACK,
    Rgb3::new(5, 0, 0),
    Rgb3::new(0, 5, 0),
    Rgb3::new(5, 5, 0),
    Rgb3::new(0, 0, 5),
    Rgb3::new(5, 0, 5),
    Rgb3::new(0, 5, 5),
    Rgb3::new(4, 4, 4),
];
pub static ANSI_BASE_HIGH_COLORS: [Rgb3; 8] = [
    Rgb3::new(2, 2, 2),
    Rgb3::new(7, 0, 0),
    Rgb3::new(0, 7, 0),
    Rgb3::new(7, 7, 0),
    Rgb3::new(0, 0, 7),
    Rgb3::new(7, 2, 7),
    Rgb3::new(2, 7, 7),
    Rgb3::WHITE,
];

pub const BLACK_FG: u8 = 30;
pub const RED_FG: u8 = 31;
pub const GREEN_FG: u8 = 32;
pub const YELLOW_FG: u8 = 33;
pub const BLUE_FG: u8 = 34;
pub const MAGENTA_FG: u8 = 35;
pub const CYAN_FG: u8 = 36;
pub const WHITE_FG: u8 = 37;

pub const BRIGHT_BLACK_FG: u8 = 90;
pub const BRIGHT_RED_FG: u8 = 91;
pub const BRIGHT_GREEN_FG: u8 = 92;
pub const BRIGHT_YELLOW_FG: u8 = 93;
pub const BRIGHT_BLUE_FG: u8 = 94;
pub const BRIGHT_MAGENTA_FG: u8 = 95;
pub const BRIGHT_CYAN_FG: u8 = 96;
pub const BRIGHT_WHITE_FG: u8 = 97;

pub const BLACK_BG: u8 = 40;
pub const RED_BG: u8 = 41;
pub const GREEN_BG: u8 = 42;
pub const YELLOW_BG: u8 = 43;
pub const BLUE_BG: u8 = 44;
pub const MAGENTA_BG: u8 = 45;
pub const CYAN_BG: u8 = 46;
pub const WHITE_BG: u8 = 47;

pub const BRIGHT_BLACK_BG: u8 = 100;
pub const BRIGHT_RED_BG: u8 = 101;
pub const BRIGHT_GREEN_BG: u8 = 102;
pub const BRIGHT_YELLOW_BG: u8 = 103;
pub const BRIGHT_BLUE_BG: u8 = 104;
pub const BRIGHT_MAGENTA_BG: u8 = 105;
pub const BRIGHT_CYAN_BG: u8 = 106;
pub const BRIGHT_WHITE_BG: u8 = 107;

pub fn ansi_base_color(fore: u8, back: u8) -> (Rgb3, Rgb3) {
    let fg = match fore {
        ANSI_BASE_FG_LOW_START..=ANSI_BASE_FG_LOW_STOP => {
            ANSI_BASE_LOW_COLORS[(fore - ANSI_BASE_FG_LOW_START) as usize]
        }
        ANSI_BASE_FG_HIGH_START..=ANSI_BASE_FG_HIGH_STOP => {
            ANSI_BASE_HIGH_COLORS[(fore - ANSI_BASE_FG_HIGH_START) as usize]
        }
        _ => Rgb3::BLACK,
    };

    let bg = match back {
        ANSI_BASE_BG_LOW_START..=ANSI_BASE_BG_LOW_STOP => {
            ANSI_BASE_LOW_COLORS[(back - ANSI_BASE_BG_LOW_START) as usize]
        }
        ANSI_BASE_BG_HIGH_START..=ANSI_BASE_BG_HIGH_STOP => {
            ANSI_BASE_HIGH_COLORS[(back - ANSI_BASE_BG_HIGH_START) as usize]
        }
        _ => Rgb3::BLACK,
    };
    (fg, bg)
}

pub fn ansi_256_color(color: u8) -> Rgb3 {
    // 0-15 are ANSI_BASE_LOW_COLORS and ANSI_BASE_HIGH_COLORS
    // then 6 x 6 x 6 color cube offset by 16
    // Last is greyscale from 232-255
    match color {
        0..=7 => ANSI_BASE_LOW_COLORS[color as usize],
        8..=15 => ANSI_BASE_HIGH_COLORS[color as usize - 8],
        16..=231 => {
            // c = red * 36 + green * 6 + blue
            let mut cube_color = color - 16;
            let red6 = cube_color / 36;
            cube_color -= red6 * 36;
            let green6 = cube_color / 6;
            cube_color -= green6 * 6;
            let blue6 = cube_color;

            fn map_6_to_8(v6: u8) -> u8 {
                match v6 {
                    0 => 0,
                    1 => 1,
                    2 => 2,
                    3 => 4,
                    4 => 5,
                    5 => 7,
                    _ => 0,
                }
            }
            Rgb3::new(map_6_to_8(red6), map_6_to_8(green6), map_6_to_8(blue6))
        }
        232..=255 => Rgb3::new(color / 24, color / 24, color / 24),
    }
}
