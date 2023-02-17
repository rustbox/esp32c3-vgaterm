use core::convert::Infallible;

use alloc::{
    collections::VecDeque,
    string::{String, ToString},
};
use embedded_graphics::{
    mono_font::{MonoTextStyle, MonoTextStyleBuilder},
    pixelcolor::raw::RawU8,
    prelude::*,
    text::Text,
    Pixel,
};

use crate::{
    color::{self, Rgb3},
    video,
};

pub struct Display {
    local_buffer: VecDeque<(usize, u8)>,
}

impl Display {
    pub fn new() -> Display {
        Display {
            local_buffer: VecDeque::new(),
        }
    }

    pub fn push(&mut self, pos: usize, color: u8) {
        if self.local_buffer.len() >= 512 {
            self.flush();
        }
        self.local_buffer.push_front((pos, color))
    }

    pub fn flush(&mut self) {
        while let Some((pos, px)) = self.local_buffer.pop_back() {
            riscv::interrupt::free(|| unsafe {
                video::BUFFER[pos] = px;
            });
        }
    }

    pub fn read(&self, x: usize, y: usize) -> u8 {
        riscv::interrupt::free(|| unsafe { video::BUFFER[y * video::WIDTH + x] })
    }
}

impl Default for Display {
    fn default() -> Self {
        Self::new()
    }
}

impl OriginDimensions for Display {
    fn size(&self) -> Size {
        Size::new(video::WIDTH as u32, video::HEIGHT as u32)
    }
}

