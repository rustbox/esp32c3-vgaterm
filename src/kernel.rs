//!
//! The main logic of the Video and Terminal controller.
//! 
//! The controller is maintained by a state machine. There are two main
//! sets of states, representing the portion the controller is writing pixels
//! from video memory via SPI into the video hardware. Pixels are written into
//! a hardware FIFO in the video display hardware, which must be in communication
//! with the software so that FIFO is never overfilled and never empties. 
//! 
//! When the controller is not actively writing pixels to the video hardware
//! it should be reading any external input (like UART for keyboard input, etc),
//! updating the video frame memory, and updating any other internal state. In
//! particular, once the frame is ended (known by an external interrupt by hardware)
//! software should reset all frame logic back to the beginning of the frame.
//!

use crate::{sprintln, video};

use esp32c3_hal::gpio::Gpio3;
use esp32c3_hal::gpio_types::{Unknown, Event};
use riscv::asm::wfi;

pub const BLANKING_WAIT_TIME: u64 = 3960; // us

pub fn start(start: Gpio3<Unknown>) {
    sprintln!("start!");
    let _ = crate::gpio::pin_interrupt(
        start.into_floating_input(),
        Event::RisingEdge,
        |_| {
            frame();
        }
    );

    let mut color: u8 = 0;
    let mut frames = 0;
    loop {
        unsafe {
            // if frames == 60 {
            //     frames = 0;
            //     color += 1;
            //     let (r, g, b) = video::rgb_from_byte(color.into());
            //     sprintln!("Color: {}, {}, {}", r, g, b);
            //     crate::video::load_test_pattern(color, color);
            // }
            frames += 1;
            wfi();
        }
    }
}


///
/// Transmit the contents of the frame buffer out to the monitor via SPI.
/// 
#[inline]
pub fn frame() {
    crate::gpio::gpio_pin_out(8, true);
    crate::video::transmit_frame();
    crate::gpio::gpio_pin_out(8, false);
}
