use alloc::vec::Vec;

///
/// Header
/// ============================
/// 0:      start "0xFE"
/// 1-2:    length [Low, High]
/// 3:      msg type
/// 4:      device type (0x6 is keyboard?)
/// 5:      device index
/// 6:      endpoint
/// 7-8:    vendor ID [Low, High]
/// 9-10:   Product ID [Low, High]
///
#[derive(Debug, Clone, Copy)]
pub struct Header {
    pub length: u16,
    pub msg_type: u8,
    pub device_type: DeviceType,
    pub device_index: u8,
    pub endpoint: u8,
    pub vendor_id: u16,
    pub product_id: u16,
}

/// The bytes contained in the USB-UART report for which keys are pressed.
#[derive(Debug, Clone)]
pub struct Message {
    pub message: Vec<u8>,
}

/// The Error variants while parsing a USB-UART Header/Message
#[derive(Debug, Clone)]
pub enum Error<'a> {
    ResponseNotLongEnough(&'a [u8]),
    WrongStartByte(&'a [u8]),
    DeviceNotKeyboard(&'a [u8]),
    WrongEndByte(Vec<u8>),
}

impl Header {
    pub fn from_bytes(report: &[u8]) -> Result<Header, Error> {
        if report.len() < HEADER_LENGTH {
            return Err(Error::ResponseNotLongEnough(report));
        }

        if report[0] != START {
            return Err(Error::WrongStartByte(report));
        }

        let length = (report[1] as u16) | (report[2] as u16) << 8;

        let vendor_id = (report[7] as u16) | (report[8] as u16) << 8;

        let product_id = (report[9] as u16) | (report[10] as u16) << 8;

        if report[4] != DeviceType::Keyboard as u8 {
            return Err(Error::DeviceNotKeyboard(report));
        }

        Ok(Header {
            length,
            msg_type: report[3],
            device_type: DeviceType::Keyboard,
            device_index: report[5],
            endpoint: report[6],
            vendor_id,
            product_id,
        })
    }
}

// Message stuff
// let mut message = Vec::new();
//         let s = &report[HEADER_LENGTH..HEADER_LENGTH + length as usize];
//         for m in s {
//             message.push(*m);
//         }

//         if report[HEADER_LENGTH + length as usize] != END {
//             return Err(Error::WrongEndByte(report));
//         }

pub const START: u8 = 0xFE;
pub const END: u8 = 0x0A;
pub const HEADER_LENGTH: usize = 11;

#[derive(Debug, Clone, Copy)]
pub enum DeviceType {
    Keyboard = 0x6,
}

#[derive(Debug, Clone, Copy, Ord, Eq, PartialEq, PartialOrd)]
pub enum Mod {
    LeftCtrl,
    LeftShift,
    LeftAlt,
    LeftGui,
    RightCtrl,
    RightShift,
    RightAlt,
    RightGui,
}

///
/// Each kind of keyboard key.
///
/// Printable Keys are for keys which change outputs upon holding the shift key.
/// Lockable keys also have shift variants, but can also be held shifted with Caps Lock.
/// The first entry in Printable and Lockable Keys is what's displayed without Shift
/// or Caps Lock applied, and the second is when either is applied. When Caps Lock is on
/// and Shift is also applied the Lockable key should display the first entry again as if
/// neither were applied.
///
/// The UndefinedKey is for situations where a keycode may not actually correspond to an
/// actual key press such as an error state.
///
/// The Reserved variant is for key code 0, where no corresponding key exists.
///
/// The Numpad variants correspond to the keys on the numpad. Many of the keys there have
/// dual functions like Printable Keys but the two functions are controllable by setting
/// the Num Lock key. With Num Lock set, the numbers (or period in the case of the period/Delete key)
/// will be selected on the numpad and when not set the Home, Page Down, etc keys are selected.
///
/// These cases must be handled individually with the Num Lock state when matching on a Key
#[derive(Debug, Clone, Copy, Ord, Eq, PartialEq, PartialOrd)]
pub enum Key {
    Printable(char, char),
    Lockable(char, char),
    ESC,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    PrntScreen,
    Insert,
    ScrollLock,
    Delete,
    Backspace,
    Tab,
    CapsLock,
    Enter,
    Mod(Mod),
    Application,
    UpArrow,
    LeftArrow,
    DownArrow,
    RightArrow,
    Spacebar,
    Home,
    End,
    PageUp,
    PageDown,
    Pause,
    Break,
    Numlock,
    KeypadSlash,
    KeypadAsterisk,
    KeypadDash,
    KeypadPlus,
    Keypad7Home,
    Keypad8Up,
    Keypad9PageUp,
    Keypad4Left,
    Keypad5,
    Keypad6Right,
    Keypad1End,
    Keypad2Down,
    Keypad3PageDown,
    Keypad0Insert,
    KeypadPeriodDelete,
    KeypadEnter,
    Reserved,
    RollOverError,
    PostFail,
    UndefinedError,
    UndefinedKey(u8),
}

pub type KeyLayout = &'static [Key];

use Mod::*;
pub const LEFT_CTRL: (usize, Key) = (0xE0, Key::Mod(LeftCtrl));
pub const LEFT_SHIFT: (usize, Key) = (0xE1, Key::Mod(LeftShift));
pub const LEFT_ALT: (usize, Key) = (0xE2, Key::Mod(LeftAlt));
pub const LEFT_GUI: (usize, Key) = (0xE3, Key::Mod(LeftGui));
pub const RIGHT_CTRL: (usize, Key) = (0xE4, Key::Mod(RightCtrl));
pub const RIGHT_SHIFT: (usize, Key) = (0xE5, Key::Mod(RightShift));
pub const RIGHT_ALT: (usize, Key) = (0xE6, Key::Mod(RightAlt));
pub const RIGHT_GUI: (usize, Key) = (0xE7, Key::Mod(RightGui));

pub const MOD_KEYS: [Key; 8] = [
    Key::Mod(LeftCtrl),
    Key::Mod(LeftShift),
    Key::Mod(LeftAlt),
    Key::Mod(LeftGui),
    Key::Mod(RightCtrl),
    Key::Mod(RightShift),
    Key::Mod(RightAlt),
    Key::Mod(RightGui),
];

pub const MOD_KEY_OFFSET: usize = 0xE0;

///
/// The layout for a US English 104 key keyboard
///
/// The index in the Key array represents the USB Usage Code generated
/// by the USB keyboard. To get the corresponding Key from a usage code (key code)
/// simply index into the array using the code:
///
/// ```
/// let key_code: u8 = 0x0b;
/// let key = &US_ENGLISH[key_code as usize];
/// assert_eq!(key  Key::Lockable('h', 'H'));
/// ```
pub static US_ENGLISH: KeyLayout = {
    use Key::*;
    let layout: &'static [Key] = &[
        Reserved,
        RollOverError,
        PostFail,
        UndefinedError,
        Lockable('a', 'A'),
        Lockable('b', 'B'),
        Lockable('c', 'C'),
        Lockable('d', 'D'),
        Lockable('e', 'E'),
        Lockable('f', 'F'),
        Lockable('g', 'G'),
        Lockable('h', 'H'),
        Lockable('i', 'I'),
        Lockable('j', 'J'),
        Lockable('k', 'K'),
        Lockable('l', 'L'),
        Lockable('m', 'M'),
        Lockable('n', 'N'),
        Lockable('o', 'O'),
        Lockable('p', 'P'),
        Lockable('q', 'Q'),
        Lockable('r', 'R'),
        Lockable('s', 'S'),
        Lockable('t', 'T'),
        Lockable('u', 'U'),
        Lockable('v', 'V'),
        Lockable('w', 'W'),
        Lockable('x', 'X'),
        Lockable('y', 'Y'),
        Lockable('z', 'Z'),
        Printable('1', '!'),
        Printable('2', '@'),
        Printable('3', '#'),
        Printable('4', '$'),
        Printable('5', '%'),
        Printable('6', '^'),
        Printable('7', '^'),
        Printable('8', '*'),
        Printable('9', '('),
        Printable('0', ')'),
        Enter,
        ESC,
        Backspace,
        Tab,
        Spacebar,
        Printable('-', '_'),
        Printable('=', '+'),
        Printable('[', '{'),
        Printable(']', '}'),
        Printable('\\', '|'),
        Printable('#', '~'),
        Printable(';', ':'),
        Printable('\'', '"'),
        Printable('`', '~'),
        Printable(',', '<'),
        Printable('.', '>'),
        Printable('/', '?'),
        CapsLock,
        F1,
        F2,
        F3,
        F4,
        F5,
        F6,
        F7,
        F8,
        F9,
        F10,
        F11,
        F12,
        PrntScreen,
        ScrollLock,
        Pause,
        Insert,
        Home,
        PageUp,
        Delete,
        End,
        PageDown,
        RightArrow,
        LeftArrow,
        DownArrow,
        UpArrow,
        Numlock,
        KeypadSlash,
        KeypadAsterisk,
        KeypadDash,
        KeypadPlus,
        KeypadEnter,
        Keypad1End,
        Keypad2Down,
        Keypad3PageDown,
        Keypad4Left,
        Keypad5,
        Keypad6Right,
        Keypad7Home,
        Keypad8Up,
        Keypad9PageUp,
        Keypad0Insert,
        KeypadPeriodDelete,
        Printable('\\', '|'),
        Application,
    ];
    layout
};

#[derive(Debug)]
pub enum KeyValue {
    Printable(char),
    Action(Key),
    Nothing,
}

///
/// KeyEvent represents a Pressed or Released key. The generic
/// type can be used to contain keycodes or a Key variant
#[derive(Debug)]
pub enum KeyEvent<T> {
    Pressed(T),
    Released(T),
}

#[derive(Debug, Clone)]
pub enum Parse<'a> {
    Continue,
    Finished(Result<Message, Error<'a>>),
}

/// State of the Keyboard USB-UART Header/Message parser. Waiting
/// indicates that the parser can accept additional bytes of input.
/// ReportStarted indicates that the Header is recognized and parsing
/// has begun. Once the Header is parsed the state transitions to
/// MessageStarted.
#[derive(Debug)]
enum ParseState {
    Waiting,
    ReportStarted,
    MessageStarted(Header),
}

///
/// The USBKeyboardDevice is for taking USB-UART input byte by byte and generates `KeyEvents`s
/// that represent a key being pressed or released.
///
/// When parsing USB-UART input, `next_report_byte` will return Parse::Continue to indicate that
/// the USBKeyboardDevice is ready to receive more input for parsing the message.
///
/// If a full USB-UART header and message has been succesfully parsed, `next_report_byte` will
/// return the Finished variant with an Ok result containing the key pressed report. This is
/// a Vec of u8 which contains the keycodes/report codes as defined in Table 12 of
/// https://web.archive.org/web/20180826215839/http://www.usb.org/developers/hidpage/Hut1_12v2.pdf.
///
/// Once a Message is parsed and returned, `next_report` will compare the incoming message with
/// the last given message and will return the list of keys pressed and released as a Vec of
/// KeyEvent of the keycodes (as u8).
///
/// Each keycode can be mapped to actual Keys using the layout internal to the USBKeyboardDevice using
/// `translate_keycode`, which would allow you to make the events list above contain Keys instead of
/// keycodes.
#[derive(Debug)]
pub struct USBKeyboardDevice {
    layout: KeyLayout,
    last_keys: Vec<u8>,
    parse_state: ParseState,
    report_buffer: Vec<u8>,
    message_buffer: Vec<u8>,
}

impl USBKeyboardDevice {
    /// Create a new Keyboard Device with the given layout
    pub fn new(layout: KeyLayout) -> USBKeyboardDevice {
        USBKeyboardDevice {
            layout,
            last_keys: Vec::new(),
            report_buffer: Vec::new(),
            message_buffer: Vec::new(),
            parse_state: ParseState::Waiting,
        }
    }

    /// Parse an incoming USB-UART message byte by byte. The function will return with
    /// `Parse::Continue` while it hasn't encountered an error or parsed an entire
    /// key report. Once a full report is parsed the function returns `Parse::Finished`
    /// containing an Ok variant with the contents of the key report.
    pub fn next_report_byte(&mut self, b: u8) -> Parse {
        match self.parse_state {
            ParseState::Waiting => {
                if b == START {
                    // When we detect the START of the Header/Report (with 0xFE)
                    // we'll ensure that the accumulated state from before is cleared.
                    self.report_buffer.clear();
                    self.message_buffer.clear();
                    self.report_buffer.push(b);
                    self.parse_state = ParseState::ReportStarted
                } else {
                    self.parse_state = ParseState::Waiting
                }
                Parse::Continue
            }
            ParseState::ReportStarted => {
                // Collect bytes until we can parse a full report
                if self.report_buffer.len() < HEADER_LENGTH {
                    self.report_buffer.push(b);
                    self.parse_state = ParseState::ReportStarted;
                    Parse::Continue
                } else {
                    match Header::from_bytes(&self.report_buffer) {
                        Ok(r) => {
                            // Since we've generated a Header, the currently incoming byte
                            // is actually part of the "message", so that byte should be
                            // pushed onto the message buffer
                            self.message_buffer.push(b);
                            self.parse_state = ParseState::MessageStarted(r);
                            Parse::Continue
                        }
                        Err(e) => {
                            self.parse_state = ParseState::Waiting;
                            Parse::Finished(Err(e))
                        }
                    }
                }
            }
            // Once we have a full report, we know the length of the
            // message, and we can collect bytes until we have the full message
            ParseState::MessageStarted(header) => {
                if self.message_buffer.len() < header.length as usize {
                    self.message_buffer.push(b);
                    self.parse_state = ParseState::MessageStarted(header);
                    Parse::Continue
                } else {
                    // Once the message is complete, we can collect one more byte
                    // and verify it's the end byte
                    if b == END {
                        self.report_buffer.clear();
                        self.parse_state = ParseState::Waiting;
                        Parse::Finished(Ok(Message {
                            message: self.message_buffer.drain(..).collect(),
                        }))
                    } else {
                        self.report_buffer.clear();
                        let mut v: Vec<_> = self.message_buffer.drain(..).collect();
                        v.push(b);
                        self.parse_state = ParseState::Waiting;
                        Parse::Finished(Err(Error::WrongEndByte(v)))
                    }
                }
            }
        }
    }

    /// With a message containing the current keys pressed as reported by the keyboard
    /// (probably parsed from `next_report_byte`) this function will compare the incoming
    /// set of pressed keys with the previous set. This tells us the list of Keys that
    /// were pressed since the last time and the list of Keys released since the last time
    /// forming a Vec of KeyEvents.
    pub fn next_report(&mut self, message: &[u8]) -> Vec<KeyEvent<u8>> {
        let mut new_keys = Vec::new();

        // Get all the currently pressed modifier keys and generate the keycodes for them
        let mod_keys = message[0];
        let mod_key_offset = 0xE0;
        for i in 0..8 {
            if (mod_keys & 1 << i) != 0 {
                new_keys.push((mod_key_offset + i) as u8);
            }
        }

        // Add each subsequent key press from the report
        for m in &message[2..] {
            new_keys.push(*m);
        }

        // Get keys added in the new report and keys removed since the last report
        let added: Vec<_> = new_keys
            .iter()
            .filter(|k| !self.last_keys.contains(*k))
            .collect();
        let released: Vec<_> = self
            .last_keys
            .iter()
            .filter(|k| !new_keys.contains(*k))
            .collect();
        let mut events = Vec::new();
        for k in added {
            events.push(KeyEvent::Pressed(*k))
        }
        for k in released {
            events.push(KeyEvent::Released(*k));
        }

        // The new report is now the previous
        self.last_keys = new_keys;

        // Return the key events
        events
    }

    /// For a given keycode/usage code this will look up the corresponding
    /// Key as determined by the layout.
    pub fn translate_keycode(&self, code: u8) -> Key {
        if code as usize >= MOD_KEY_OFFSET {
            let ix = code as usize - MOD_KEY_OFFSET;
            return MOD_KEYS[ix];
        }
        if (code as usize) < self.layout.len() {
            self.layout[code as usize]
        } else {
            Key::UndefinedKey(code)
        }
    }

    pub fn code_event_into_key(&self, event: KeyEvent<u8>) -> KeyEvent<Key> {
        match event {
            KeyEvent::Pressed(k) => KeyEvent::Pressed(self.translate_keycode(k)),
            KeyEvent::Released(k) => KeyEvent::Released(self.translate_keycode(k)),
        }
    }
}
