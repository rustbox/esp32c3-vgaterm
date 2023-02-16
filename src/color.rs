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

pub fn rgb3_from_rgb(red: u8, green: u8, blue: u8) -> u16 {
    let r3 = red / 36;
    let g3 = green / 36;
    let b3 = blue / 36;

    let r: u16 = (r3 & 0b00000111).into();
    let g: u16 = (g3 as u16 & 0b00000111_u16) << 3;
    let b: u16 = (b3 as u16 & 0b00000111_u16) << 6;
    r + g + b
}

pub fn rgb3_to_byte(rgb3: u16) -> u8 {
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
}

impl Rgb3 {
    pub const fn new(r: u8, g: u8, b: u8) -> Rgb3 {
        Rgb3 {
            red: r % 8,
            green: g % 8,
            blue: b % 8,
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
