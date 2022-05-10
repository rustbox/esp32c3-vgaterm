#![no_std]

pub mod println;
mod timer;
pub mod gpio;
pub mod interrupt;
pub mod video;
pub mod spi;

pub use println::configure;
pub use timer::{configure_timer0, enable_timer0_interrupt, clear_timer0, start_timer0};
pub use gpio::check_gpio_source;

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