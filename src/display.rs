use core::convert::Infallible;

use alloc::{vec::Vec, string::{ToString, String}, collections::VecDeque};
use embedded_graphics::{prelude::*, pixelcolor::raw::RawU8, Pixel, text::Text, mono_font::{MonoTextStyle, MonoTextStyleBuilder}};
use esp_println::println;

use crate::{video, color::{self, Rgb3}};


pub struct Display {
    local_buffer: Vec<(usize, u8)>
}

impl Display {

    pub fn new() -> Display {
        Display { local_buffer: Vec::new() }
    }

    pub fn push(&mut self, pos: usize, color: u8) {
        if self.local_buffer.len() >= 512 {
            self.internal_flush()
        }
        self.local_buffer.push((pos, color))
    }

    fn internal_flush(&mut self) {
        riscv::interrupt::free(|| unsafe {
            // TODO can we do like bulk insert this local buffer as a slice into the BUFFER?
            while let Some((pos, px)) = self.local_buffer.pop() {
                video::BUFFER[pos] = px;
            }
        });
    }

    pub fn flush(&mut self) {
        while let Some((pos, px)) = self.local_buffer.pop() {
            riscv::interrupt::free(|| unsafe {
                video::BUFFER[pos] = px;
            });
        }
    }

    pub fn read(&self, x: usize, y: usize) -> u8 {
        riscv::interrupt::free(|| unsafe {
            video::BUFFER[y * video::WIDTH + x]
        })
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
            I: IntoIterator<Item = embedded_graphics::Pixel<Self::Color>> {
        
        for Pixel(coord, color) in pixels.into_iter() {
            if coord.x >= 0 && coord.x < video::WIDTH as i32 && coord.y >= 0 && coord.y < video::HEIGHT as i32 {
                let i = coord.y as usize * video::WIDTH + coord.x as usize;
                let raw = RawU8::from(color);
                self.push(i, raw.into_inner());
            }
        }
                
        Ok(())
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub struct Character {
    character: [u8; 2],
    color: CharColor
}

impl Character {

    pub fn new(ch: char) -> Character{
        let mut chrs: [u8; 2] = [0; 2];
        let s = ch.to_string();
        let ch = s.as_str().as_bytes();
        chrs[0] = ch[0];
        if ch.len() > 1 {
            chrs[1] = ch[1];
        }
        Character { character: chrs, color: CharColor::default() }
    }

    pub fn text_and_style(&self) -> (String, MonoTextStyle<Rgb3>) {
        let text = core::str::from_utf8(&self.character)
            .unwrap_or(" ").to_string();

        let style = MonoTextStyleBuilder::new()
            .text_color(self.color.foreground())
            .background_color(self.color.background())
            .font(&crate::text::TAMZEN_FONT_6x12)
            .build();

        
        (text, style)
    }
}


/// Reverse: 15, Underline: 14, Strike: 13, Blink: 12, Back: 6-11 bits, Fore: 0-5 bits
/// RUSB|bbgg.rr|bbggrr
#[derive(Debug, Clone, Copy)]
struct CharColor(u16);

impl CharColor {
    fn new(foreground: Rgb3, background: Rgb3) -> CharColor {
        let fore2 = foreground.rgb2();
        let back2 = background.rgb2();
        let c = 
            ((fore2.0 as u16) >> 1) |
            ((fore2.1 as u16) << 1) |
            ((fore2.2 as u16) << 3) |
            ((back2.0 as u16) << 5) |
            ((back2.1 as u16) << 7) |
            ((back2.2 as u16) << 9);
        

        // RUSB|bbgg.rr|bbggrr
        // bb0.gg0.rr
        CharColor(c)
    }

    fn foreground(&self) -> Rgb3 {
        let r = ((self.0 & 0b00000011) << 1) as u8;
        let g = ((self.0 & 0b00001100) >> 1) as u8;
        let b = ((self.0 & 0b00110000) >> 3) as u8;
        Rgb3::from_rgb2(r, g, b)
    }

    fn background(&self) -> Rgb3 {
        let r = ((self.0 & 0b11000000) >> 5) as u8;
        let g = ((self.0 & 0b00000011_00000000) >> 7) as u8;
        let b = ((self.0 & 0b00001100_00000000) >> 9) as u8;
        Rgb3::from_rgb2(r, g, b)
    }
}

impl Default for CharColor {
    fn default() -> CharColor {
        CharColor::new(Rgb3::new(6, 6, 6), Rgb3::BLACK)
    }
}


enum Decoration {
    Blink,
    Strikethrough,
    Underline,
    Inverse
}

pub const COLUMNS: usize = 105;
pub const ROWS: usize = 33;


pub struct TextDisplay {
    buffer: [Character; ROWS * COLUMNS],
    dirty: VecDeque<((usize, usize), Character)>
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
                println!("resetting line, {}, {}", row, col);
                if row == ROWS {
                    row = 0
                }
            }

        }
    }

    pub fn draw<D>(&self, line: usize, col: usize, target: &mut D)
    where
        D: DrawTarget<Color = Rgb3> {

        let ch = self.read_char(line, col);
        let (text, style) = ch.text_and_style();
        
        let w = style.font.character_size.width;
        let h = style.font.character_size.height;
        let x = 2 + col as u32 * (w + style.font.character_spacing);
        let y = line as u32 * h + h;

        let text = Text::new(
            &text, 
            Point::new(x as i32, y as i32), 
            style
        );
        
        let _ = text.draw(target);
    }

    pub fn draw_all<D>(&self, target: &mut D)
    where
        D: DrawTarget<Color = Rgb3> {

        for l in (0..ROWS).rev() {
            for c in (0..COLUMNS).rev() {
                self.draw(l, c, target);
            }
        }
    }

    pub fn draw_dirty<D>(&mut self, target: &mut D)
    where
        D: DrawTarget<Color = Rgb3> {

        while let Some(((line, col), _)) = self.dirty.pop_back() {
            self.draw(line, col, target)
        }
    }
}
