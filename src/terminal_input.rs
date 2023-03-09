use alloc::{
    collections::{BTreeMap, BTreeSet},
    string::String,
};
use esp32c3_hal::systimer::SystemTimer;
use lazy_static::lazy_static;

use crate::{keyboard::PressedSet, timer, usb_keyboard::Key};

// const ESC: char = '\u{27}';
const ESC: &str = "\u{27}";
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

    static ref KEY_COMBINATION: BTreeMap<KeyCombo, String> = {
        let mut map = BTreeMap::new();
        // map.insert(KeyCombo::new([Key::Mod(LeftCtrl, Key::Lockable('a', 'A')]), String::from("\u{01}"));
        // map.insert(KeyCombo::new([Key::RightCtrl, Key::Lockable('a', 'A')]), String::from("\u{01}"));
        // map.insert(KeyCombo::new([Key::LeftCtrl, Key::Lockable('b', 'B')]), String::from("\u{02}"));
        // map.insert(KeyCombo::new([Key::RightCtrl, Key::Lockable('b', 'B')]), String::from("\u{02}"));
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

    pub fn key_char(&mut self, pressed: &PressedSet) -> Option<char> {
        if let Some(k) = self.key(pressed) {
            // println!("{:?}", k);
            match k {
                Key::Lockable(low, up) => {
                    let shifted = pressed.caps_lock ^ pressed.shift();
                    if shifted {
                        Some(up)
                    } else {
                        Some(low)
                    }
                }
                Key::Printable(low, up) => {
                    if pressed.shift() {
                        Some(up)
                    } else {
                        Some(low)
                    }
                }
                Key::Keypad0Insert => {
                    if pressed.num_lock {
                        Some('0')
                    } else {
                        None
                    }
                }
                Key::Keypad1End => {
                    if pressed.num_lock {
                        Some('1')
                    } else {
                        None
                    }
                }
                Key::Keypad2Down => {
                    if pressed.num_lock {
                        Some('2')
                    } else {
                        None
                    }
                }
                Key::Keypad3PageDown => {
                    if pressed.num_lock {
                        Some('3')
                    } else {
                        None
                    }
                }
                Key::Keypad4Left => {
                    if pressed.num_lock {
                        Some('4')
                    } else {
                        None
                    }
                }
                Key::Keypad5 => Some('5'),
                Key::Keypad6Right => {
                    if pressed.num_lock {
                        Some('6')
                    } else {
                        None
                    }
                }
                Key::Keypad7Home => {
                    if pressed.num_lock {
                        Some('7')
                    } else {
                        None
                    }
                }
                Key::Keypad8Up => {
                    if pressed.num_lock {
                        Some('8')
                    } else {
                        None
                    }
                }
                Key::Keypad9PageUp => {
                    if pressed.num_lock {
                        Some('9')
                    } else {
                        None
                    }
                }
                Key::KeypadPeriodDelete => {
                    if pressed.num_lock {
                        Some('.')
                    } else {
                        None
                    }
                }
                Key::KeypadSlash => Some('/'),
                Key::KeypadAsterisk => Some('*'),
                Key::KeypadDash => Some('-'),
                Key::KeypadPlus => Some('+'),
                Key::Spacebar => Some(' '),
                _ => None,
            }
        } else {
            None
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
                Some(k) => {
                    if p == k {
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
                    } else {
                        (
                            HeldState::LongDelay(
                                k,
                                timer::deadline((self.key_delay_ms * 1000) as u64),
                            ),
                            Some(k),
                        )
                    }
                }
            },
            HeldState::ShortDelay(p, deadline) => match key {
                None => (HeldState::Waiting, None),
                Some(k) => {
                    if p == k {
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
                    } else {
                        (
                            HeldState::ShortDelay(
                                k,
                                timer::deadline((self.repeat_delay_ms * 1000) as u64),
                            ),
                            Some(k),
                        )
                    }
                }
            },
        }
    }
}

impl Default for TerminalInput {
    fn default() -> Self {
        TerminalInput::new(300, 80)
    }
}
