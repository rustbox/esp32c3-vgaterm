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
//! ESC [ <n> S         => Scroll up n lines
//! ESC [ <n> T         => Scroll down n lines
//! ESC [ 6 n           => Request cursor position, as `ESC [ <r> ; <c> R` at row r and column c
//! ESC 7               => Save cursor position
//! ESC 8               => Restore cursor position
//! ESC [ 3 > ~         => Delete
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

use alloc::{
    borrow::ToOwned,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use core::{fmt::Debug, str::FromStr};
use esp_println::println;
use nom::{combinator::fail, error::context, IResult, Parser};

const ESC: char = '\u{1B}';

#[derive(Debug, PartialEq, Eq)]
pub enum Op {
    MoveCursorDelta { dx: isize, dy: isize },
    MoveCursorAbs { x: usize, y: usize },
    MoveCursorAbsCol { x: usize },
    MoveCursorBeginningAndLine { dy: isize },
    Scroll { delta: isize },
    RequestCursorPos,
    SaveCursorPos,
    RestoreCursorPos,
    EraseScreen(EraseMode),
    EraseLine(EraseMode),
    TextOp(Vec<TextOp>),
    InPlaceDelete,
    DecPrivateSet(String),
    DecPrivateReset(String),
    Vgaterm(Vgaterm),
}

#[derive(Debug, PartialEq, Eq)]
pub enum TextOp {
    SetBGBasic { bg: u8 },
    SetFGBasic { fg: u8 },
    SetFGColor256 { fg: u8 },
    SetBGColor256 { bg: u8 },
    ResetColors,
    SetTextMode(SetUnset, Style),
}

#[derive(Debug, PartialEq, Eq)]
pub enum Style {
    Bold,
    Dim,
    Italic,
    Strike,
    Underline,
    Blinking,
    Inverse,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SetUnset {
    Set,
    Unset,
}

#[derive(Debug, PartialEq, Eq)]
pub enum EraseMode {
    FromCursor,
    ToCursor,
    All,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Vgaterm {
    Redraw,
}

trait StrParser<'a, O> = nom::Parser<&'a str, O, nom::error::Error<&'a str>>;

type OpResult<'a> = IResult<&'a str, Op>;

type TextOpResult<'a> = IResult<&'a str, TextOp>;

trait StrParseFnMut<'a, O> = FnMut(&'a str) -> IResult<&'a str, O>;

fn start_with_char<'a, O, P: StrParser<'a, O>>(start: char, parser: P) -> impl StrParser<'a, O> {
    nom::sequence::preceded(nom::character::streaming::char(start), parser)
}

/// Recognize ESC, and then parses via the P parser. If P fails, this parser will return
/// the Failure variant (by using nom `cut`). If the this parser does not recognize ESC
/// it will return with the nom Error variant.
fn start_with_esc<'a, O, P: StrParser<'a, O>>(parser: P) -> impl StrParser<'a, O> {
    start_with_char(ESC, nom::combinator::cut(parser))
}

// This will parse "...P... <ending>" for some char ending and parsed sequence P
fn sequence_with_ending<'a, O, P: StrParser<'a, O>>(
    mut parser: P,
    ending: char,
) -> impl StrParseFnMut<'a, O> {
    move |input: &'a str| {
        nom::sequence::terminated(
            |x: &'a str| parser.parse(x),
            nom::character::streaming::char(ending),
        )(input)
    }
}

/// This will parse <n> and return n
fn single_int_parameter_atom<N: FromStr>() -> impl FnMut(&str) -> IResult<&str, N>
where
    <N as FromStr>::Err: Debug,
{
    move |input: &str| {
        nom::character::streaming::digit1(input).map(|(rest, n)| (rest, N::from_str(n).unwrap()))
    }
}

// This will parse ''|<n> <ending> and return n, with a default
fn optional_int_param_sequence<N: FromStr + Copy>(
    ending: char,
    default: N,
) -> impl FnMut(&str) -> IResult<&str, N>
where
    <N as FromStr>::Err: Debug,
{
    move |input: &str| {
        sequence_with_ending(
            nom::combinator::opt(nom::character::streaming::digit1),
            ending,
        )(input)
        .map(|(rest, n)| {
            (
                rest,
                match n {
                    None => default,
                    Some(n) => N::from_str(n).unwrap(),
                },
            )
        })
    }
}

