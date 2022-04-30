#![no_std]

pub mod println;
mod timer;

pub use println::configure;
pub use timer::{configure_timer0, enable_timer0_interrupt, clear_timer0, start_timer0};


pub fn hello() -> &'static str {
    "hello"
}