#![no_std]
#![feature(array_chunks)]

extern crate alloc;

pub mod println;
mod timer;
pub mod gpio;
pub mod interrupt;
pub mod video;
pub mod spi;
pub mod kernel;

pub use println::configure;
pub use timer::{
    configure_timer0,
    enable_timer0_interrupt,
    clear_timer0,
    start_timer0,
    start_timer0_callback,
    Delay,
    delay,
    deadline,
    wait_until,
};
use unroll::unroll_for_loops;

use core::arch::asm;

pub fn hello() -> &'static str {
    "hello"
}

#[no_mangle]
#[inline]
pub fn start_cycle_count() {
    unsafe {
        // Set event counter to 0
        asm!(
            "csrwi 0x7E2, 0x00",
        )
    }
}

#[no_mangle]
#[inline]
pub fn measure_cycle_count() -> u32{
    let d: u32;
    unsafe {
        asm!(
            "csrr {}, 0x7E2",
            out(reg) d
        );
    }
    d
}

#[unroll_for_loops]
pub fn noops(n: u32) {
    for _ in 0..n {
        unsafe {
            asm!("nop");
        }
    }
}