/// This will parse x ; y <end>
fn dual_int_parameter_sequence<N: FromStr>(
    ending: char,
) -> impl FnMut(&str) -> IResult<&str, (N, N)>
where
    <N as FromStr>::Err: Debug,
{
    move |input: &str| {
        nom::sequence::terminated(
            nom::sequence::separated_pair(
                nom::character::streaming::digit1,
                nom::character::streaming::char(';'),
                nom::character::streaming::digit1,
            ),
            nom::character::streaming::char(ending),
        )(input)
        .map(|(rest, (a, b))| (rest, (N::from_str(a).unwrap(), N::from_str(b).unwrap())))
    }
}

fn cursor_to_0(input: &str) -> OpResult {
    nom::character::streaming::char('H')(input)
        .map(|(rest, _)| (rest, Op::MoveCursorAbs { x: 0, y: 0 }))
}

fn cursor_to_line_col(input: &str) -> OpResult {
    // Recognize <digits> `;` <digits>
    // Start with CSI and end with H or f
    nom::branch::alt((
        dual_int_parameter_sequence::<usize>('H'),
        dual_int_parameter_sequence::<usize>('f'),
    ))(input)
    .map(|(rest, (a, b))| {
        (
            rest,
            Op::MoveCursorAbs {
                x: b.saturating_sub(1),
                y: a.saturating_sub(1),
            },
        )
    })
}

/// ESC [ <n> A         => Cursor up n lines
fn cursor_up_lines(input: &str) -> OpResult {
    optional_int_param_sequence::<isize>('A', 1)(input)
        .map(|(rest, n)| (rest, Op::MoveCursorDelta { dx: 0, dy: -n }))
}

/// ESC [ <n> B
fn cursor_down_lines(input: &str) -> OpResult {
    optional_int_param_sequence::<isize>('B', 1)(input)
        .map(|(rest, n)| (rest, Op::MoveCursorDelta { dx: 0, dy: n }))
}

// ESC [ <n> C         => Cursor right n columns
fn cursor_right_col(input: &str) -> OpResult {
    optional_int_param_sequence::<isize>('C', 1)(input)
        .map(|(rest, n)| (rest, Op::MoveCursorDelta { dx: n, dy: 0 }))
}

// ESC [ <n> D         => Cursor left n columns
fn cursor_left_col(input: &str) -> OpResult {
    optional_int_param_sequence::<isize>('D', 1)(input)
        .map(|(rest, n)| (rest, Op::MoveCursorDelta { dx: -n, dy: 0 }))
}

// ESC [ <n> E         => Cursor to beginning of next line, n lines down
fn cursor_beginning_down(input: &str) -> OpResult {
    optional_int_param_sequence::<isize>('E', 1)(input)
        .map(|(rest, n)| (rest, Op::MoveCursorBeginningAndLine { dy: n }))
}

// ESC [ <n> F         => Cursor to beginning of prev line, n lines up
fn cursor_beginning_up(input: &str) -> OpResult {
    optional_int_param_sequence::<isize>('F', 1)(input)
        .map(|(rest, n)| (rest, Op::MoveCursorBeginningAndLine { dy: -n }))
}

// ESC [ <n> G         => Cursor to column n
fn cursor_to_column(input: &str) -> OpResult {
    optional_int_param_sequence::<usize>('G', 0)(input).map(|(rest, n)| {
        (
            rest,
            Op::MoveCursorAbsCol {
                x: n.saturating_sub(1),
            },
        )
    })
}

/// ESC [ n S
fn scroll_up(input: &str) -> OpResult {
    optional_int_param_sequence::<isize>('S', 1)(input)
        .map(|(rest, n)| (rest, Op::Scroll { delta: -n }))
}

/// ESC [ n T
fn scroll_down(input: &str) -> OpResult {
    optional_int_param_sequence::<isize>('T', 1)(input)
        .map(|(rest, n)| (rest, Op::Scroll { delta: n }))
}

// Request Cursor Position
// ESC [ 6 n
fn request_cursor_position(input: &str) -> OpResult {
    sequence_with_ending(nom::character::streaming::char('6'), 'n')(input)
        .map(|(rest, _)| (rest, Op::RequestCursorPos))
}

// ESC 7               => Save cursor position
fn save_cursor_position(input: &str) -> OpResult {
    nom::bytes::streaming::tag("\u{1B}7")(input).map(|(rest, _)| (rest, Op::SaveCursorPos))
}

// ESC 8               => Restore cursor position
fn restore_cursor_position(input: &str) -> OpResult {
    nom::bytes::streaming::tag("\u{1B}8")(input).map(|(rest, _)| (rest, Op::RestoreCursorPos))
}

