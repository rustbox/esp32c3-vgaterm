use crate::{
    color::Rgb3,
    display::{self, Character, Inverse, TextDisplay},
};
use embedded_graphics::prelude::DrawTarget;
use esp32c3_hal::systimer::SystemTimer;
use esp_println::println;

pub const IROWS: isize = display::ROWS as isize;
pub const ICOLS: isize = display::COLUMNS as isize;

pub type Row = usize;
pub type Col = usize;

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy)]
pub struct CursorPos(pub Row, pub Col);

impl CursorPos {
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
    pub fn offset(&self, r: isize, c: isize) -> CursorPos {
        let cols = self.1 as isize + c;
        let (p, cols) = (cols.div_euclid(ICOLS), cols.rem_euclid(ICOLS));
        let rows = (self.0 as isize + r + p) % IROWS;
        let rows = (rows + IROWS) % IROWS; // (-IROWS, IROWS) -> [0, IROWS)

        CursorPos(rows as usize, cols as usize)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Cursor {
    pub pos: CursorPos,
    pub character: Character,
    pub changed: bool,
    time_to_next_blink: u64,
    blink_length: u64,
}

impl Cursor {
    fn offset(&mut self, r: isize, c: isize) -> CursorPos {
        let pos = self.pos.offset(r, c);
        if pos != self.pos {
            self.changed = true;
            self.set_inverted();
            self.pos = pos;
        }
        pos
    }

    fn set_inverted(&mut self) {
        self.character.color.with_decoration(
            Some(Inverse),
            self.character.color.underline(),
            self.character.color.strikethrough(),
            self.character.color.blink(),
        );
        // Reset blink timer while we're typing
        self.time_to_next_blink = SystemTimer::now().wrapping_add(self.blink_length);
    }

    fn swap_invert(&mut self) {
        self.character.color.invert_fore_back();
        self.changed = true;
    }

    fn set_char(&mut self, c: char) {
        self.character.with_char(c);
        self.changed = true;
    }

    fn draw<D>(&mut self, target: &mut D, text: &TextDisplay)
    where
        D: DrawTarget<Color = Rgb3>,
    {
        let now = SystemTimer::now();
        if now >= self.time_to_next_blink {
            self.time_to_next_blink = now.wrapping_add(self.blink_length);
            self.swap_invert();
        }

        if self.changed {
            // let (t, s) = self.character.text_and_style();
            // println!("Drawing cursor: fore: {:?}, back: {:?}", s.text_color, s.background_color);
            // println!("{:?}", t);
            text.draw_character(self.pos.0, self.pos.1, self.character, target);
            self.changed = false;
        }
    }
}

impl Default for Cursor {
    fn default() -> Self {
        let mut c = Cursor {
            pos: Default::default(),
            character: Character::default(),
            changed: true,
            time_to_next_blink: SystemTimer::now().wrapping_add(8_000_000),
            blink_length: 12_000_000,
        };
        c.swap_invert();
        c
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

    /// Moves the cursor by the given offset, and sets the cursor character to character
    /// currently being selected by the new cursor position
    pub fn move_cursor(&mut self, r: isize, c: isize) {
        // self.text.write(self.cursor.pos.0, self.cursor.pos.1, self.cursor.character.char());
        let moved = self.cursor.offset(r, c);
        println!("Cursor moving to ({}, {})", r, c);
        let c = self.text.read_char(moved.0, moved.1).char();
        self.cursor.set_char(c);
    }

    pub fn type_next(&mut self, t: char) {
        #[allow(unused)] // used for \r and \n below
        let icursor = (self.cursor.pos.0 as isize, self.cursor.pos.1 as isize);

        match t {
            '\u{08}' | '\u{7f}' => {
                // backspace
                let curs_char = self.cursor.character.char();
                self.text
                    .write(self.cursor.pos.0, self.cursor.pos.1, curs_char);
                self.move_cursor(0, -1);
                // self.cursor.set_char(' ');
                self.text.write(self.cursor.pos.0, self.cursor.pos.1, ' ');
            }

            // these two don't work so hot yet, because of terminal <-> serial interaction reasons
            // '\r' => self.cursor.offset(0, -icursor.1),
            // '\n' => self.cursor.offset(1, -icursor.1),

            // taken from char::escape_default (below)
            '\\' | '\'' | '"' => {
                self.text.write(self.cursor.pos.0, self.cursor.pos.1, t);
                self.move_cursor(0, 1);
            }
            _ => {
                for c in t.escape_default() {
                    self.text.write(self.cursor.pos.0, self.cursor.pos.1, c);
                    self.move_cursor(0, 1);
                }
            }
        };
    }

    pub fn draw<D>(&mut self, target: &mut D)
    where
        D: DrawTarget<Color = Rgb3>,
    {
        self.text.draw_dirty(target);
        self.cursor.draw(target, &self.text);
    }
}

impl Default for TextField {
    fn default() -> Self {
        Self::new()
    }
}
