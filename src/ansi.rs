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
use nom::{IResult, Parser};

const ESC: char = '\u{1B}';

#[derive(Debug)]
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

#[derive(Debug)]
pub enum TextOp {
    SetBGBasic { bg: u8 },
    SetFGBasic { fg: u8 },
    SetFGColor256 { fg: u8 },
    SetBGColor256 { bg: u8 },
    ResetColors,
    SetTextMode(SetUnset, Style),
}

#[derive(Debug)]
pub enum Style {
    Bold,
    Dim,
    Italic,
    Strike,
    Underline,
    Blinking,
    Inverse,
}

#[derive(Debug)]
pub enum SetUnset {
    Set,
    Unset,
}

#[derive(Debug)]
pub enum EraseMode {
    FromCursor,
    ToCursor,
    All,
}

#[derive(Debug)]
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

pub fn parse6(input: &str) -> OpResult {
    fn gen_parse<'a, 'str>(
        input: &'str str,
        q: &'a mut [&'str str; 4],
    ) -> IResult<&'str str, &'a [&'str str]> {
        let (input, start) = nom::combinator::cut(nom::combinator::recognize(
            nom::character::streaming::one_of("\u{1b}\u{9b}"),
        ))
        .parse(input)?;

        q[0] = start;
        // c0
        match nom::combinator::cond(
            start == "\x1b",
            nom::sequence::tuple((
                nom::combinator::opt(nom::sequence::tuple((
                    nom::combinator::recognize(nom::character::streaming::char('\x21')),
                    nom::combinator::recognize(nom::character::streaming::char('\x40')),
                ))),
                nom::combinator::recognize(nom::character::streaming::satisfy(|ch| {
                    '\x00' < ch && ch < '\x1f'
                    // TODO: what set do these belong to ? any?
                    || ch == '7' || ch == '8'
                })),
            )),
        )
        .parse(input)
        {
            // collapse the two intro sequences to one
            Ok((rest, Some((Some(_), n)))) | Ok((rest, Some((None, n)))) => {
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

        // control sequences
        let input = if start == "\x1b" {
            let (input, _) = nom::character::streaming::char('[').parse(input)?;
            // map everything to this particular CSI
            q[0] = "\u{9b}";
            input
        } else {
            input
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
            nom::bytes::streaming::is_a("0123456789:;<=>?"),
            nom::combinator::success(""),
        ));
        let intermediate = nom::branch::alt((
            nom::bytes::streaming::is_a(concat!(" ", "!\"#$%&'()*+,/")),
            nom::combinator::success(""),
        ));
        let fin = nom::combinator::recognize(nom::character::streaming::satisfy(|ch| {
            ('\x40'..='\x7e').contains(&ch)
        }));

        let (rest, ((params, intermediate), fin)) =
            params.and(intermediate).and(fin).parse(input)?;

        q[1] = params;
        q[2] = intermediate;
        q[3] = fin;

        Ok((rest, &q[..=3]))
    }

    trait Params<'a>: Sized {
        fn parse(input: &'a str) -> IResult<&'a str, Self>;
    }
    impl<'a> Params<'a> for usize {
        fn parse(input: &'a str) -> IResult<&'a str, Self> {
            nom::combinator::map_res(nom::character::streaming::digit1, usize::from_str)
                .parse(input)
        }
    }

    /// kind of like [nom::multi::fill], but for up to N repeats rather than exactly N
    fn param<'s, 'p, P: Params<'s>, const N: usize>(
        input: &'s str,
        params: &'p mut [P; N],
    ) -> Result<&'p [P], nom::Err<nom::error::Error<&'s str>>> {
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

    const ESC: &str = "\u{1b}";
    const CSI: &str = "\u{9b}";

    let mut seq = [""; 4];
    let (rest, seq) = gen_parse(input, &mut seq)?;

    let op = match *seq {
        [ESC, "7"] => Op::SaveCursorPos,
        [ESC, "8"] => Op::RestoreCursorPos,

        [CSI, params, "", "H"] | [CSI, params, "", "f"] => {
            match *param(params, &mut [usize::default(); 2])? {
                [] => Op::MoveCursorAbs { x: 0, y: 0 },
                [a, b] => Op::MoveCursorAbs {
                    x: b.saturating_sub(1),
                    y: a.saturating_sub(1),
                },
                _ => {
                    todo!("return Err(Failure(..)) with appropriate context")
                    // return Err(alloc::format!(
                    //     "Bad number of params got {:?} wanted 0 or 2",
                    //     params
                    // ))
                }
            }
        }

        _ => todo!(),
    };

    Ok((rest, op))
}

fn parse(input: &str) -> OpResult {
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
