//! Escapes:
//! in the grammar, <x> will denote a variable.
//! ==============================
//! ESC [ H             => Cursor to (0, 0)
//! ESC [ <r> ; <c> H   => Cursor to line r, column c
//! ESC [ <r> ; <c> f   => Cursor to line r, column c
//! ESC [ <n> A         => Cursor up n lines
//! ESC [ <n> B         => Cursor down n lines
//! ESC [ <n> C         => Cursor right n columns
//! ESC [ <n> D         => Cursor left n columns
//! ESC [ <n> E         => Cursor to beginning of next line, n lines down
//! ESC [ <n> F         => Cursor to beginning of prev line, n lines up
//! ESC [ <n> G         => Cursor to column n
//! ESC [ 6 n           => Request cursor postion, as `ESC [ <r> ; <c> R` at row r and column c
//! ESC 7               => Save cursor position
//! ESC 8               => Restore cursor position
//! ESC [ J             => Erase from cursor until end of screen
//! ESC [ 0 J           => Erase from cursor until end of screen
//! ESC [ 1 J           => Erase from cursor to beginning of screen
//! ESC [ 2 J           => Erase entire screen
//! ESC [ K             => Erase from cursor until end of line
//! ESC [ 0 K           => Erase start of line to cursor
//! ESC [ 1 K           => Erase start of line to the cursor
//! ESC [ 2 K           => Erase entire line
//! 
//! Graphics/Colors
//! ===============
//! ESC [ <fg>;<bg>; m  => Set fg color between 30-37; 90-97. bg color between 40-47, 100-107
//! ESC [ <fg>;<bg>; m => Set fg/bg colors to "bold" or "bright"
//! ESC [ 38; 5; <c> m  => Set fg color to c where c is a color index of 256 colors
//! ESC [ 48; 5; <c> m  => Set bg color to c where c is a color index of 256 colors
//! ESC [ 0 m           => Reset all colors to "default"
//! ESC [ 1 m           => Set "bold" mode (perhaps use the "bright" set of colors)
//! ESC [ 2 m           => Set "dim" mode
//! ESC [ 22 m          => Reset "dim" or "bold" mode
//! ESC [ 3 m           => set italic mode
//! ESC [ 23 m          => Unset italic mode
//! ESC [ 4 m           => set underline mode
//! ESC [ 24 m          => unset underline mode
//! ESC [ 5 m           => set blinking mode
//! ESC [ 25 m          => unset blinking mode
//! ESC [ 7 m           => set inverse mode
//! ESC [ 27 m          => unset inverse mode
//! ESC [ 9 m           => set strikethrough
//! ESC [ 29 m          => unset strikethrough
//! 
//! 
//! [Op(name), [Param(value)]]
//! 


struct Color {
    fg: u8,
    bg: u8
}

enum TextMode {
    Bright,
    Dim,
    Italic,
    Underline,
    Blink,
    Inverse,
    Strikethrough,
}
