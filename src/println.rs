//! Implement the sprintln macro

use core::fmt::Write;

use embedded_hal::prelude::_embedded_hal_serial_Write;
use riscv::interrupt;

use esp32c3_hal::Serial;
use esp32c3_hal::pac::UART0;



static mut SERIAL: Option<SerialWrapper> = None;


struct SerialWrapper(Serial<UART0>);

/// Constructs a Serial type from UART0 which initializes the Serial instance
pub fn configure(uart: UART0) {
    let sr = Serial::new(uart);
    interrupt::free(|_| unsafe {
        SERIAL.replace(SerialWrapper(sr))
    });
}

impl core::fmt::Write for SerialWrapper {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for byte in s.as_bytes() {
            if *byte == '\n' as u8 {
                let res = self.0.write_char('\r');
                if res.is_err() {
                    return Err(core::fmt::Error);
                }
            }

            let res = self.0.write(*byte);
            if res.is_err() {
                return Err(core::fmt::Error);
            }
        }
        Ok(())
    }
}

pub fn write_str(s: &str) {
    interrupt::free(|_| unsafe {
        if let Some(serial) = SERIAL.as_mut() {
            let _ = serial.write_str(s);
        }
    })
}

pub fn write_fmt(args: core::fmt::Arguments) {
    interrupt::free(|_| unsafe {
        if let Some(serial) = SERIAL.as_mut() {
            let _ = serial.write_fmt(args);
        }
    })
}

/// Macro for printing to the serial standard output
#[macro_export]
macro_rules! sprint {
    ($s:expr) => {
        $crate::println::write_str($s)
    };
    ($($tt:tt)*) => {
        $crate::println::write_fmt(format_args!($($tt)*))
    };
}

/// Macro for printing to the serial standard output, with a newline.
#[macro_export]
macro_rules! sprintln {
    () => {
        $crate::println::write_str("\n")
    };
    ($s:expr) => {
        $crate::println::write_str(concat!($s, "\n"))
    };
    ($s:expr, $($tt:tt)*) => {
        $crate::println::write_fmt(format_args!(concat!($s, "\n"), $($tt)*))
    };
}
