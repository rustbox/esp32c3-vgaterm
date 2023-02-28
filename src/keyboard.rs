use core::cell::RefCell;

use alloc::collections::VecDeque;
use critical_section::Mutex;
use esp32c3_hal::{
    clock::Clocks,
    gpio::{Gpio1, Gpio3, Unknown},
    interrupt,
    interrupt::Priority,
    peripherals::UART1,
};
use esp32c3_hal::{interrupt::CpuInterrupt, prelude::*, Cpu, Uart};
use esp_println::println;

use crate::{
    uart::{self},
    usb_keyboard::{Key, KeyEvent, Parse, USBKeyboardDevice, US_ENGLISH},
};

static KEYBOARD: Mutex<RefCell<Option<Keyboard>>> = Mutex::new(RefCell::new(None));

pub fn configure(tx: Gpio1<Unknown>, rx: Gpio3<Unknown>, uart: UART1, clocks: &Clocks) {
    let uart = uart::make_uart1(uart, tx, rx, clocks);

    interrupt::enable(
        esp32c3_hal::peripherals::Interrupt::UART1,
        Priority::Priority4,
    )
    .unwrap();
    interrupt::set_kind(
        Cpu::ProCpu,
        CpuInterrupt::Interrupt4,
        interrupt::InterruptKind::Edge,
    );

    let keyboard = Keyboard::new(US_ENGLISH, uart);
    critical_section::with(|cs| {
        KEYBOARD.borrow_ref_mut(cs).replace(keyboard);
    });
}

pub fn next_event() -> KeyEvent<Key> {
    loop {
        let ke = critical_section::with(|cs| {
            let mut kb = KEYBOARD.borrow_ref_mut(cs);
            let ke = kb
                .as_mut()
                .expect("Keyboard must be configured before key events can be detected")
                .next_event();
            ke
        });

        if let Some(k) = ke {
            return k;
        } else {
            unsafe {
                riscv::asm::wfi();
            }
        }
    }
}

pub struct Keyboard<'a> {
    device: USBKeyboardDevice,
    key_events: VecDeque<KeyEvent<Key>>,
    uart: Uart<'a, UART1>,
}

impl<'a> Keyboard<'a> {
    pub fn new(layout: &'static [Key], uart: Uart<'a, UART1>) -> Keyboard<'a> {
        let mut uart = uart;
        uart.set_rx_fifo_full_threshold(1);
        uart.listen_rx_fifo_full();

        Keyboard {
            device: USBKeyboardDevice::new(layout),
            key_events: VecDeque::new(),
            uart,
        }
    }

    pub fn next_event(&mut self) -> Option<KeyEvent<Key>> {
        // First receive any transaction bytes and attempt to parse
        self.key_events.pop_back()
    }

    fn parse_next_byte(&mut self, b: u8) -> Parse {
        self.device.next_report_byte(b)
    }
}

#[interrupt]
fn UART1() {
    critical_section::with(|cs| {
        if let Some(keyboard) = KEYBOARD.borrow_ref_mut(cs).as_mut() {
            while let nb::Result::Ok(c) = keyboard.uart.read() {
                // print!(".");
                if let Parse::Finished(message) = keyboard.parse_next_byte(c) {
                    match message {
                        Ok(m) => {
                            let events = keyboard.device.next_report(&m.message);
                            for k in events {
                                keyboard
                                    .key_events
                                    .push_front(keyboard.device.code_event_into_key(k));
                            }
                        }
                        Err(e) => {
                            println!("Error: {:?}", e);
                        }
                    }
                }
            }
            keyboard.uart.reset_rx_fifo_full_interrupt();
        }
    });
}
