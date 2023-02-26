#![no_std]
#![feature(array_chunks)]
#![feature(const_trait_impl)]

extern crate alloc;

pub mod gpio;
pub mod interrupt;
pub mod kernel;
// pub mod println;
pub mod ansi;
pub mod channel;
pub mod color;
pub mod display;
pub mod usb_keyboard;
pub mod spi;
pub mod terminal;
pub mod text;
mod timer;
pub mod uart;
pub mod video;

/// This is required for the #[interrupt] macro on interrupt handler functions to work properly.
/// Such as gpio::GPIO() interrupt handler. This is due to how the esp32c3_hal crate implmented
/// this macro, it expects `peripherals::Interrupt` to be available.
pub mod peripherals {
    pub use esp32c3_hal::peripherals::Interrupt;
}

// pub use println::configure;
pub use timer::{
    clear_timer0, configure_timer0, deadline, delay, enable_timer0_interrupt, start_timer0,
    start_timer0_callback, wait_until, Delay,
};

use core::arch::asm;

pub fn hello() -> &'static str {
    "hello"
}

#[no_mangle]
#[inline]
pub fn start_cycle_count() {
    unsafe {
        // Set event counter to 0
        asm!("csrwi 0x7E2, 0x00",)
    }
}

#[no_mangle]
#[inline]
pub fn measure_cycle_count() -> u32 {
    let d: u32;
    unsafe {
        asm!(
            "csrr {}, 0x7E2",
            out(reg) d
        );
    }
    d
}

#[inline]
pub fn noops<const N: u8>() {
    if 0 < N {
        unsafe { asm!("nop") }
    }
    if 1 < N {
        unsafe { asm!("nop") }
    }
    if 2 < N {
        unsafe { asm!("nop") }
    }
    if 3 < N {
        unsafe { asm!("nop") }
    }
    if 4 < N {
        unsafe { asm!("nop") }
    }
    if 5 < N {
        unsafe { asm!("nop") }
    }
    if 6 < N {
        unsafe { asm!("nop") }
    }
    if 7 < N {
        unsafe { asm!("nop") }
    }
    if 8 < N {
        unsafe { asm!("nop") }
    }
    if 9 < N {
        unsafe { asm!("nop") }
    }
    if 10 < N {
        unsafe { asm!("nop") }
    }
    if 11 < N {
        unsafe { asm!("nop") }
    }
    if 12 < N {
        unsafe { asm!("nop") }
    }
    if 13 < N {
        unsafe { asm!("nop") }
    }
    if 14 < N {
        unsafe { asm!("nop") }
    }
    if 15 < N {
        unsafe { asm!("nop") }
    }
    if 16 < N {
        unsafe { asm!("nop") }
    }
    if 17 < N {
        unsafe { asm!("nop") }
    }
    if 18 < N {
        unsafe { asm!("nop") }
    }
    if 19 < N {
        unsafe { asm!("nop") }
    }
    if 20 < N {
        unsafe { asm!("nop") }
    }
    if 21 < N {
        unsafe { asm!("nop") }
    }
    if 22 < N {
        unsafe { asm!("nop") }
    }
    if 23 < N {
        unsafe { asm!("nop") }
    }
    if 24 < N {
        unsafe { asm!("nop") }
    }
    if 25 < N {
        unsafe { asm!("nop") }
    }
    if 26 < N {
        unsafe { asm!("nop") }
    }
    if 27 < N {
        unsafe { asm!("nop") }
    }
    if 28 < N {
        unsafe { asm!("nop") }
    }
    if 29 < N {
        unsafe { asm!("nop") }
    }
    if 30 < N {
        unsafe { asm!("nop") }
    }
    if 31 < N {
        unsafe { asm!("nop") }
    }

    for _ in 32..N {
        unsafe {
            asm!("nop");
        }
    }
}
