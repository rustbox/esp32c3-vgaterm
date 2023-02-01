use embedded_graphics::prelude::DrawTarget;

use crate::{
    color::Rgb3,
    display::{self, TextDisplay},
};

pub type Row = usize;
pub type Col = usize;

pub struct TextField {
    text: TextDisplay,
    cursor: (Row, Col),
}

impl TextField {
    pub fn new() -> TextField {
        TextField {
            text: TextDisplay::new(),
            cursor: (0, 0),
        }
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
        D: DrawTarget<Color = Rgb3>,
    {
        self.text.draw_dirty(target);
    }
}
