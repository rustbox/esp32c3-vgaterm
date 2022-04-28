#![no_std]

pub mod println;

pub use println::configure;


pub fn hello() -> &'static str {
    "hello"
}