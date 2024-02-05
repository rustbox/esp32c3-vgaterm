use crate::{
    ansi::{self, EraseMode, Op, OpStr, Style, SetUnset},
    color::{Rgb3, self},
    display::{self, TextDisplay, ROWS, COLUMNS, Decoration},
    CHARACTER_DRAW_CYCLES,
};
use alloc::string::String;
use embedded_graphics::prelude::DrawTarget;
use esp32c3_hal::systimer::SystemTimer;

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

enum VerticalLocation {
    Middle,
    Top,
    Bottom
}

enum HorizontalLocation {
    Middle,
    Left,
    Right
}

#[derive(Debug, Clone, Copy)]
pub struct Cursor {
    pub pos: CursorPos,
    time_to_next_blink: u64,
    blink_length: u64,
    visible: bool,
}

impl Cursor {
    /// To move the cursor:
    /// 1. Reset the char at (r, c) to not inverted
    /// 2. Move the cursor
    /// 3. Set character at new position to be inverted
    /// 4. Update time_to_next_blink
    fn offset(&self, r: isize, c: isize, text: &mut TextDisplay) -> Cursor {
        let pos = self.pos.offset(r, c);
        if pos != self.pos {
            self.unset_highlight(text);
            let cursor = Cursor {
                pos,
                time_to_next_blink: SystemTimer::now().wrapping_add(self.blink_length),
                blink_length: self.blink_length,
                visible: self.visible
            };
            cursor.set_highlight(text);
            return cursor;
        }
        *self
    }

    fn location(&self) -> (VerticalLocation, HorizontalLocation) {
        const BOT: usize = ROWS - 1;
        const RIGHT: usize = COLUMNS - 1;
        let vert = match self.pos.row() {
            BOT => VerticalLocation::Bottom,
            0 => VerticalLocation::Top,
            _ => VerticalLocation::Middle,
        };
        let horz = match self.pos.col() {
            RIGHT => HorizontalLocation::Right,
            0 => HorizontalLocation::Left,
            _ => HorizontalLocation::Middle,
        };
        (vert, horz)
    }

    fn set_highlight(&self, text: &mut TextDisplay) {
        let mut c = text.read_char(self.pos.row(), self.pos.col());
        if self.visible {
            c.color.set_inverted();
        }
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

    fn reset_highlight_timer(&self, text: &mut TextDisplay) -> Cursor {
        self.set_highlight(text);
        let time_to_next_blink = SystemTimer::now().wrapping_add(self.blink_length);
        Cursor {
            pos: self.pos,
            time_to_next_blink,
            blink_length: self.blink_length,
            visible: self.visible
        }
    }

    fn update(&self, text: &mut TextDisplay) -> Cursor {
        let now = SystemTimer::now();
        if now >= self.time_to_next_blink {
            if self.visible {
                self.swap_highlight(text);
            }
            let time_to_next_blink = now.wrapping_add(self.blink_length);
            return Cursor {
                pos: self.pos,
                blink_length: self.blink_length,
                time_to_next_blink,
                visible: self.visible
            };
        }
        *self
    }
}

impl Default for Cursor {
    fn default() -> Self {
        Cursor {
            pos: Default::default(),
            time_to_next_blink: SystemTimer::now(),
            blink_length: 12_000_000,
            visible: true
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
        self.cursor = self.cursor.offset(r, c, &mut self.text);
    }

    pub fn type_str(&mut self, s: &str) {
        self.input_buffer.push_str(s);
        let res = ansi::parse_esc_str(self.input_buffer.as_str());
        let len = res.rest.len();

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
        let _ = self.input_buffer.drain(..(self.input_buffer.len() - len));
    }

    fn handle_char_in(&mut self, t: char) {
        if t.is_ascii_control() {
            // println!("ascii {}", t.escape_debug());
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
                match self.cursor.location() {
                    (VerticalLocation::Bottom, _) => {
                        self.cursor.unset_highlight(&mut self.text);
                        self.text.scroll_down(1);
                        self.move_cursor(0, -(self.cursor.pos.col() as isize))
                    },
                    _ => {
                        self.move_cursor(1, -(self.cursor.pos.col() as isize))
                    }
                }
            }
            '\r' => self.move_cursor(0, -(self.cursor.pos.col() as isize)),
            _ => {
                for c in t.escape_default() {
                    self.text.write(self.cursor.pos.0, self.cursor.pos.1, c);
                    match self.cursor.location() {
                        (_, HorizontalLocation::Left | HorizontalLocation::Middle) => {
                            self.move_cursor(0, 1);
                        },
                        (VerticalLocation::Top | VerticalLocation::Middle, HorizontalLocation::Right) => {
                            self.move_cursor(1, -(self.cursor.pos.col() as isize))
                        },
                        (VerticalLocation::Bottom, HorizontalLocation::Right) => {
                            self.cursor.unset_highlight(&mut self.text);
                            self.text.scroll_down(1);
                            self.cursor.set_highlight(&mut self.text);
                            self.move_cursor(0, -(self.cursor.pos.col() as isize));
                        }
                    }
                }
            }
        }
    }

