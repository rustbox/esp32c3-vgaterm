use core::cell::RefCell;

use alloc::collections::VecDeque;
use critical_section::Mutex;
use esp32c3_hal::{gpio::{Gpio1, Gpio3, Unknown}, peripherals::UART1, clock::Clocks, interrupt::{Priority}};
use esp_println::{print, println};

use crate::{usb_keyboard::{Key, KeyEvent, USBKeyboardDevice, Parse, US_ENGLISH}, uart, channel::Receiver, interrupt};

static KEYBOARD: Mutex<RefCell<Option<Keyboard>>> = Mutex::new(RefCell::new(None));

pub fn configure(tx: Gpio1<Unknown>, rx: Gpio3<Unknown>, uart: UART1, clocks: &Clocks) {
    let rx = uart::configure1(uart, tx, rx, clocks);
    uart::interrupt_enable1(Priority::Priority5);

    let keyboard = Keyboard::new(US_ENGLISH, rx);
    critical_section::with(|cs| {
        KEYBOARD.borrow_ref_mut(cs).replace(keyboard);
    });
}

pub fn configure2(kb: Keyboard) {
    critical_section::with(|cs| {
        KEYBOARD.borrow_ref_mut(cs).replace(kb);
    });
}

pub fn next_event() -> KeyEvent<Key> {
    println!("pre cs");
    critical_section::with(|cs| {
        println!("in cs");
        let mut kb = KEYBOARD.borrow_ref_mut(cs);
        println!("getting the kb instance {:?}", kb);
        kb.as_mut().expect("Keyboard must be configured before key events can be detected").next_event()
    })
}

pub fn next_event2(kb: &mut Keyboard) -> KeyEvent<Key> {
    kb.next_event()
}

#[derive(Debug)]
pub struct Keyboard {
    device: USBKeyboardDevice,
    key_events: VecDeque<KeyEvent<Key>>,
    rx: Receiver<u8>,
}

impl Keyboard {
    pub fn new(layout: &'static [Key], rx: Receiver<u8>) -> Keyboard {
        Keyboard {
            device: USBKeyboardDevice::new(layout),
            key_events: VecDeque::new(),
            rx
        }
    }

    pub fn next_event(&mut self) -> KeyEvent<Key> {
        loop {
            // First receive any transaction bytes and attempt to parse
            while let Some(k) = self.rx.recv() {
                // print!(".");
                if let Parse::Finished(r) = self.device.next_report_byte(k) {
                    match r {
                        Ok(m) => {
                            let events = self.device.next_report(&m.message);
                            for e in events {
                                let ke = self.device.code_event_into_key(e);
                                self.key_events.push_front(ke);
                            }
                        },
                        Err(e) => {
                            println!("Error: {:?}", e);
                        }
                    }
                }
            }

            // Grab the next item off the queue. But if there are none,
            // Then we should wait for the next byte coming along rx, and
            // start from 'transaction again
            if let Some(k) = self.key_events.pop_back() {
                return k;
            } else {
                println!("wfi");
                unsafe {
                    riscv::asm::wfi();
                }
                println!("{:?}", interrupt::source());
                println!("awakened");
            }
        }


    }
}
