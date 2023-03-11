use alloc::{
    borrow::ToOwned,
    collections::{BTreeMap, BTreeSet},
    string::String,
    vec::Vec,
};
use esp32c3_hal::systimer::SystemTimer;
use lazy_static::lazy_static;

use crate::{
    keyboard::PressedSet,
    timer,
    usb_keyboard::{Key, Mod},
};

// const ESC: char = '\u{27}';
const ESC: &str = "\u{1B}";
// static ESCS: String = String::from_str(ESCH).unwrap();

fn join(c: &str, tail: &str) -> String {
    let mut x = String::new();
    x.push_str(c);
    x.push_str(tail);
    x
}

lazy_static! {
    static ref KEY_TERMINAL_SEQUENCES: BTreeMap<Key, String> = {
        let mut map = BTreeMap::new();
        map.insert(Key::UpArrow, join(ESC, "[A"));
        map.insert(Key::DownArrow, join(ESC, "[B"));
        map.insert(Key::RightArrow, join(ESC, "[C"));
        map.insert(Key::LeftArrow, join(ESC, "[D"));
        map.insert(Key::End, join(ESC, "[F"));
        map.insert(Key::Home, join(ESC, "[H"));
        map.insert(Key::Insert, join(ESC, "[2~"));
        map.insert(Key::Delete, join(ESC, "[3~"));
        map.insert(Key::PageUp, join(ESC, "[5~"));
        map.insert(Key::PageDown, join(ESC, "[6~"));
        map.insert(Key::ESC, String::from(ESC));
        map.insert(Key::F1, join(ESC, "[1P"));
        map.insert(Key::F2, join(ESC, "[1Q"));
        map.insert(Key::F3, join(ESC, "[1R"));
        map.insert(Key::F4, join(ESC, "[1S"));
        map.insert(Key::F5, join(ESC, "[15~"));
        map.insert(Key::F6, join(ESC, "[17~"));
        map.insert(Key::F7, join(ESC, "[18~"));
        map.insert(Key::F8, join(ESC, "[19~"));
        map.insert(Key::F9, join(ESC, "[20~"));
        map.insert(Key::F10, join(ESC, "[21~"));
        map.insert(Key::F11, join(ESC, "[23~"));
        map.insert(Key::F12, join(ESC, "[24~"));
        map.insert(Key::Pause, join(ESC, "[P"));
        map
    };
    static ref KEY_COMBINATION: Vec<(&'static [Key], String)> = {
        let mut map = Vec::new();
        map.push((
            &[Key::Mod(Mod::LeftCtrl), Key::Lockable('a', 'A')] as &[_],
            String::from("\u{01}"),
        ));
        map.push((
            &[Key::Mod(Mod::RightCtrl), Key::Lockable('a', 'A')] as &[_],
            String::from("\u{01}"),
        ));
        map.push((
            &[Key::Mod(Mod::LeftCtrl), Key::Lockable('b', 'B')] as &[_],
            String::from("\u{02}"),
        ));
        map.push((
            &[Key::Mod(Mod::RightCtrl), Key::Lockable('b', 'B')] as &[_],
            String::from("\u{02}"),
        ));
        map.push((
            &[Key::Mod(Mod::LeftCtrl), Key::Lockable('c', 'C')] as &[_],
            String::from("\u{03}"),
        ));
        map.push((
            &[Key::Mod(Mod::RightCtrl), Key::Lockable('c', 'C')] as &[_],
            String::from("\u{03}"),
        ));
        map.push((
            &[Key::Mod(Mod::LeftCtrl), Key::Lockable('d', 'D')] as &[_],
            String::from("\u{04}"),
        ));
        map.push((
            &[Key::Mod(Mod::RightCtrl), Key::Lockable('d', 'D')] as &[_],
            String::from("\u{04}"),
        ));
        map.push((
            &[Key::Mod(Mod::LeftCtrl), Key::Lockable('e', 'E')] as &[_],
            String::from("\u{05}"),
        ));
        map.push((
            &[Key::Mod(Mod::RightCtrl), Key::Lockable('e', 'E')] as &[_],
            String::from("\u{05}"),
        ));
        map.push((
            &[Key::Mod(Mod::LeftCtrl), Key::Lockable('f', 'F')] as &[_],
            String::from("\u{06}"),
        ));
        map.push((
            &[Key::Mod(Mod::RightCtrl), Key::Lockable('f', 'F')] as &[_],
            String::from("\u{06}"),
        ));
        map.push((
            &[Key::Mod(Mod::LeftCtrl), Key::Lockable('g', 'G')] as &[_],
            String::from("\u{07}"),
        ));
        map.push((
            &[Key::Mod(Mod::RightCtrl), Key::Lockable('g', 'G')] as &[_],
            String::from("\u{07}"),
        ));
        map.push((
            &[Key::Mod(Mod::LeftCtrl), Key::Lockable('h', 'H')] as &[_],
            String::from("\u{08}"),
        ));
        map.push((
            &[Key::Mod(Mod::RightCtrl), Key::Lockable('h', 'H')] as &[_],
            String::from("\u{08}"),
        ));
        map.push((
            &[Key::Mod(Mod::LeftCtrl), Key::Lockable('i', 'I')] as &[_],
            String::from("\u{09}"),
        ));
        map.push((
            &[Key::Mod(Mod::RightCtrl), Key::Lockable('i', 'I')] as &[_],
            String::from("\u{09}"),
        ));
        map.push((
            &[Key::Mod(Mod::LeftCtrl), Key::Lockable('j', 'J')] as &[_],
            String::from("\u{0A}"),
        ));
        map.push((
            &[Key::Mod(Mod::RightCtrl), Key::Lockable('j', 'J')] as &[_],
            String::from("\u{0A}"),
        ));
        map.push((
            &[Key::Mod(Mod::LeftCtrl), Key::Lockable('k', 'K')] as &[_],
            String::from("\u{0B}"),
        ));
        map.push((
            &[Key::Mod(Mod::RightCtrl), Key::Lockable('k', 'K')] as &[_],
            String::from("\u{0B}"),
        ));
        map.push((
            &[Key::Mod(Mod::LeftCtrl), Key::Lockable('l', 'L')] as &[_],
            String::from("\u{0C}"),
        ));
        map.push((
            &[Key::Mod(Mod::RightCtrl), Key::Lockable('l', 'L')] as &[_],
            String::from("\u{0C}"),
        ));
        map.push((
            &[Key::Mod(Mod::LeftCtrl), Key::Lockable('m', 'M')] as &[_],
            String::from("\u{0D}"),
        ));
        map.push((
            &[Key::Mod(Mod::RightCtrl), Key::Lockable('m', 'M')] as &[_],
            String::from("\u{0D}"),
        ));
        map.push((
            &[Key::Mod(Mod::LeftCtrl), Key::Lockable('n', 'N')] as &[_],
            String::from("\u{0E}"),
        ));
        map.push((
            &[Key::Mod(Mod::RightCtrl), Key::Lockable('n', 'N')] as &[_],
            String::from("\u{0E}"),
        ));
        map.push((
            &[Key::Mod(Mod::LeftCtrl), Key::Lockable('o', 'O')] as &[_],
            String::from("\u{0F}"),
        ));
        map.push((
            &[Key::Mod(Mod::RightCtrl), Key::Lockable('o', 'O')] as &[_],
            String::from("\u{0F}"),
        ));
        map.push((
            &[Key::Mod(Mod::LeftCtrl), Key::Lockable('p', 'P')] as &[_],
            String::from("\u{10}"),
        ));
        map.push((
            &[Key::Mod(Mod::RightCtrl), Key::Lockable('p', 'P')] as &[_],
            String::from("\u{10}"),
        ));
        map.push((
            &[Key::Mod(Mod::LeftCtrl), Key::Lockable('q', 'Q')] as &[_],
            String::from("\u{11}"),
        ));
        map.push((
            &[Key::Mod(Mod::RightCtrl), Key::Lockable('q', 'Q')] as &[_],
            String::from("\u{11}"),
        ));
        map.push((
            &[Key::Mod(Mod::LeftCtrl), Key::Lockable('r', 'R')] as &[_],
            String::from("\u{12}"),
        ));
        map.push((
            &[Key::Mod(Mod::RightCtrl), Key::Lockable('r', 'R')] as &[_],
            String::from("\u{12}"),
        ));
        map.push((
            &[Key::Mod(Mod::LeftCtrl), Key::Lockable('s', 'S')] as &[_],
            String::from("\u{13}"),
        ));
        map.push((
            &[Key::Mod(Mod::RightCtrl), Key::Lockable('s', 'S')] as &[_],
            String::from("\u{13}"),
        ));
        map.push((
            &[Key::Mod(Mod::LeftCtrl), Key::Lockable('t', 'T')] as &[_],
            String::from("\u{14}"),
        ));
        map.push((
            &[Key::Mod(Mod::RightCtrl), Key::Lockable('t', 'T')] as &[_],
            String::from("\u{14}"),
        ));
        map.push((
            &[Key::Mod(Mod::LeftCtrl), Key::Lockable('u', 'U')] as &[_],
            String::from("\u{15}"),
        ));
        map.push((
            &[Key::Mod(Mod::RightCtrl), Key::Lockable('u', 'U')] as &[_],
            String::from("\u{15}"),
        ));
        map.push((
            &[Key::Mod(Mod::LeftCtrl), Key::Lockable('v', 'V')] as &[_],
            String::from("\u{16}"),
        ));
        map.push((
            &[Key::Mod(Mod::RightCtrl), Key::Lockable('v', 'V')] as &[_],
            String::from("\u{16}"),
        ));
        map.push((
            &[Key::Mod(Mod::LeftCtrl), Key::Lockable('w', 'W')] as &[_],
            String::from("\u{17}"),
        ));
        map.push((
            &[Key::Mod(Mod::RightCtrl), Key::Lockable('w', 'W')] as &[_],
            String::from("\u{17}"),
        ));
        map.push((
            &[Key::Mod(Mod::LeftCtrl), Key::Lockable('x', 'X')] as &[_],
            String::from("\u{18}"),
        ));
        map.push((
            &[Key::Mod(Mod::RightCtrl), Key::Lockable('x', 'X')] as &[_],
            String::from("\u{18}"),
        ));
        map.push((
            &[Key::Mod(Mod::LeftCtrl), Key::Lockable('y', 'Y')] as &[_],
            String::from("\u{19}"),
        ));
        map.push((
            &[Key::Mod(Mod::RightCtrl), Key::Lockable('y', 'Y')] as &[_],
            String::from("\u{19}"),
        ));
        map.push((
            &[Key::Mod(Mod::LeftCtrl), Key::Lockable('z', 'Z')] as &[_],
            String::from("\u{1A}"),
        ));
        map.push((
            &[Key::Mod(Mod::RightCtrl), Key::Lockable('z', 'Z')] as &[_],
            String::from("\u{1A}"),
        ));
        map
    };
}

