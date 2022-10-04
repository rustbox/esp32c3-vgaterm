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

use alloc::format;

use esp32c3_hal::gpio::{Gpio3, Gpio8};
use esp32c3_hal::gpio_types::{Unknown, Event};
use riscv::asm::wfi;

// const BLANKING_WAIT_TIMER: u64 = 3000; // us
// const DRAIN_WAIT_TIMER: u64 = 70; // us
// const FILL_AMOUNT: u32 = 3200; // bytes
pub const BLANKING_WAIT_TIME: u64 = 3960; // us

pub fn start(vis: Gpio3<Unknown>) {
    sprintln!("start!");
    let _ = crate::gpio::pin_interrupt(
        vis.into_pull_down_input(),
        Event::FallingEdge,
        |_| {
            // sprint!(".");
            crate::start_timer0_callback(BLANKING_WAIT_TIME, move|| {
                // sprintln!("*");
                frame();
                
            })
        }
    );

    loop {
        unsafe {
            // sprintln!("loop?");
            wfi();
        }
    }
}

enum ControllerState {
    FirstChunk,
    Frame(usize),
}

impl ControllerState {
    fn update(&self, transition: Transition) -> ControllerState{
        use ControllerState::*;
        use Transition::*;
        let size = 5 * crate::video::WIDTH;
        match (self, transition) {
            (FirstChunk, VisLow) => FirstChunk,
            (FirstChunk, FifoEmpty) => Frame(size),
            (Frame(p), FifoEmpty) => Frame(p + size),
            (Frame(_), VisLow) => FirstChunk
        }
    }
}

enum Transition {
    FifoEmpty,
    VisLow
}

const CHUNK_79: usize = 79 * 5 * crate::video::WIDTH;

static mut CONTROLLER_STATE: ControllerState = ControllerState::FirstChunk;

pub fn start2(vis: Gpio3<Unknown>, fifo_empty: Gpio8<Unknown>) {
    let _ = crate::gpio::pin_interrupt(
        vis.into_pull_down_input(),
        Event::FallingEdge,
        |_| unsafe {
            // Reset state back to first chunk
            // Turn on fifo_empty interrupt
            CONTROLLER_STATE = CONTROLLER_STATE.update(Transition::VisLow);
            
        }
    );

    let mut chunk_ready = crate::gpio::pin_interrupt(
        fifo_empty.into_pull_down_input(), 
        Event::FallingEdge, 
        |p| unsafe {
            CONTROLLER_STATE = CONTROLLER_STATE.update(Transition::FifoEmpty);
            // if we're chunk 79, pause 
            match CONTROLLER_STATE {
                ControllerState::Frame(px) => {
                    if px == CHUNK_79 {
                        p.unlisten();
                    }
                },
                _ => {}
            };
        }
    );

    loop {
        unsafe {
            match CONTROLLER_STATE {
                ControllerState::FirstChunk => {
                    chunk_ready = crate::gpio::pin_resume(chunk_ready, Event::FallingEdge);
                    first_chunk();
                    wfi();
                },
                ControllerState::Frame(p) => {
                    chunk(p);
                    wfi();
                }
            };
            wfi();
        }
    }
}

pub fn chunk(start: usize) {
    let size = 5 * crate::video::WIDTH;
    unsafe {
        crate::spi::transmit(&crate::video::BUFFER[start..start+size]);
    }
}

pub fn first_chunk() {
    let size = 5 * crate::video::WIDTH;
    unsafe {
        crate::spi::transmit(&crate::video::BUFFER[0..size]);
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
#[inline]
pub fn frame() {
    let mut length = 6 * crate::video::WIDTH;
    let mut px = 0;
    unsafe {
        // First 6 lines
        let deadline = crate::deadline(170);
        crate::spi::transmit(&crate::video::BUFFER[px..px+length]);
        px += length;
        length = 5 * crate::video::WIDTH;
        crate::wait_until(deadline);

        // The next 390 lines
        for _ in 1..79 {
            let deadline = crate::deadline(170);
            crate::spi::transmit(&crate::video::BUFFER[px..px+length]);
            px += length;
            crate::wait_until(deadline);
        }

        // The last 4 lines
        length = 4 * crate::video::WIDTH;
        crate::spi::transmit(&crate::video::BUFFER[px..px+length]);
    }
}
