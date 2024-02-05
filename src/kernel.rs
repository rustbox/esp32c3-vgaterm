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

use esp32c3_hal::gpio::{Event, Unknown};
use esp32c3_hal::gpio::Input;
use esp32c3_hal::gpio::{Gpio3, PullDown};
use esp32c3_hal::macros::ram;
use esp_println::println;

use crate::video;

pub const BLANKING_WAIT_TIME: u64 = 3960; // us

pub fn start(start: Gpio3<Unknown>) {
    println!("start!");
    // let mut terminal = TextField::new();
    // let mut display = Display::new();
    let _ = crate::gpio::pin_interrupt(
        start.into_pull_down_input(),
        Event::FallingEdge,
        frame,
        // move |_| {
        // frame();
        // Get the pressed chars
        // while let Some(t) = receiver.recv() {
        //     terminal.type_next(t);
        // }
        // // Draw the characters on the frame
        // terminal.draw(&mut display);
        // // Flush the Display to the BUFFER
        // display.flush()
        // }
    );
}

///
/// Transmit the contents of the frame buffer out to the monitor via SPI.
///
#[ram]
pub fn frame(_: &mut Gpio3<Input<PullDown>>) {
    // Beginning of frame, so let's guarantee we start at 0
    // print!("*");
    unsafe {
        video::OFFSET = 0;
    }
    video::transmit_chunk();
}