// #[derive(PartialEq, Eq, PartialOrd, Ord)]
// enum Mod {
//     Shift,
//     Caps,
//     Num,
//     Ctrl,
//     Alt,
//     Gui,
// }

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct KeyCombo {
    // pub mods: BTreeSet<Mod>,
    pub keys: BTreeSet<Key>,
}

impl KeyCombo {
    pub fn new<I: IntoIterator<Item = Key>>(key: I) -> KeyCombo {
        let mut combo = BTreeSet::new();
        for k in key.into_iter() {
            combo.insert(k);
        }

        KeyCombo { keys: combo }
    }
}

/// No printable keys are pressed, we're in the waiting state.
/// When a key is first pressed we enter the LongDelay state which
/// contains the key being pressed and the time until we enter the
/// ShortDelay state. If no ther key is pressed in the meantime or
/// if the key is not released yet and continues to be held, when
/// the time reaches the time contained in the LongDelay, we enter
/// the ShortDelay state containing the Key being held as well as the
/// time until we enter the ShortDelay state again with the time updated.
/// When a Key is released we return to the Waiting state.
/// When a different key is pressed, we move to the LongState.
enum HeldState {
    LongDelay(Key, u64),
    ShortDelay(Key, u64),
    Waiting,
}

/// Processes Keyboard input and converts the keyboard state
/// into a character stream into a terminal
pub struct TerminalInput {
    key_delay_ms: u32,
    repeat_delay_ms: u32,
    state: HeldState,
}