    fn handle_op(&mut self, op: Op) {
        use Op::*;
        // println!("{:?}", op);
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
                // Constrain dx and dy so that the result added to the current position
                // stays within the window
                let x = (self.cursor.pos.col() as isize + dx).clamp(0, COLUMNS as isize - 1) - self.cursor.pos.col() as isize;
                let y = (self.cursor.pos.row() as isize + dy).clamp(0, ROWS as isize - 1) - self.cursor.pos.row() as isize;
                self.move_cursor(y, x);
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
                    self.cursor = self.cursor.reset_highlight_timer(&mut self.text);
                    for c in 0..display::COLUMNS {
                        self.text.write(self.cursor.pos.row(), c, ' ');
                    }
                }
                EraseMode::FromCursor => {
                    self.cursor = self.cursor.reset_highlight_timer(&mut self.text);
                    for c in self.cursor.pos.col()..display::COLUMNS {
                        self.text.write(self.cursor.pos.row(), c, ' ');
                        self.cursor.update(&mut self.text);
                    }
                }
                EraseMode::ToCursor => {
                    self.cursor = self.cursor.reset_highlight_timer(&mut self.text);
                    for c in 0..self.cursor.pos.col() {
                        self.text.write(self.cursor.pos.row(), c, ' ');
                        self.cursor.update(&mut self.text);
                    }
                }
            },
            Scroll { delta } => {
                self.cursor.unset_highlight(&mut self.text);
                self.text.scroll_down(delta);
            }
            TextOp(ops) => {
                for op in ops {
                    match op {
                        ansi::TextOp::SetFGBasic { fg } => {
                            let (f, _) = color::ansi_base_color(fg, 0);
                            self.text.current_color.fore = f;
                        },
                        ansi::TextOp::SetBGBasic { bg } => {
                            let (_, b) = color::ansi_base_color(0, bg);
                            self.text.current_color.back = b;
                        },
                        ansi::TextOp::SetTextMode(s, style) => {
                            match (s, style) {
                                (SetUnset::Set, Style::Inverse) => {
                                    if !self.text.current_color.decs.contains(&Decoration::Inverse) {
                                        self.text.current_color.decs.push(Decoration::Inverse);
                                    }
                                },
                                (SetUnset::Unset, Style::Inverse) => {
                                    if let Some((i, _)) = self.text.current_color.decs.iter().enumerate().find(|(_, d)| *d == &Decoration::Inverse) {
                                        self.text.current_color.decs.remove(i);
                                    }
                                }
                                (SetUnset::Set, Style::Strike) => {
                                    if !self.text.current_color.decs.contains(&Decoration::Strikethrough) {
                                        self.text.current_color.decs.push(Decoration::Strikethrough);
                                    }
                                },
                                (SetUnset::Unset, Style::Strike) => {
                                    if let Some((i, _)) = self.text.current_color.decs.iter().enumerate().find(|(_, d)| *d == &Decoration::Strikethrough) {
                                        self.text.current_color.decs.remove(i);
                                    }
                                },
                                (SetUnset::Set, Style::Blinking) => {
                                    if !self.text.current_color.decs.contains(&Decoration::Blink) {
                                        self.text.current_color.decs.push(Decoration::Blink);
                                    }
                                },
                                (SetUnset::Unset, Style::Blinking) => {
                                    if let Some((i, _)) = self.text.current_color.decs.iter().enumerate().find(|(_, d)| *d == &Decoration::Blink) {
                                        self.text.current_color.decs.remove(i);
                                    }
                                },
                                (SetUnset::Set, Style::Underline | Style::Italic) => {
                                    if !self.text.current_color.decs.contains(&Decoration::Underline) {
                                        self.text.current_color.decs.push(Decoration::Underline);
                                    }
                                },
                                (SetUnset::Unset, Style::Underline | Style::Italic) => {
                                    if let Some((i, _)) = self.text.current_color.decs.iter().enumerate().find(|(_, d)| *d == &Decoration::Underline) {
                                        self.text.current_color.decs.remove(i);
                                    }
                                },
                                (SetUnset::Set | SetUnset::Unset, Style::Bold | Style::Dim) => {
                                    // Not implemented yet, do nothing
                                },
                            }
                        },
                        ansi::TextOp::SetFGColor256 { fg } => {
                            let f = color::ansi_256_color(fg);
                            self.text.current_color.fore = f;
                        },
                        ansi::TextOp::SetBGColor256 { bg } => {
                            let b = color::ansi_256_color(bg);
                            self.text.current_color.fore = b;
                        },
                        ansi::TextOp::ResetColors => {
                            // Turn off attributes
                            self.text.current_color.decs.clear();
                        }
                    }
                }
            }
            InPlaceDelete => self.text.write(self.cursor.pos.0, self.cursor.pos.1, ' '),
            DecPrivateSet(_) => {}
            DecPrivateReset(_) => {}
            Vgaterm(ansi::Vgaterm::Redraw) => {
                self.text.dirty_all();
                unsafe {
                    CHARACTER_DRAW_CYCLES = 0;
                    crate::perf::reset_cycle_count();
                }
            }
        }
    }

    pub fn draw<D>(&mut self, target: &mut D)
    where
        D: DrawTarget<Color = Rgb3>,
    {
        self.text.draw_dirty(target);
        self.cursor = self.cursor.update(&mut self.text);
    }

    pub fn draw_up_to<D>(&mut self, up_to: usize, target: &mut D)
    where
        D: DrawTarget<Color = Rgb3>,
    {
        self.text.draw_dirty_up_to(up_to, target);
        self.cursor = self.cursor.update(&mut self.text);
    }
}

impl Default for TextField {
    fn default() -> Self {
        Self::new()
    }
}
