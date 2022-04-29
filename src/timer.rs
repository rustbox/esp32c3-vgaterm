//!
//! To create a timer interrupt:
//! 1. call `configure_timer0` passing in the `TIMG0` peripheral. This initializes the global
//! TIMER0, and then disables it before it is used or needed.
//! 2. enable the interrupt with `enable_timer0_interrupt` setting which interrupt handler
//! to route to along with a priority
//! 3. To route the timer interrupt to handler `N`, create a function called `interruptN` for `N`
//! between 1 and 31. In the interrupt handler, typically the implementation should turn off
//! interrupts (commonly with riscv::interrupt::free(...)) and then the interrupt should be cleared
//! with `clear_timer0` passing in the correct interrupt number enum variant.
//! 4. Start the timer with `start_timer0` and the number of ticks (?) to count down from.
//! 
//! As a reminder, interrupts (Machine interrupts: the `mie` bit of the `mstatus` register) will need
//! to be enabled for any interrupt to occur generally.


use bare_metal::Mutex;
use esp_hal_common::pac::TIMG0;
use esp_hal_common::{interrupt, interrupt::CpuInterrupt, interrupt::Priority, Cpu, pac};
use esp32c3_hal::{Timer, prelude::*};
use riscv;

use core::cell::RefCell;


static TIMER0: Mutex<RefCell<Option<Timer<TIMG0>>>> = Mutex::new(RefCell::new(None));

pub fn configure_timer0(timg0: TIMG0) {
    let mut timer0 = Timer::new(timg0);
    timer0.disable();


    riscv::interrupt::free(|cs| {
        TIMER0.borrow(*cs).replace(Some(timer0));
    });
}

/// Grab the Interrupt enum value from a reference.
/// 
/// This is needed because Interrupt is not Copy nor Clone
fn which_interrupt(interrupt: &CpuInterrupt) -> CpuInterrupt {
    use CpuInterrupt::*;
    match interrupt {
        Interrupt1 => Interrupt1,
        Interrupt2 => Interrupt2,
        Interrupt3 => Interrupt3,
        Interrupt4 => Interrupt4,
        Interrupt5 => Interrupt5,
        Interrupt6 => Interrupt6,
        Interrupt7 => Interrupt7,
        Interrupt8 => Interrupt8,
        Interrupt9 => Interrupt9,
        Interrupt10 => Interrupt10,
        Interrupt11 => Interrupt11,
        Interrupt12 => Interrupt12,
        Interrupt13 => Interrupt13,
        Interrupt14 => Interrupt14,
        Interrupt15 => Interrupt15,
        Interrupt16 => Interrupt16,
        Interrupt17 => Interrupt17,
        Interrupt18 => Interrupt18,
        Interrupt19 => Interrupt19,
        Interrupt20 => Interrupt20,
        Interrupt21 => Interrupt21,
        Interrupt22 => Interrupt22,
        Interrupt23 => Interrupt23,
        Interrupt24 => Interrupt24,
        Interrupt25 => Interrupt25,
        Interrupt26 => Interrupt26,
        Interrupt27 => Interrupt27,
        Interrupt28 => Interrupt28,
        Interrupt29 => Interrupt29,
        Interrupt30 => Interrupt30,
        Interrupt31 => Interrupt31,
    }
}

pub fn enable_timer0_interrupt(interrupt: &CpuInterrupt, priority: Priority) {
    interrupt::enable(
        Cpu::ProCpu,
        pac::Interrupt::TG0_T0_LEVEL,
        which_interrupt(interrupt),
    );
    interrupt::set_kind(
        Cpu::ProCpu,
        which_interrupt(interrupt),
        interrupt::InterruptKind::Level,
    );
    interrupt::set_priority(
        Cpu::ProCpu,
        which_interrupt(interrupt),
        priority,
    );

    riscv::interrupt::free(|cs| {
        match TIMER0.borrow(*cs).borrow_mut().as_mut() {
            Some(timer) => {
                timer.listen();
            },
            None => {}
        }
    })
}

pub fn start_timer0(t: u64) {
    riscv::interrupt::free(|cs| {
        match TIMER0.borrow(*cs).borrow_mut().as_mut() {
            Some(timer) => {
                timer.start(t)
            },
            None => {}
        }
    })
}

pub fn clear_timer0(interrupt: CpuInterrupt) {
    interrupt::clear(Cpu::ProCpu, interrupt);
    riscv::interrupt::free(|cs| {
        match TIMER0.borrow(*cs).borrow_mut().as_mut() {
            Some(timer) => {
                timer.clear_interrupt();
            },
            None => {}
        }
    })
}