/// ESC [ 3 > ~         => Delete
fn delete(input: &str) -> OpResult {
    nom::bytes::streaming::tag("3~")(input).map(|(rest, _)| (rest, Op::InPlaceDelete))
}

// ESC [ J             => Erase from cursor until end of screen
// ESC [ 0 J           => Erase from cursor until end of screen
// ESC [ 1 J           => Erase from cursor to beginning of screen
// ESC [ 2 J           => Erase entire screen
fn erase_screen(input: &str) -> OpResult {
    sequence_with_ending(
        nom::combinator::opt(nom::character::streaming::one_of("012")),
        'J',
    )(input)
    .map(|(rest, arg)| {
        (
            rest,
            match arg {
                None => Op::EraseScreen(EraseMode::FromCursor),
                Some('0') => Op::EraseScreen(EraseMode::FromCursor),
                Some('1') => Op::EraseScreen(EraseMode::ToCursor),
                Some('2') => Op::EraseScreen(EraseMode::All),
                Some(_) => unreachable!(),
            },
        )
    })
}

// ESC [ K             => Erase from cursor until end of line
// ESC [ 0 K           => Erase start of line to cursor
// ESC [ 1 K           => Erase start of line to the cursor
// ESC [ 2 K           => Erase entire line
fn erase_line(input: &str) -> OpResult {
    // nom::combinator::opt(arg)(input).map(|(rest, x)| (rest, ))
    sequence_with_ending(
        nom::combinator::opt(nom::character::streaming::one_of("012")),
        'K',
    )(input)
    .map(|(rest, arg)| {
        (
            rest,
            match arg {
                None => Op::EraseLine(EraseMode::FromCursor),
                Some('0') => Op::EraseLine(EraseMode::FromCursor),
                Some('1') => Op::EraseLine(EraseMode::ToCursor),
                Some('2') => Op::EraseLine(EraseMode::All),
                Some(_) => unreachable!(),
            },
        )
    })
}

//  <fg>;<bg>;   => Set fg color between 30-37; 90-97. bg color between 40-47, 100-107
fn set_basic_color_atom(input: &str) -> TextOpResult {
    let params = single_int_parameter_atom::<u8>();
    nom::combinator::map_opt(params, |a| match a {
        30..=37 => Some(TextOp::SetFGBasic { fg: a }),
        90..=97 => Some(TextOp::SetFGBasic { fg: a }),
        40..=47 => Some(TextOp::SetBGBasic { bg: a }),
        100..=107 => Some(TextOp::SetBGBasic { bg: a }),
        _ => None,
    })(input)
}

// 38; 5; <c> m  => Set fg color to c where c is a color index of 256 colors
fn set_fg_256_color_atom(input: &str) -> TextOpResult {
    nom::sequence::preceded(
        nom::bytes::streaming::tag("38;5;"),
        nom::character::streaming::u8,
    )(input)
    .map(|(rest, fg)| (rest, TextOp::SetFGColor256 { fg }))
}

// 48; 5; <c>  => Set bg color to c where c is a color index of 256 colors
fn set_bg_256_color_atom(input: &str) -> TextOpResult {
    nom::sequence::preceded(
        nom::bytes::streaming::tag("48;5;"),
        nom::character::streaming::u8,
    )(input)
    .map(|(rest, bg)| (rest, TextOp::SetBGColor256 { bg }))
}

/// ESC [ 0 m           => Reset all colors to "default"
/// ESC [ 1 m           => Set "bold" mode (perhaps use the "bright" set of colors)
/// ESC [ 2 m           => Set "dim" mode
/// ESC [ 22 m          => Reset "dim" or "bold" mode
/// ESC [ 3 m           => set italic mode
/// ESC [ 23 m          => Unset italic mode
/// ESC [ 4 m           => set underline mode
/// ESC [ 24 m          => unset underline mode
/// ESC [ 5 m           => set blinking mode
/// ESC [ 25 m          => unset blinking mode
/// ESC [ 7 m           => set inverse mode
/// ESC [ 27 m          => unset inverse mode
/// ESC [ 9 m           => set strikethrough
/// ESC [ 29 m          => unset strikethrough
///
fn set_text_mode_atom(input: &str) -> TextOpResult {
    nom::combinator::map_opt(single_int_parameter_atom(), |n| match n {
        0 => Some(TextOp::ResetColors),
        1 => Some(TextOp::SetTextMode(SetUnset::Set, Style::Bold)),
        2 => Some(TextOp::SetTextMode(SetUnset::Set, Style::Dim)),
        22 => Some(TextOp::SetTextMode(SetUnset::Unset, Style::Bold)),
        3 => Some(TextOp::SetTextMode(SetUnset::Set, Style::Italic)),
        23 => Some(TextOp::SetTextMode(SetUnset::Unset, Style::Italic)),
        4 => Some(TextOp::SetTextMode(SetUnset::Set, Style::Underline)),
        24 => Some(TextOp::SetTextMode(SetUnset::Unset, Style::Underline)),
        5 => Some(TextOp::SetTextMode(SetUnset::Set, Style::Blinking)),
        25 => Some(TextOp::SetTextMode(SetUnset::Unset, Style::Blinking)),
        7 => Some(TextOp::SetTextMode(SetUnset::Set, Style::Inverse)),
        27 => Some(TextOp::SetTextMode(SetUnset::Unset, Style::Inverse)),
        9 => Some(TextOp::SetTextMode(SetUnset::Set, Style::Strike)),
        29 => Some(TextOp::SetTextMode(SetUnset::Unset, Style::Strike)),
        _ => None,
    })(input)
}

