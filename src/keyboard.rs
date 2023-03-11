use alloc::{collections::VecDeque, vec::Vec};
use esp32c3_hal::{
    clock::Clocks,
    gpio::{Gpio1, Gpio3, Unknown},
    interrupt::Priority,
    peripherals::UART1,
};
use esp_println::println;

use crate::{
    channel::Receiver,
    uart::{self},
    usb_keyboard::{Key, KeyEvent, KeyLayout, Mod, Parse, USBKeyboardDevice},
};

pub struct Keyboard {
    device: USBKeyboardDevice,
    key_events: VecDeque<KeyEvent<Key>>,
    rx: Receiver<u8>,
    pressed: PressedSet,
}

impl Keyboard {
    fn new(layout: KeyLayout, rx: Receiver<u8>) -> Keyboard {
        Keyboard {
            device: USBKeyboardDevice::new(layout),
            key_events: VecDeque::new(),
            rx,
            pressed: PressedSet::new(),
        }
    }

    pub fn from_peripherals(
        layout: KeyLayout,
        tx: Gpio1<Unknown>,
        rx: Gpio3<Unknown>,
        uart: UART1,
        clocks: &Clocks,
    ) -> Keyboard {
        let receiver = uart::configure1(uart, tx, rx, clocks);
        uart::interrupt_enable1(Priority::Priority5);
        Keyboard::new(layout, receiver)
    }

    pub fn next_event(&mut self) -> Option<KeyEvent<Key>> {
        self.flush_and_parse();
        self.key_events.pop_back()
    }

    pub fn current(&self) -> &PressedSet {
        &self.pressed
    }

    /// Update the key event queue from parsed uart bytes. Then dequeue one
    /// element from the queue and push it into the pressed
    pub fn update(&mut self) {
        // This will update the key_event queue
        self.flush_and_parse();
        if let Some(event) = self.key_events.pop_back() {
            self.pressed.push(event);
        }
    }

    /// Read all the bytes currently in the Receiver and parse them
    /// into KeyEvents, placing them onto the queue
    pub fn flush_and_parse(&mut self) {
        while let Some(b) = self.rx.recv() {
            if let Parse::Finished(m) = self.device.next_report_byte(b) {
                match m {
                    Ok(m) => {
                        let events = self.device.next_report(&m.message);
                        events
                            .into_iter()
                            .filter_map(|ke| {
                                let key_event = self.device.code_event_into_key(ke);
                                // get the key out of the event
                                let key = match key_event {
                                    KeyEvent::Pressed(k) => k,
                                    KeyEvent::Released(k) => k,
                                };
                                // If it's some error, just report to stdout and swallow the bad key
                                match key {
                                    Key::RollOverError
                                    | Key::UndefinedError
                                    | Key::UndefinedKey(_) => {
                                        // println!("Key Error: {:?}", key_event);
                                        None
                                    }
                                    Key::Reserved => None,
                                    _ => Some(key_event),
                                }
                                // Put each event on the event queue
                            })
                            .for_each(|event| self.key_events.push_front(event));
                    }
                    Err(e) => {
                        println!("Parse error: {:?}", e);
                    }
                }
            }
        }
    }
}

pub struct PressedSet {
    pressed: Vec<Key>,
    modifiers: Vec<Key>,
    pub caps_lock: bool,
    pub num_lock: bool,
}

impl PressedSet {
    fn new() -> PressedSet {
        PressedSet {
            pressed: Vec::new(),
            modifiers: Vec::new(),
            caps_lock: false,
            num_lock: false,
        }
    }

    fn push(&mut self, event: KeyEvent<Key>) {
        match event {
            KeyEvent::Pressed(k) => match k {
                Key::Mod(_) => {
                    if !self.modifiers.contains(&k) {
                        self.modifiers.push(k);
                    }
                }
                Key::CapsLock => {
                    self.caps_lock = !self.caps_lock;
                }
                Key::Numlock => {
                    self.num_lock = !self.num_lock;
                }
                _ => {
                    if !self.pressed.contains(&k) {
                        self.pressed.push(k);
                    }
                }
            },
            KeyEvent::Released(k) => {
                if let Key::Mod(_) = k {
                    self.modifiers.retain(|&c| c != k);
                } else {
                    self.pressed.retain(|&c| c != k);
                }
            }
        }
    }

    pub fn recent(&self) -> Option<Key> {
        self.pressed.last().copied()
    }

    pub fn pressed(&self) -> (&[Key], &[Key]) {
        (self.modifiers.as_slice(), self.pressed.as_slice())
    }

    pub fn shift(&self) -> bool {
        self.modifiers.contains(&Key::Mod(Mod::LeftShift))
            || self.modifiers.contains(&Key::Mod(Mod::RightShift))
    }

    pub fn matches_combo(&self, combo: &[Key]) -> bool {
        if combo.len() == self.pressed.len() + self.modifiers.len() {
            // check if all keys in the combo are in the contents
            combo.iter().all(|k| match k {
                &Key::Mod(_) => self.modifiers.contains(k),
                _ => self.pressed.contains(k),
            })
        } else {
            false
        }
    }
}
