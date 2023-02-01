use embedded_graphics::prelude::DrawTarget;

use crate::{display::{TextDisplay, self, Inverse}, color::Rgb3};

pub type Row = usize;
pub type Col = usize;

pub struct TextField {
    text: TextDisplay,
    cursor: (Row, Col)
}

impl TextField {
    pub fn new() -> TextField {
        TextField { text: TextDisplay::new(), cursor: (0, 0) }
    }

    pub fn type_next(&mut self, t: char) {
        self.text.write(self.cursor.0, self.cursor.1, t);
        self.cursor.1 += 1;
        if self.cursor.1 == display::COLUMNS {
            self.cursor.1 = 0;
            self.cursor.0 += 1;
            if self.cursor.0 == display::ROWS {
                self.cursor.0 = 0;
            }
        }
    }

    pub fn draw<D>(&mut self, target: &mut D)
    where
        D: DrawTarget<Color = Rgb3> {

        let mut cursor_char = self.text.read_char(self.cursor.0, self.cursor.1);
        cursor_char.color.with_decoration(Some(Inverse), None, None, None);
        self.text.draw_dirty(target);
        self.text.draw_character(self.cursor.0, self.cursor.1, cursor_char, target);
        
    }
}