impl TerminalInput {
    pub fn new(key_delay_ms: u32, repeat_delay_ms: u32) -> TerminalInput {
        TerminalInput {
            key_delay_ms,
            repeat_delay_ms,
            state: HeldState::Waiting,
        }
    }

    pub fn key(&mut self, pressed: &PressedSet) -> Option<Key> {
        let recent = pressed.recent();
        let (next_state, out_key) = self.next_state(recent);
        self.state = next_state;

        out_key
    }

    fn combo(&self, pressed: &PressedSet) -> Option<String> {
        for (combo, s) in KEY_COMBINATION.iter() {
            if pressed.matches_combo(combo) {
                return Some(s.to_owned());
            }
        }
        None
    }

    pub fn key_char(&mut self, pressed: &PressedSet) -> String {
        if let Some(s) = self.combo(pressed) {
            return s;
        }
        if let Some(k) = self.key(pressed) {
            match k {
                Key::Lockable(low, up) => {
                    let shifted = pressed.caps_lock ^ pressed.shift();
                    if shifted {
                        String::from(up)
                    } else {
                        String::from(low)
                    }
                }
                Key::Printable(low, up) => {
                    if pressed.shift() {
                        String::from(up)
                    } else {
                        String::from(low)
                    }
                }
                Key::Keypad0Insert => {
                    if pressed.num_lock {
                        String::from('0')
                    } else {
                        KEY_TERMINAL_SEQUENCES[&Key::Insert].to_owned()
                    }
                }
                Key::Keypad1End => {
                    if pressed.num_lock {
                        String::from('1')
                    } else {
                        KEY_TERMINAL_SEQUENCES[&Key::End].to_owned()
                    }
                }
                Key::Keypad2Down => {
                    if pressed.num_lock {
                        String::from('2')
                    } else {
                        KEY_TERMINAL_SEQUENCES[&Key::DownArrow].to_owned()
                    }
                }
                Key::Keypad3PageDown => {
                    if pressed.num_lock {
                        String::from('3')
                    } else {
                        KEY_TERMINAL_SEQUENCES[&Key::PageDown].to_owned()
                    }
                }
                Key::Keypad4Left => {
                    if pressed.num_lock {
                        String::from('4')
                    } else {
                        KEY_TERMINAL_SEQUENCES[&Key::LeftArrow].to_owned()
                    }
                }
                Key::Keypad5 => {
                    if pressed.num_lock {
                        String::from('5')
                    } else {
                        String::new()
                    }
                }
                Key::Keypad6Right => {
                    if pressed.num_lock {
                        String::from('6')
                    } else {
                        KEY_TERMINAL_SEQUENCES[&Key::RightArrow].to_owned()
                    }
                }
                Key::Keypad7Home => {
                    if pressed.num_lock {
                        String::from('7')
                    } else {
                        KEY_TERMINAL_SEQUENCES[&Key::Home].to_owned()
                    }
                }
                Key::Keypad8Up => {
                    if pressed.num_lock {
                        String::from('8')
                    } else {
                        KEY_TERMINAL_SEQUENCES[&Key::UpArrow].to_owned()
                    }
                }
                Key::Keypad9PageUp => {
                    if pressed.num_lock {
                        String::from('9')
                    } else {
                        KEY_TERMINAL_SEQUENCES[&Key::PageUp].to_owned()
                    }
                }
                Key::KeypadPeriodDelete => {
                    if pressed.num_lock {
                        String::from('.')
                    } else {
                        KEY_TERMINAL_SEQUENCES[&Key::Delete].to_owned()
                    }
                }
                Key::KeypadEnter => String::from('\n'),
                Key::KeypadSlash => String::from('/'),
                Key::KeypadAsterisk => String::from('*'),
                Key::KeypadDash => String::from('-'),
                Key::KeypadPlus => String::from('+'),
                Key::Spacebar => String::from(' '),
                Key::Backspace => String::from('\u{08}'),
                Key::Enter => String::from('\n'),
                Key::Tab => String::from('\u{09}'),
                _ => KEY_TERMINAL_SEQUENCES
                    .get(&k)
                    .unwrap_or(&String::new())
                    .to_owned(),
            }
        } else {
            String::new()
        }
    }

