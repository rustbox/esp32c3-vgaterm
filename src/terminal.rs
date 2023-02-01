use embedded_graphics::prelude::DrawTarget;

use crate::{
    color::Rgb3,
    display::{self, TextDisplay},
};

pub const IROWS: isize = display::ROWS as isize;
pub const ICOLS: isize = display::COLUMNS as isize;

pub type Row = usize;
pub type Col = usize;

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy)]
pub struct Cursor(pub Row, pub Col);

impl Cursor {
    /// a cursor offset by r and c, which may be negative
    ///
    /// # Examples
    ///
    /// ```rust
    /// use vgaterm::{IROWS, ICOLS, Cursor, display::{ROWS, COLUMNS}};
    ///
    /// assert_eq!(Cursor(0, 0).offset(0, 1), Cursor(0, 1));
    /// assert_eq!(Cursor(0, 1).offset(0, -1), Cursor(0, 0));
    ///
    /// // columns wrap
    /// assert_eq!(Cursor(0, 0).offset(0, ICOLS + 2), Cursor(1, 2));
    /// assert_eq!(Cursor(1, 0).offset(0, -ICOLS), Cursor(0, 0));
    /// assert_eq!(Cursor(2, 0).offset(0, -ICOLS - 1), Cursor(0, COLUMNS - 1));
    ///
    /// // as do rows
    /// assert_eq!(Cursor(0, 0).offset(IROWS + 1, 0), Cursor(1, 0));
    /// assert_eq!(Cursor(1, 0).offset(-2, 0), Cursor(ROWS - 1, 0));
    /// assert_eq!(Cursor(0, 0).offset(-1, -1), Cursor(ROWS - 2, COLUMNS - 1));
    /// ```
    ///
    /// (see also: https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=3aacdae98b11d36599604d6300f1c71f
    ///  whoever said there's no testing in no_std?)
    pub fn offset(&self, r: isize, c: isize) -> Cursor {
        let cols = self.1 as isize + c;
        let (p, cols) = (cols.div_euclid(ICOLS), cols.rem_euclid(ICOLS));
        let rows = (self.0 as isize + r + p) % IROWS;
        let rows = (rows + IROWS) % IROWS; // (-IROWS, IROWS) -> [0, IROWS)

        Cursor(rows as usize, cols as usize)
    }
}

pub struct TextField {
    text: TextDisplay,
    cursor: Cursor,
}

impl TextField {
    pub fn new() -> TextField {
        TextField {
            text: TextDisplay::new(),
            cursor: Cursor::default(),
        }
    }

    pub fn type_next(&mut self, t: char) {
        let icursor = (self.cursor.0 as isize, self.cursor.1 as isize);

        self.cursor = match t {
            '\u{08}' | '\u{7f}' => {
                // backspace
                let cur = self.cursor.offset(0, -1);
                self.text.write(cur.0, cur.1, ' ');
                cur
            }
            // these two don't work so hot yet, because of terminal <-> serial interaction reasons
            // '\r' => self.cursor.offset(0, -icursor.1),
            // '\n' => self.cursor.offset(1, -icursor.1),
            // taken from char::escape_default (below)
            '\\' | '\'' | '"' => {
                self.text.write(self.cursor.0, self.cursor.1, t);
                self.cursor.offset(0, 1)
            }
            _ => {
                for c in t.escape_default() {
                    self.text.write(self.cursor.0, self.cursor.1, c);
                    self.cursor = self.cursor.offset(0, 1);
                }

                self.cursor
            }
        };
    }

    pub fn draw<D>(&mut self, target: &mut D)
    where
        D: DrawTarget<Color = Rgb3>,
    {
        self.text.draw_dirty(target);
    }
}
