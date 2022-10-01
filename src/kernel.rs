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

use crate::{sprint, sprintln};
use crate::timer::Delay;

use esp32c3_hal::gpio::Gpio3;
use esp32c3_hal::gpio_types::{Unknown, Event};
use riscv::asm::wfi;

// const BLANKING_WAIT_TIMER: u64 = 3000; // us
// const DRAIN_WAIT_TIMER: u64 = 70; // us
// const FILL_AMOUNT: u32 = 3200; // bytes
const BLANKING_WAIT_TIME: u64 = 3972; // us

pub fn start(vis: Gpio3<Unknown>, delay: Delay) {
    sprintln!("start!");
    let _ = crate::gpio::pin_interrupt(
        vis.into_pull_down_input(),
        Event::FallingEdge,
        move |_| {
            sprint!(".");
            crate::start_timer0_callback(BLANKING_WAIT_TIME, move|| {
                // sprintln!("*");
                frame(delay);
            })
        }
    );

    loop {
        unsafe {
            sprintln!("loop?");
            wfi();
        }
    }
}

/// Go ~5 lines at a time (3200 bytes) (a chunk)
/// It takes 31.78us for a whole line to drain so
/// 5 lines will be about 158.9 us to drain fully.
/// 
/// So if we fill 6 lines as the first chunk and
/// then wait 159us to drain 5 lines we'll always have
/// some leftover until the end and we won't go over either
/// 
/// Do we use a timer to calculate the vertical blank??
/// Vertical blank time total is 1430.36 us, so set an interrupt timer for maybe 1400 us?
fn frame(delay: Delay) {
    let mut length = 6 * crate::video::WIDTH;
    let mut px = 0;
    unsafe {
        // First 6 lines
        let deadline = delay.deadline(159);
        crate::spi::transmit(&crate::video::BUFFER[px..px+length]);
        px += length;
        length = 5 * crate::video::WIDTH;
        delay.wait_until(deadline);

        // The next 390 lines
        for _ in 1..79 {
            let deadline = delay.deadline(159);
            crate::spi::transmit(&crate::video::BUFFER[px..px+length]);
            px += length;
            delay.wait_until(deadline);
        }

        // The last 4 lines
        length = 4 * crate::video::WIDTH;
        crate::spi::transmit(&crate::video::BUFFER[px..px+length]);
    }
}
