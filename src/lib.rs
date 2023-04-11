#![no_std]
#![feature(array_chunks)]
#![feature(const_trait_impl)]
#![feature(iter_collect_into)]
#![feature(trait_alias)]
#![feature(round_char_boundary)]

extern crate alloc;

pub mod ansi;
pub mod channel;
pub mod color;
pub mod display;
pub mod gpio;
pub mod interrupt;
pub mod kernel;
pub mod keyboard;
pub mod perf;
pub mod spi;
pub mod terminal;
pub mod terminal_input;
pub mod text;
pub mod timer;
pub mod uart;
pub mod usb_keyboard;
pub mod video;

// pub use println::configure;
pub use timer::{
    clear_timer0, configure_timer0, deadline, delay, enable_timer0_interrupt, start_timer0,
    start_timer0_callback, wait_until, Delay,
};

use core::arch::asm;

pub static mut CHARACTER_DRAW_CYCLES: usize = 0;

// back compat
pub use perf::measure_cycle_count;
pub use perf::reset_cycle_count as start_cycle_count;

#[derive(Debug)]
pub enum Work<T> {
    Item(T), // implicitly: awaken immediately

    WouldBlock, // indefinitely
    WouldBlockUntil(u64),
}

pub fn hello() -> &'static str {
    "hello"
}

pub fn measure<O>(count: &mut usize, f: impl FnOnce() -> O) -> O {
    start_cycle_count();
    let r = f();
    *count = measure_cycle_count() as usize;

    r
}

pub fn measure<O>(count: &mut usize, f: impl FnOnce() -> O) -> O {
    start_cycle_count();
    let r = f();
    *count = measure_cycle_count() as usize;

    r
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