/// ESC [ ? <numbers> h
fn set_private_sequence(input: &str) -> OpResult {
    nom::sequence::tuple((
        nom::character::streaming::char('?'),
        nom::character::streaming::digit0,
        nom::character::streaming::char('h'),
    ))(input)
    .map(|(rest, (_, b, _))| (rest, Op::DecPrivateSet(b.to_owned())))
}

/// ESC [ ? <numbers> l
fn reset_private_sequence(input: &str) -> OpResult {
    nom::sequence::tuple((
        nom::character::streaming::char('?'),
        nom::character::streaming::digit0,
        nom::character::streaming::char('l'),
    ))(input)
    .map(|(rest, (_, b, _))| (rest, Op::DecPrivateReset(b.to_owned())))
}

/// ESC [ V x D
fn vgaterm_sequence(input: &str) -> OpResult {
    nom::bytes::streaming::tag("VxD")(input).map(|(rest, _)| (rest, Op::Vgaterm(Vgaterm::Redraw)))
}

/// <text> m
fn any_text_mode(input: &str) -> TextOpResult {
    nom::branch::alt((
        set_basic_color_atom,
        set_bg_256_color_atom,
        set_fg_256_color_atom,
        set_text_mode_atom,
    ))(input)
}

fn set_text_mode(input: &str) -> OpResult {
    nom::sequence::terminated(
        nom::multi::separated_list0(nom::character::streaming::char(';'), any_text_mode),
        nom::character::streaming::char('m'),
    )(input)
    .map(|(rest, found)| (rest, Op::TextOp(found)))
}

