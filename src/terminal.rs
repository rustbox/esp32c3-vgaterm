use crate::{
    ansi::{self, EraseMode, Op, OpStr},
    color::Rgb3,
    display::{self, Character, TextDisplay, ROWS},
};
use alloc::string::String;
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
    #[inline]
    pub fn offset(&self, r: isize, c: isize) -> CursorPos {
        let cols = self.1 as isize + c;
        let (p, cols) = (cols.div_euclid(ICOLS), cols.rem_euclid(ICOLS));
        let rows = (self.0 as isize + r + p) % IROWS;
        let rows = (rows + IROWS) % IROWS; // (-IROWS, IROWS) -> [0, IROWS)

        CursorPos(rows as usize, cols as usize)
    }

    #[inline]
    pub fn row(&self) -> Row {
        self.0
    }

    #[inline]
    pub fn col(&self) -> Col {
        self.1
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Cursor {
    pub pos: CursorPos,
    pub character: Character,
    time_to_next_blink: u64,
    blink_length: u64,
}

impl Cursor {
    /// To move the cursor:
    /// 1. Reset the char at (r, c) to not inverted
    /// 2. Move the cursor
    /// 3. Set character at new position to be inverted
    /// 4. Update time_to_next_blink
    fn offset(&mut self, r: isize, c: isize, text: &mut TextDisplay) -> CursorPos {
        let pos = self.pos.offset(r, c);
        if pos != self.pos {
            self.unset_highlight(text);
            self.pos = pos;
            self.set_highlight(text);
            self.time_to_next_blink = SystemTimer::now().wrapping_add(self.blink_length);
        }
        pos
    }

    fn is_at_bottom(&self) -> bool {
        self.pos.row() == ROWS - 1
    }

    fn set_highlight(&self, text: &mut TextDisplay) {
        let mut c = text.read_char(self.pos.row(), self.pos.col());
        c.color.set_inverted();
        text.write_char(self.pos.row(), self.pos.col(), c);
    }

    fn unset_highlight(&self, text: &mut TextDisplay) {
        let mut c = text.read_char(self.pos.row(), self.pos.col());
        c.color.reset_inverted();
        text.write_char(self.pos.row(), self.pos.col(), c);
    }

    fn swap_highlight(&self, text: &mut TextDisplay) {
        let mut c = text.read_char(self.pos.row(), self.pos.col());
        c.color.invert_colors();
        text.write_char(self.pos.row(), self.pos.col(), c);
    }

    fn reset_highlight_timer(&mut self, text: &mut TextDisplay) {
        self.set_highlight(text);
        self.time_to_next_blink = SystemTimer::now().wrapping_add(self.blink_length);
    }

    fn update(&mut self, text: &mut TextDisplay) {
        let now = SystemTimer::now();
        if now >= self.time_to_next_blink {
            self.time_to_next_blink = now.wrapping_add(self.blink_length);
            self.swap_highlight(text);
        }
    }
}

impl Default for Cursor {
    fn default() -> Self {
        Cursor {
            pos: Default::default(),
            character: Character::default(),
            time_to_next_blink: SystemTimer::now(),
            blink_length: 12_000_000,
        }
    }
}

pub struct TextField {
    pub text: TextDisplay,
    cursor: Cursor,
    input_buffer: String,
}

impl TextField {
    pub fn new() -> TextField {
        TextField {
            text: TextDisplay::new(),
            cursor: Cursor::default(),
            input_buffer: String::default(),
        }
    }

    /// Moves the cursor by the given offset, and sets the cursor character to character
    /// currently being selected by the new cursor position
    pub fn move_cursor(&mut self, r: isize, c: isize) {
        self.cursor.offset(r, c, &mut self.text);
    }

    pub fn type_str(&mut self, s: &str) {
        self.input_buffer.push_str(s);
        let drained: String = self.input_buffer.drain(..).collect();
        let res = ansi::parse_esc_str(drained.as_str());

        // At the end buffer should have only the contents of res.rest
        for op in res.opstr {
            match op {
                OpStr::Str(s) => {
                    for ch in s.chars() {
                        self.handle_char_in(ch);
                    }
                }
                OpStr::Op(op) => {
                    self.handle_op(op);
                }
            }
        }
        self.input_buffer.push_str(res.rest);
    }

    fn handle_char_in(&mut self, t: char) {
        if t.is_ascii_control() {
            println!("ascii {}", t.escape_debug());
        }
        match t {
            '\u{08}' => {
                // backspace
                // 1. Move cursor back 1
                // 2. Write a space over the new existing character
                self.move_cursor(0, -1);
                self.text
                    .write(self.cursor.pos.row(), self.cursor.pos.col(), ' ');
            }
            '\u{07}' => {
                // Bell not impl
            }

            '\u{7f}' => {
                // Del not impl
            }

            '\\' | '\'' | '"' => {
                self.text
                    .write(self.cursor.pos.row(), self.cursor.pos.col(), t);
                self.move_cursor(0, 1);
            }
            '\n' => {
                if self.cursor.is_at_bottom() {
                    self.cursor.unset_highlight(&mut self.text);
                    self.text.scroll_down(1);
                    self.move_cursor(0, -(self.cursor.pos.col() as isize))
                } else {
                    self.move_cursor(1, -(self.cursor.pos.col() as isize))
                }
            },
            '\r' => self.move_cursor(0, -(self.cursor.pos.col() as isize)),
            _ => {
                for c in t.escape_default() {
                    self.text.write(self.cursor.pos.0, self.cursor.pos.1, c);
                    self.move_cursor(0, 1);
                }
            }
        }
    }

    fn handle_op(&mut self, op: Op) {
        use Op::*;
        println!("{:?}", op);
        match op {
            MoveCursorAbs { x, y } => {
                self.move_cursor(
                    y as isize - self.cursor.pos.row() as isize,
                    x as isize - self.cursor.pos.col() as isize,
                );
            }
            MoveCursorAbsCol { x } => {
                self.move_cursor(0, x as isize - self.cursor.pos.col() as isize);
            }
            MoveCursorDelta { dx, dy } => {
                self.move_cursor(dy, dx);
            }
            MoveCursorBeginningAndLine { dy } => {
                self.move_cursor(dy, -(self.cursor.pos.col() as isize));
            }
            RequstCursorPos => {}
            SaveCursorPos => {}
            RestoreCursorPos => {}
            EraseScreen(erase) => {
                match erase {
                    EraseMode::All => {
                        self.text.clear();
                    }
                    EraseMode::FromCursor => {
                        // Line the cursor is on
                        for c in self.cursor.pos.col()..display::COLUMNS {
                            self.text.write(self.cursor.pos.row(), c, ' ');
                        }
                        // Rest of the screen
                        for r in self.cursor.pos.row()..display::ROWS {
                            for c in 0..display::COLUMNS {
                                self.text.write(r, c, ' ');
                            }
                        }
                    }
                    EraseMode::ToCursor => {
                        // All lines up to the cursor
                        for r in 0..self.cursor.pos.row() {
                            for c in 0..display::COLUMNS {
                                self.text.write(r, c, ' ');
                            }
                        }
                        // Characters up to the cursor
                        for c in 0..self.cursor.pos.col() {
                            self.text.write(self.cursor.pos.row(), c, ' ');
                        }
                    }
                }
            }
            EraseLine(erase) => match erase {
                EraseMode::All => {
                    self.cursor.reset_highlight_timer(&mut self.text);
                    for c in 0..display::COLUMNS {
                        self.text.write(self.cursor.pos.row(), c, ' ');
                    }
                }
                EraseMode::FromCursor => {
                    self.cursor.reset_highlight_timer(&mut self.text);
                    for c in self.cursor.pos.col()..display::COLUMNS {
                        self.text.write(self.cursor.pos.row(), c, ' ');
                        self.cursor.update(&mut self.text);
                    }
                }
                EraseMode::ToCursor => {
                    self.cursor.reset_highlight_timer(&mut self.text);
                    for c in 0..self.cursor.pos.col() {
                        self.text.write(self.cursor.pos.row(), c, ' ');
                        self.cursor.update(&mut self.text);
                    }
                }
            },
            Scroll { delta } => {
                if delta > 0 {
                    // self.cursor.offset(0, 1, &mut self.text);
                    self.cursor.unset_highlight(&mut self.text);
                    self.text.scroll_down(delta as usize);
                    // self.cursor.set_highlight(&mut self.text);
                }
            }
            TextOp(_ops) => {}
            InPlaceDelete => self.text.write(self.cursor.pos.0, self.cursor.pos.1, ' '),
            DecPrivateSet(_) => {}
            DecPrivateReset(_) => {}
        }
    }

    pub fn draw<D>(&mut self, target: &mut D)
    where
        D: DrawTarget<Color = Rgb3>,
    {
        self.text.draw_dirty(target);
        self.cursor.update(&mut self.text);
    }

    pub fn draw_up_to<D>(&mut self, up_to: usize, target: &mut D)
    where
        D: DrawTarget<Color = Rgb3>,
    {
        self.text.draw_dirty_up_to(up_to, target);
        self.cursor.update(&mut self.text);
    }
}

impl Default for TextField {
    fn default() -> Self {
        Self::new()
    }
}