impl DrawTarget for Display {
    type Color = color::Rgb3;
    type Error = Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = embedded_graphics::Pixel<Self::Color>>,
    {
        for Pixel(coord, color) in pixels.into_iter() {
            if coord.x >= 0
                && coord.x < video::WIDTH as i32
                && coord.y >= 0
                && coord.y < video::HEIGHT as i32
            {
                let i = coord.y as usize * video::WIDTH + coord.x as usize;
                let raw = RawU8::from(color);
                self.push(i, raw.into_inner());
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Character {
    character: [u8; 2],
    pub color: CharColor,
}

impl Character {
    pub fn new(ch: char) -> Character {
        let mut chrs: [u8; 2] = [0; 2];
        let s = ch.to_string();
        let ch = s.as_str().as_bytes();
        chrs[0] = ch[0];
        if ch.len() > 1 {
            chrs[1] = ch[1];
        }
        Character {
            character: chrs,
            color: CharColor::default(),
        }
    }

    pub fn with_char(&mut self, ch: char) -> Character {
        let mut chrs: [u8; 2] = [0; 2];
        let ch = (ch as u32).to_ne_bytes();
        chrs[0] = ch[0];
        chrs[1] = ch[1];

        self.character = chrs;
        Character {
            character: chrs,
            color: self.color,
        }
    }

    pub fn with_fore(self, color: Rgb3) -> Character {
        Character {
            character: self.character,
            color: self.color.with_foreground(color),
        }
    }

    pub fn with_back(self, color: Rgb3) -> Character {
        Character {
            character: self.character,
            color: self.color.with_background(color),
        }
    }

    pub fn char(&self) -> char {
        let c: u32 = ((self.character[0] as u32) + (self.character[1] as u32)) << 8;
        char::from_u32(c).unwrap_or(' ')
    }

    pub fn text_and_style(&self) -> (String, MonoTextStyle<Rgb3>) {
        let text = core::str::from_utf8(&self.character)
            .unwrap_or(" ")
            .to_string();

        let mut style_builder = if self.color.inverse().is_some() {
            MonoTextStyleBuilder::new()
                .text_color(self.color.background())
                .background_color(self.color.foreground())
                .font(&crate::text::TAMZEN_FONT_6x12)
        } else {
            MonoTextStyleBuilder::new()
                .text_color(self.color.foreground())
                .background_color(self.color.background())
                .font(&crate::text::TAMZEN_FONT_6x12)
        };

        if self.color.strikethrough().is_some() {
            style_builder = style_builder.strikethrough();
        }

        if self.color.underline().is_some() {
            style_builder = style_builder.underline();
        }

        let style = style_builder.build();

        (text, style)
    }
}

impl Default for Character {
    fn default() -> Self {
        Character {
            character: [32, 0],
            color: CharColor::default(),
        }
    }
}

/// Inverse: 15, Underline: 14, Strike: 13, Blink: 12, Back: 6-11 bits, Fore: 0-5 bits
/// IUSB|bbgg.rr|bbggrr
#[derive(Debug, Clone, Copy)]
pub struct CharColor(u16);

impl CharColor {
    pub fn new(foreground: Rgb3, background: Rgb3) -> CharColor {
        let fore2 = foreground.rgb2();
        let back2 = background.rgb2();
        let c = ((fore2.0 as u16) >> 1)
            | ((fore2.1 as u16) << 1)
            | ((fore2.2 as u16) << 3)
            | ((back2.0 as u16) << 5)
            | ((back2.1 as u16) << 7)
            | ((back2.2 as u16) << 9);

        // IUSB|bbgg.rr|bbggrr
        // bb0.gg0.rr
        CharColor(c)
    }

    pub fn foreground(&self) -> Rgb3 {
        let r = ((self.0 & 0b00000011) << 1) as u8;
        let g = ((self.0 & 0b00001100) >> 1) as u8;
        let b = ((self.0 & 0b00110000) >> 3) as u8;
        Rgb3::from_rgb2(r, g, b)
    }

    pub fn background(&self) -> Rgb3 {
        let r = ((self.0 & 0b11000000) >> 5) as u8;
        let g = ((self.0 & 0b0000_0011_0000_0000) >> 7) as u8;
        let b = ((self.0 & 0b0000_1100_0000_0000) >> 9) as u8;
        Rgb3::from_rgb2(r, g, b)
    }

    pub fn inverse(&self) -> Option<Inverse> {
        if self.0 & Inverse.bit() != 0 {
            Some(Inverse)
        } else {
            None
        }
    }

    pub fn underline(&self) -> Option<Underline> {
        if self.0 & Underline.bit() != 0 {
            Some(Underline)
        } else {
            None
        }
    }

    pub fn strikethrough(&self) -> Option<Strikethrough> {
        if self.0 & Strikethrough.bit() != 0 {
            Some(Strikethrough)
        } else {
            None
        }
    }

    pub fn blink(&self) -> Option<Blink> {
        if self.0 & Blink.bit() != 0 {
            Some(Blink)
        } else {
            None
        }
    }

    pub fn with_foreground(self, color: Rgb3) -> CharColor {
        let (r2, g2, b2) = color.rgb2();
        let c = ((r2 + g2) << (2 + b2) << 4) as u16;

        CharColor((self.0 & 0b1111_1111_1100_0000) | c)
    }

    pub fn with_background(self, color: Rgb3) -> CharColor {
        let (r2, g2, b2) = color.rgb2();
        // Background starts at bit 6
        let c = (((r2 + g2) << (2 + b2) << 4) as u16) << 6;

        CharColor((self.0 & 0b1111_0000_0011_1111) | c)
    }

    pub fn with_decoration(
        &mut self,
        inverse: Option<Inverse>,
        underline: Option<Underline>,
        strikethrough: Option<Strikethrough>,
        blink: Option<Blink>,
    ) -> CharColor {
        let decs = inverse.bit() + underline.bit() + strikethrough.bit() + blink.bit();
        self.0 = (self.0 & 0b0000_1111_1111_1111) | decs;
        *self
    }

    // If inverted, go back to not inverted, and if not inverted, do the invert
    pub fn invert_fore_back(&mut self) {
        if self.inverse().is_some() {
            self.with_decoration(None, self.underline(), self.strikethrough(), self.blink());
        } else {
            self.with_decoration(
                Some(Inverse),
                self.underline(),
                self.strikethrough(),
                self.blink(),
            );
        }
    }
}

impl Default for CharColor {
    fn default() -> CharColor {
        CharColor::new(Rgb3::new(6, 6, 6), Rgb3::BLACK)
    }
}

pub struct Blink;

impl Flag for Blink {
    fn bit(&self) -> u16 {
        1 << 12
    }
}

pub struct Strikethrough;

impl Flag for Strikethrough {
    fn bit(&self) -> u16 {
        1 << 13
    }
}

pub struct Underline;

impl Flag for Underline {
    fn bit(&self) -> u16 {
        1 << 14
    }
}

pub struct Inverse;

impl Flag for Inverse {
    fn bit(&self) -> u16 {
        1 << 15
    }
}

impl<T: Flag> Flag for Option<T> {
    fn bit(&self) -> u16 {
        match self {
            Some(f) => f.bit(),
            None => 0,
        }
    }
}

pub trait Flag {
    fn bit(&self) -> u16;
}

pub const COLUMNS: usize = 105;
pub const ROWS: usize = 33;

pub struct TextDisplay {
    buffer: [Character; ROWS * COLUMNS],
    dirty: VecDeque<((usize, usize), Character)>,
}

impl TextDisplay {
    pub fn new() -> TextDisplay {
        TextDisplay {
            buffer: [Character::default(); COLUMNS * ROWS],
            dirty: VecDeque::new(),
        }
    }

    pub fn read_char(&self, line: usize, col: usize) -> Character {
        self.buffer[line * COLUMNS + col]
    }

    pub fn write(&mut self, line: usize, col: usize, c: char) {
        let ch = Character::new(c);
        let i = line * COLUMNS + col;
        self.buffer[i] = ch;
        self.dirty.push_front(((line, col), ch));
    }

    pub fn write_text(&mut self, start_line: usize, start_column: usize, text: &str) {
        let start_line = start_line % ROWS;
        let start_column = start_column % COLUMNS;
        // We know now that the start cell is within the frame
        let mut row = start_line;
        let mut col = start_column;
        for (_, c) in text.chars().enumerate() {
            self.write(row, col, c);
            col += 1;
            if col == COLUMNS {
                col = 0;
                row += 1;
                if row == ROWS {
                    row = 0
                }
            }
        }
    }

    pub fn draw<D>(&self, line: usize, col: usize, target: &mut D)
    where
        D: DrawTarget<Color = Rgb3>,
    {
        let ch = self.read_char(line, col);
        self.draw_character(line, col, ch, target);
    }

    pub fn draw_all<D>(&self, target: &mut D)
    where
        D: DrawTarget<Color = Rgb3>,
    {
        for l in (0..ROWS).rev() {
            for c in (0..COLUMNS).rev() {
                self.draw(l, c, target);
            }
        }
    }

    pub fn draw_dirty<D>(&mut self, target: &mut D)
    where
        D: DrawTarget<Color = Rgb3>,
    {
        while let Some(((line, col), _)) = self.dirty.pop_back() {
            self.draw(line, col, target)
        }
    }

    pub fn draw_character<D>(&self, line: usize, col: usize, character: Character, target: &mut D)
    where
        D: DrawTarget<Color = Rgb3>,
    {
        let (text, style) = character.text_and_style();

        let w = style.font.character_size.width;
        let h = style.font.character_size.height;
        let x = 2 + col as u32 * (w + style.font.character_spacing);
        let y = line as u32 * h + h;

        let text = Text::new(&text, Point::new(x as i32, y as i32), style);

        let _ = text.draw(target);
    }
}

impl Default for TextDisplay {
    fn default() -> Self {
        Self::new()
    }
}