pub fn parse_new(input: &str) -> OpResult {
    fn gen_parse<'a, 'str>(
        input: &'str str,
        q: &'a mut [&'str str; 4],
    ) -> IResult<&'str str, &'a [&'str str]> {
        let (input, start) =
            nom::combinator::recognize(nom::character::streaming::one_of("\u{1b}\u{9b}"))
                .parse(input)?;

        match context(
            "c0",
            nom::combinator::cond(
                start == "\x1b",
                nom::sequence::tuple((
                    nom::combinator::opt(nom::sequence::tuple((
                        nom::combinator::recognize(nom::character::complete::char('\x21')),
                        nom::combinator::recognize(nom::character::complete::char('\x40')),
                    ))),
                    nom::combinator::recognize(nom::character::streaming::satisfy(|ch| {
                        '\x00' < ch && ch < '\x1f'
                    })),
                )),
            ),
        )
        .parse(input)
        {
            // collapse the two intro sequences to one
            Ok((rest, Some((Some(_), n)))) | Ok((rest, Some((None, n)))) => {
                q[0] = start;
                q[1] = n;
                return Ok((rest, &q[..=1]));
            }
            Err(err @ nom::Err::Failure(_)) | Err(err @ nom::Err::Incomplete(_)) => {
                return Err(err)
            }
            // We didn't match a c0 sequence, nothing to return yet
            Err(nom::Err::Error(_)) | Ok((_, None)) => {}
        };

        // TODO: c1 set
        match context(
            // cf. https://github.com/fusesource/jansi/issues/226
            "vt100 (non-standard)",
            nom::combinator::cond(
                start == "\x1b",
                nom::combinator::recognize(nom::character::streaming::satisfy(|ch| {
                    ch == '7' || ch == '8'
                })),
            ),
        )
        .parse(input)
        {
            Ok((rest, Some(n))) => {
                q[0] = start;
                q[1] = n;
                return Ok((rest, &q[..=1]));
            }
            Err(err @ nom::Err::Failure(_)) | Err(err @ nom::Err::Incomplete(_)) => {
                return Err(err)
            }
            // We didn't match a non-standard VT100 sequence, nothing to return yet
            Err(nom::Err::Error(_)) | Ok((_, None)) => {}
        }

        match context(
            // catch-all
            "errybody else (non-standard)",
            nom::combinator::cond(
                start == "\x1b",
                // TODO: (can't do this right now because it prevents us from recognizing CSIs and would need to come "later")
                // nom::combinator::recognize(nom::character::streaming::anychar),
                nom::combinator::recognize(nom::character::streaming::none_of("[")),
            ),
        )
        .parse(input)
        {
            Ok((rest, Some(n))) => {
                q[0] = start;
                q[1] = n;
                return Ok((rest, &q[..=1]));
            }
            Err(err @ nom::Err::Failure(_)) | Err(err @ nom::Err::Incomplete(_)) => {
                return Err(err)
            }
            // We didn't match a non-standard VT100 sequence, nothing to return yet
            Err(nom::Err::Error(_)) | Ok((_, None)) => {}
        }

        // control sequences
        let (input, start) = if start == "\x1b" {
            let (input, _) = nom::character::streaming::char('[').parse(input)?;
            // map everything to this particular CSI
            (input, "\u{9b}")
        } else {
            (input, start)
        };

        // CSI P ... P I ... I F
        //
        // where
        //    P ... P are Parameter Bytes, which, if present, consist of bit combinations from 03/00 (\x30) to 03/15 (\x3f)
        //    I ... I are Intermediate Bytes, which, if present, consist of bit combinations from 02/00 (\x20) to 02/15 (\x2f)
        //    F is the Final Byte; it consists of a bit combination from 04/00 (\x40) to 07/14 (\x7e)
        //
        // NB: the ECMA-43/48 standards use `nibble/nibble`, in decimal, to represent a 7- or 8-bit number.
        // For example, `01/02` can be either 7- or 8-bit in their notation, and is equivalent to a more
        // familiar hex notation `0x12`. Similarly, `15/15` (which is necessarily 8-bit) is equivalent to `0xff`.
        //
        // cf. https://www.ecma-international.org/wp-content/uploads/ECMA-48_5th_edition_june_1991.pdf#page=24
        // and https://www.ecma-international.org/wp-content/uploads/ECMA-43_3rd_edition_december_1991.pdf#page=14

        let params = nom::branch::alt((
            nom::bytes::complete::is_a("0123456789:;<=>?"),
            nom::combinator::success(""),
        ));
        let intermediate = nom::branch::alt((
            nom::bytes::complete::is_a(concat!(" ", "!\"#$%&'()*+,/")),
            nom::combinator::success(""),
        ));
        let mut fin = nom::combinator::recognize(nom::character::streaming::satisfy(|ch| {
            ('\x40'..='\x7e').contains(&ch)
        }));

        // TODO?
        // let (rest, ((params, intermediate), fin)) =
        //     params.and(intermediate).and(fin).parse(input)?;
        // bug-compat:
        // currently, we bail out on sequences like "\u{1b}[;" with an error (even though it's reasonably considered incomplete, as we report without this check)
        let (rest, (params, intermediate)) = params.and(intermediate).parse(input)?;
        // but the trick is to avoid bailing on sequences like "\u{1b}[1;", which the old code considers "incomplete"
        // this
        if params.split(';').rev().skip(1).any(str::is_empty) {
            return Err(nom::Err::Failure(nom::error::Error {
                input: params,
                code: nom::error::ErrorKind::Char,
            }));
        }
        let (rest, fin) = fin.parse(rest)?;

        // TODO: collapse params & intr to "mid" w/ recognize(params.and(alt((inter, nonstandard)))) ?
        q[0] = start;
        q[1] = params;
        q[2] = intermediate;
        q[3] = fin;

        Ok((rest, &q[..]))
    }

    trait Params<'a>: Sized {
        fn parse(input: &'a str) -> IResult<&'a str, Self>;
    }

    trait FromDigits: core::str::FromStr {}

    impl<'a, T> Params<'a> for T
    where
        T: FromDigits,
    {
        fn parse(input: &'a str) -> IResult<&'a str, Self> {
            nom::combinator::map_res(nom::character::complete::digit1, Self::from_str).parse(input)
        }
    }
    impl FromDigits for usize {}
    impl FromDigits for isize {}

    trait AllConsuming<I, O, E>: nom::Parser<I, O, E> + Sized
    where
        I: nom::InputLength,
        E: nom::error::ParseError<I>,
    {
        fn parse_all(self, input: I) -> Result<O, nom::Err<E>> {
            nom::combinator::cut(nom::combinator::all_consuming(self))
                .parse(input)
                .map(|(_, o)| o)
        }
    }

    impl<I, O, E, T> AllConsuming<I, O, E> for T
    where
        T: nom::Parser<I, O, E>,
        I: nom::InputLength,
        E: nom::error::ParseError<I>,
    {
    }

    fn bail<O>(input: &str) -> IResult<&str, O> {
        nom::combinator::cut(nom::combinator::fail).parse(input)
    }

    /// kind of like [nom::multi::fill], but for up to N repeats rather than exactly N
    // TODO: can this be a parser? we could use .parse_all then
    // TODO?
    // g) If the parameter string starts with the bit combination 03/11, an empty parameter sub-string is
    //    assumed preceding the separator; if the parameter string terminates with the bit combination 03/11,
    //    an empty parameter sub-string is assumed following the separator; if the parameter string contains
    //    successive bit combinations 03/11, empty parameter sub-strings are assumed between the separators.
    //
    // h) If the control function has more than one parameter, and some parameter sub-strings are empty, the
    //    separators (bit combination 03/11) must still be present. However, if the last parameter sub-string(s)
    //    is empty, the separator preceding it may be omitted, see B.2 in annex B.
    // — https://www.ecma-international.org/wp-content/uploads/ECMA-48_5th_edition_june_1991.pdf#page=26
    fn many_param<'s, 'p, P: Params<'s>, const N: usize>(
        input: &'s str,
        params: &'p mut [P; N],
    ) -> Result<&'p [P], nom::Err<nom::error::Error<&'s str>>> {
        // todo: check first byte for '\x30'..=\x3b ([0-9:;]); else we're in Special Params-land
        let (_, i) = nom::combinator::all_consuming(nom::multi::fold_many_m_n(
            0,
            N,
            nom::sequence::terminated(
                P::parse,
                // TODO: this is wrong: it's not optional, unless it's in the last position
                nom::combinator::opt(nom::character::complete::char(';')),
            ),
            || 0,
            |i, p| {
                params[i] = p;
                i + 1
            },
        ))
        .parse(input)?;

        Ok(&params[..i])
    }

    fn param<'a, P: Params<'a> + Clone>(
        default: P,
    ) -> impl nom::Parser<&'a str, P, nom::error::Error<&'a str>> {
        nom::branch::alt((
            P::parse,
            nom::combinator::eof.and_then(nom::combinator::success(default)),
        ))
    }

    const ESC: &str = "\u{1b}";
    const CSI: &str = "\u{9b}";

    let mut buf = [""; 4];
    let (rest, seq) = gen_parse(input, &mut buf)?;

    println!("{:?}", seq);

    let back_compat_err = move |err| match err {
        nom::Err::Error(_) => nom::Err::Error(nom::error::Error {
            input: &input[2..],
            code: nom::error::ErrorKind::Fail,
        }),
        nom::Err::Failure(_) => nom::Err::Failure(nom::error::Error {
            input: &input[2..],
            code: nom::error::ErrorKind::Fail,
        }),
        nom::Err::Incomplete(n) => nom::Err::Incomplete(n),
    };

    let op = match *seq {
        // TODO:
        // nom::bytes::streaming::tag("VxD")(input).map(|(rest, _)| (rest, Op::Vgaterm(Vgaterm::Redraw)))

        //TODO:
        // [ESC, "7"] => Op::SaveCursorPos,
        // [ESC, "8"] => Op::RestoreCursorPos,
        // for now, bug-compat:
        [ESC, ESC, "7"] => Op::SaveCursorPos,
        [ESC, ESC, "8"] => Op::RestoreCursorPos,

        // bug-compat:
        [CSI, _, _, "f"] => return bail(&input[2..]),
        // TODO
        // [CSI, params, intr, "H"] | [CSI, params, intr, "f"] => {
        [CSI, params, intr, "H"] => {
            if !intr.is_empty() {
                return context("unrecognized intermediates", bail)(intr);
            }
            match *many_param(params, &mut [usize::default(); 2]).map_err(back_compat_err)? {
                [] => Op::MoveCursorAbs { x: 0, y: 0 },
                [a, b] => Op::MoveCursorAbs {
                    x: b.saturating_sub(1),
                    y: a.saturating_sub(1),
                },
                _ => return context("expected 0 or 2 params", bail).parse(params),
            }
        }

        [CSI, params, "", "A"] => param::<isize>(1)
            .parse_all(params)
            .map_err(back_compat_err)
            .map(|n| Op::MoveCursorDelta { dx: 0, dy: -n })?,
        [CSI, params, "", "B"] => param::<isize>(1)
            .parse_all(params)
            .map_err(back_compat_err)
            .map(|n| Op::MoveCursorDelta { dx: 0, dy: n })?,
        [CSI, params, "", "C"] => param::<isize>(1)
            .parse_all(params)
            .map_err(back_compat_err)
            .map(|n| Op::MoveCursorDelta { dx: n, dy: 0 })?,
        [CSI, params, "", "D"] => param::<isize>(1)
            .parse_all(params)
            .map_err(back_compat_err)
            .map(|n| Op::MoveCursorDelta { dx: -n, dy: 0 })?,
        [CSI, params, "", "E"] => param::<isize>(1)
            .parse_all(params)
            .map_err(back_compat_err)
            .map(|n| Op::MoveCursorBeginningAndLine { dy: n })?,
        [CSI, params, "", "F"] => param::<isize>(1)
            .parse_all(params)
            .map_err(back_compat_err)
            .map(|n| Op::MoveCursorBeginningAndLine { dy: -n })?,
        [CSI, params, "", "G"] => param::<usize>(0 /* <-- TODO?  */)
            .parse_all(params)
            .map_err(back_compat_err)
            .map(|n| Op::MoveCursorAbsCol {
                x: n.saturating_sub(1),
            })?,

        [CSI, params, "", "J"] => match params {
            "" | "0" => Op::EraseScreen(EraseMode::FromCursor),
            "1" => Op::EraseScreen(EraseMode::ToCursor),
            "2" => Op::EraseScreen(EraseMode::All),
            _ => {
                return context("invalid screen erase mode", bail)
                    .parse(params)
                    .map_err(back_compat_err);
            }
        },
        [CSI, params, "", "K"] => match params {
            "" | "0" => Op::EraseLine(EraseMode::FromCursor),
            "1" => Op::EraseLine(EraseMode::ToCursor),
            "2" => Op::EraseLine(EraseMode::All),
            _ => {
                return context("invalid line erase mode", bail)
                    .parse(params)
                    .map_err(back_compat_err);
            }
        },

        [CSI, "3", "", "~"] => Op::InPlaceDelete,

        [CSI, "6", "", "n"] => Op::RequestCursorPos,
        [CSI, params, "", "m"] => {
            nom::multi::separated_list0(
                nom::character::complete::char(';'),
                nom::combinator::complete(any_text_mode),
            )
            // back-compat (for using `any_text_mode`)
            // we need some sort of terminator for the streaming digit parsers to recognize the last item
            // so, let's keep it classic and pick '\0'
            .and(nom::character::complete::char('\0'))
            .map(|(r, _)| r)
            .parse_all(alloc::format!("{}\0", params).as_str())
            // TODO:
            // .parse_all(params)
            .map(Op::TextOp)
            .map_err(back_compat_err)?
        }

        [CSI, params, "", "h"] => nom::sequence::preceded(
            nom::character::complete::char('?'),
            nom::character::complete::digit0,
        )
        .map(|s: &str| Op::DecPrivateSet(s.to_owned()))
        .parse_all(params)
        .map_err(back_compat_err)?,
        [CSI, params, "", "l"] => nom::sequence::preceded(
            nom::character::complete::char('?'),
            nom::character::complete::digit0,
        )
        .map(|s: &str| Op::DecPrivateReset(s.to_owned()))
        .parse_all(params)
        .map_err(back_compat_err)?,

        // TODO:
        // [ESC, ..] | [CSI, ..] => return bail(input),
        // _ => return fail(input), // `fail` is (confusingly) not Failure, but Error
        // for now (back-compat):
        [ESC, ..] => return bail(&input[1..]),
        [CSI, ..] => return bail(&input[2..]),
        _ => {
            return nom::sequence::preceded(nom::bytes::complete::tag("\u{1b}"), fail).parse(input)
        }
    };

    Ok((rest, op))
}

