use core::convert::Infallible;

use alloc::{
    collections::VecDeque,
    string::{String, ToString},
};
use embedded_graphics::{
    mono_font::{MonoTextStyle, MonoTextStyleBuilder},
    prelude::*,
    primitives::Rectangle,
    text::Text,
    Pixel,
};
use esp_println::print;

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

    /// Sets the pixel color the location in the video BUFFER
    /// to the given color
    ///
    /// SAFETY: This directly sets the pixel to video memory which
    /// is unsafe, but should be okay since we're the only ones
    /// setting memory in the buffer and SPI takes exclusive control
    /// when it runs to display the pixels
    #[inline(always)]
    pub fn set_pixel(&mut self, pos: usize, color: u8) {
        *unsafe { &mut video::BUFFER[pos] } = color;
    }

    pub fn flush(&mut self) {
        while let Some((pos, px)) = self.local_buffer.pop_back() {
            *unsafe { &mut video::BUFFER[pos] } = px;
        }
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
        let mut count = 0;
        crate::measure(&mut count, || {
            for Pixel(coord, color) in pixels.into_iter() {
                if coord.x >= 0
                    && coord.x < video::WIDTH as i32
                    && coord.y >= 0
                    && coord.y < video::HEIGHT as i32
                {
                    let i = coord.y as usize * video::WIDTH + coord.x as usize;
                    // let raw = RawU8::from(color);
                    self.set_pixel(i, color.to_byte());
                }
            }
        });
        // unsafe { crate::CHARACTER_DRAW_CYCLES += count };
        Ok(())
    }

    #[inline(always)]
    fn fill_contiguous<I>(&mut self, area: &Rectangle, colors: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Self::Color>,
    {
        let mut count = 0;
        crate::measure(&mut count, || {
            let mut colors = colors.into_iter();
            let screen_width = self.size().width as usize;
            let area_width = area.size.width as usize;

            let mut offset = screen_width * area.top_left.y as usize + area.top_left.x as usize;
            for _ in 0..area.size.height {
                for col in 0..area_width {
                    let i = offset + col;
                    unsafe { video::BUFFER[i] = colors.next().unwrap().to_byte() };
                }
                offset += screen_width;
            }
        });
        unsafe { crate::CHARACTER_DRAW_CYCLES += count };
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

        let mut style_builder = if self.color.inverse() {
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

        if self.color.strikethrough() {
            style_builder = style_builder.strikethrough();
        }

        if self.color.underline() {
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

    pub fn inverse(&self) -> bool {
        self.0 & Decoration::Inverse.bit() != 0
    }

    pub fn underline(&self) -> bool {
        self.0 & Decoration::Underline.bit() != 0
    }

    pub fn strikethrough(&self) -> bool {
        self.0 & Decoration::Strikethrough.bit() != 0
    }

    pub fn blink(&self) -> bool {
        self.0 & Decoration::Blink.bit() != 0
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

    pub fn with_decorations(&mut self, decs: &[Decoration]) -> CharColor {
        let mut dec_value = 0;
        for d in decs {
            dec_value |= d.bit();
        }
        self.0 = (self.0 & 0b0000_1111_1111_1111) | dec_value;
        *self
    }

    pub fn invert_colors(&mut self) -> CharColor {
        self.0 ^= Decoration::Inverse.bit();
        *self
    }

    pub fn set_inverted(&mut self) -> CharColor {
        if self.0 & Decoration::Inverse.bit() == 0 {
            self.0 |= Decoration::Inverse.bit();
        }
        *self
    }

    pub fn reset_inverted(&mut self) -> CharColor {
        if self.0 & Decoration::Inverse.bit() != 0 {
            self.0 &= !Decoration::Inverse.bit();
        }
        *self
    }
}

impl Default for CharColor {
    fn default() -> CharColor {
        CharColor::new(Rgb3::new(6, 6, 6), Rgb3::BLACK)
    }
}

#[derive(Debug, PartialEq, PartialOrd)]
pub enum Decoration {
    Blink,
    Strikethrough,
    Underline,
    Inverse,
}

impl Flag for Decoration {
    fn bit(&self) -> u16 {
        match self {
            Decoration::Blink => 1 << 12,
            Decoration::Strikethrough => 1 << 13,
            Decoration::Underline => 1 << 14,
            Decoration::Inverse => 1 << 15,
        }
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

#[inline(always)]
fn index(row: usize, col: usize) -> usize {
    row * COLUMNS + col
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Drawn {
    Dirty,
    Clean,
}

pub struct TextDisplay {
    buffer: [(Character, Drawn); ROWS * COLUMNS],
    num_dirty: usize,
}

impl TextDisplay {
    pub fn new() -> TextDisplay {
        TextDisplay {
            buffer: [(Character::default(), Drawn::Clean); COLUMNS * ROWS],
            num_dirty: 0,
        }
    }

    #[inline(always)]
    pub fn read_char(&self, line: usize, col: usize) -> Character {
        self.buffer[index(line, col)].0
    }

    #[inline(always)]
    pub fn write_char(&mut self, line: usize, col: usize, c: Character) {
        self.buffer[index(line, col)] = (c, Drawn::Dirty);
        self.num_dirty += 1;
    }

    #[inline(always)]
    pub fn write(&mut self, line: usize, col: usize, c: char) {
        let ch = Character::new(c);
        let i = line * COLUMNS + col;
        self.buffer[i] = (ch, Drawn::Dirty);
        self.num_dirty += 1;
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
    /// Rows = 11;
    /// 0 a b c d e f
    /// 1 a b c d e f _
    /// 2 a b c d e f
    /// 3 a b c d e f _
    /// 4 a b c d e f
    /// 5 a b c d e f _
    /// 6 a b c d e f
    /// 7 a b c d e f _
    /// 8 a b c d e f
    /// 9 a b c d e f _
    /// 10 a b c d e f
    ///
    /// 0, 2, 4, 6, 8, 10, 12, 14, 16, 18, 20, 22, 24, 26, 28, |30 [31 32
    ///
    pub fn scroll_down(&mut self, amount: usize) {
        // We add a correction if amount and COLUMNS are differ in even/odd parity
        let odd = if amount & 1 == COLUMNS & 1 { 0 } else { 1 };
        for l in (0..ROWS - amount - odd).step_by(amount) {
            let double_line = &mut self.buffer[l * COLUMNS..(l + 2 * amount) * COLUMNS];
            for (_, drawn) in double_line.iter_mut() {
                *drawn = Drawn::Dirty;
            }
            let (first, second) = double_line.split_at_mut(amount * COLUMNS);
            first.swap_with_slice(second);
        }
        if odd == 1 {
            for l in (ROWS - amount..ROWS).rev() {
                let double_line = &mut self.buffer[(l - 1) * COLUMNS..(l + 1) * COLUMNS];
                for (_, drawn) in double_line.iter_mut() {
                    *drawn = Drawn::Dirty;
                }
                let (first, second) = double_line.split_at_mut(COLUMNS);
                first.swap_with_slice(second);
            }
        }
        let last = &mut self.buffer[(ROWS - amount) * COLUMNS..ROWS * COLUMNS];
        last.fill((Character::new(' '), Drawn::Dirty))
    }

    #[inline(always)]
    pub fn clear(&mut self) {
        for row in 0..ROWS {
            for col in 0..COLUMNS {
                self.write(row, col, ' ');
            }
        }
    }

    #[inline(always)]
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

    #[inline(always)]
    pub fn draw_dirty<D>(&mut self, target: &mut D)
    where
        D: DrawTarget<Color = Rgb3>,
    {
        if self.num_dirty == 0 {
            return;
        }
        for row in 0..ROWS {
            for col in 0..COLUMNS {
                let i = COLUMNS * row + col;
                if self.buffer[i].1 == Drawn::Dirty {
                    self.buffer[i].1 = Drawn::Clean;
                    self.draw(row, col, target);
                    self.num_dirty -= 1;
                }
            }
        }
    }

    #[inline(always)]
    pub fn draw_dirty_up_to<D>(&mut self, up_to: usize, target: &mut D)
    where
        D: DrawTarget<Color = Rgb3>,
    {
        if self.num_dirty == 0 {
            return;
        }
        let mut drawn = 0;
        for row in 0..ROWS {
            for col in 0..COLUMNS {
                let i = COLUMNS * row + col;
                if self.buffer[i].1 == Drawn::Dirty {
                    self.buffer[i].1 = Drawn::Clean;
                    self.draw(row, col, target);
                    self.num_dirty -= 1;
                    drawn += 1;
                    if drawn >= up_to || self.num_dirty == 0 {
                        return;
                    }
                }
            }
        }
    }

    #[inline(always)]
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