    fn next_state(&self, key: Option<Key>) -> (HeldState, Option<Key>) {
        match self.state {
            HeldState::Waiting => match key {
                None => (HeldState::Waiting, None),
                Some(k) => (
                    HeldState::LongDelay(k, timer::deadline((self.key_delay_ms * 1000) as u64)),
                    Some(k),
                ),
            },
            HeldState::LongDelay(p, deadline) => match key {
                None => (HeldState::Waiting, None),
                Some(k) if p == k => {
                    let ticks = SystemTimer::now();
                    if ticks >= deadline {
                        (
                            HeldState::ShortDelay(
                                k,
                                timer::deadline((self.repeat_delay_ms * 1000) as u64),
                            ),
                            Some(k),
                        )
                    } else {
                        (HeldState::LongDelay(k, deadline), None)
                    }
                }
                Some(k) => (
                    HeldState::LongDelay(k, timer::deadline((self.key_delay_ms * 1000) as u64)),
                    Some(k),
                ),
            },
            HeldState::ShortDelay(p, deadline) => match key {
                None => (HeldState::Waiting, None),
                Some(k) if p == k => {
                    let ticks = SystemTimer::now();
                    if ticks >= deadline {
                        (
                            HeldState::ShortDelay(
                                k,
                                timer::deadline((self.repeat_delay_ms * 1000) as u64),
                            ),
                            Some(k),
                        )
                    } else {
                        (HeldState::LongDelay(k, deadline), None)
                    }
                }
                Some(k) => (
                    HeldState::ShortDelay(k, timer::deadline((self.repeat_delay_ms * 1000) as u64)),
                    Some(k),
                ),
            },
        }
    }
}

impl Default for TerminalInput {
    fn default() -> Self {
        TerminalInput::new(300, 40)
    }
}