fn parse_classic(input: &str) -> OpResult {
    start_with_esc(nom::branch::alt((
        save_cursor_position,
        restore_cursor_position,
        start_with_char(
            '[',
            nom::branch::alt((
                vgaterm_sequence,
                cursor_to_0,
                cursor_to_line_col,
                cursor_up_lines,
                cursor_down_lines,
                cursor_left_col,
                cursor_right_col,
                cursor_to_column,
                cursor_beginning_down,
                cursor_beginning_up,
                scroll_up,
                scroll_down,
                delete,
                erase_screen,
                erase_line,
                request_cursor_position,
                set_text_mode,
                set_private_sequence,
                reset_private_sequence,
            )),
        ),
    )))
    .parse(input)
}

pub fn parse(input: &str) -> OpResult {
    match (parse_classic(input), parse_new(input)) {
        (Ok(classic), Ok(new)) if classic == new => Ok(classic),
        (
            r @ Err(nom::Err::Error(..)),
            Err(nom::Err::Error(nom::error::Error {
                input: ref new,
                code: _,
            })),
        ) if {
            let Err(nom::Err::Error(nom::error::Error {
                input: ref classic,
                code: _,
            })) = r else { unreachable!() };
            classic
        } == new =>
        {
            r
        }
        (
            r @ Err(nom::Err::Failure(..)),
            Err(nom::Err::Failure(nom::error::Error {
                input: ref new,
                code: _,
            })),
        ) if {
            let Err(nom::Err::Failure(nom::error::Error {
                input: ref classic,
                code: _,
            })) = r else { unreachable!() };
            classic
        } == new =>
        {
            r
        }
        (r @ Err(nom::Err::Incomplete(..)), Err(nom::Err::Incomplete(_))) => r,
        (r, new) => {
            println!(
                "whuh oh! wanted: {:?}\n            got: {:?}\nfor input: {:?}",
                r, new, input
            );
            r
        }
    }
}

pub enum OpChar {
    Char(char),
    Op(Op),
}

#[derive(Debug)]
pub enum OpStr {
    Str(String),
    Op(Op),
}

#[derive(Debug)]
pub struct ParseRes<'a> {
    pub rest: &'a str,
    pub opstr: Vec<OpStr>,
}

impl<'a> ParseRes<'a> {
    fn from_ops(ops: Vec<OpStr>) -> ParseRes<'a> {
        ParseRes {
            rest: "",
            opstr: ops,
        }
    }

    fn new(ops: Vec<OpStr>, rest: &'a str) -> ParseRes<'a> {
        ParseRes { rest, opstr: ops }
    }
}

impl From<char> for OpChar {
    fn from(value: char) -> Self {
        OpChar::Char(value)
    }
}

pub fn parse_esc_str(s: &str) -> ParseRes {
    parse_esc_str_tail(s, vec![])
}

///
/// buffered version
/// "hello" -> "hello" (nom returns Error)
/// "ESC[","garbage" -> `InSequence`, "arbage" (nom returns Failure)
/// "ESC[Ablah", "garbage" -> [Foo(Op(A)), Foo("blah")], [Foo("garbage")],
/// "garbageESC[Ablah" -> ["garbage", Op(A), "blah"]
/// "garbageESC[Xblah" -> ["garbage", "blah"]
///
pub fn parse_esc_str_tail(s: &str, mut current: Vec<OpStr>) -> ParseRes {
    if s.is_empty() {
        return ParseRes::from_ops(current);
    }
    match parse(s) {
        Err(nom::Err::Incomplete(_)) => {
            // If we are incomplete, then do nothing
            // print!("{}", c.escape_default());
            ParseRes::new(current, s)
        }
        Err(nom::Err::Error(_)) => {
            // If we got an error, then we didn't recognize an esc seq at all
            // So pop the char back off the buffer
            let (rest, esc) = s.find(ESC).map_or((s, ""), |i| s.split_at(i));
            // abcde,ESC[
            current.push(OpStr::Str(rest.to_string()));
            parse_esc_str_tail(esc, current)
            // h e l l o
        }
        Err(nom::Err::Failure(e)) => {
            // And clear the buffer
            // ESC [ 6 * ESC [ 6 * ESC [ 6 * h e l l o literally the word loop and then some returns

            // If failure, then we were in a sequence but bombed out, and consume all the chars
            // ESC [ XYZ
            println!("Unknown {}", s.escape_debug());
            let skip_index = e.input.ceil_char_boundary(1);
            parse_esc_str_tail(&e.input[skip_index..], current)
        }
        Ok((rest, op)) => {
            // If we parsed an escape sequence, then clear the buffer and return the Op

            current.push(OpStr::Op(op));
            parse_esc_str_tail(rest, current)
        }
    }
}
