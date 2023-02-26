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
extern crate alloc;

use critical_section::Mutex;
use esp32c3_hal::clock::Clocks;
use esp32c3_hal::peripherals::{self, TIMG0};
use esp32c3_hal::systimer::SystemTimer;
use esp32c3_hal::timer::{Timer0, TimerGroup};
use esp32c3_hal::{interrupt, interrupt::Priority};
use esp32c3_hal::{prelude::*, timer::Timer};
use fugit::HertzU64;

use alloc::boxed::Box;
use core::cell::RefCell;

static TIMER0: Mutex<RefCell<Option<Timer<Timer0<TIMG0>>>>> = Mutex::new(RefCell::new(None));
static mut TIMER0_CALLBACK: Option<Box<dyn FnMut()>> = None;
static mut DELAY: Option<Delay> = None;

/// Uses the `SYSTIMER` peripheral for counting clock cycles, as
/// unfortunately the ESP32-C3 does NOT implement the `mcycle` CSR, which is
/// how we would normally do this.
#[derive(Copy, Clone)]
pub struct Delay {
    pub freq: HertzU64,
}

impl Delay {
    /// Create a new Delay instance
    pub fn new(clocks: &Clocks) -> Self {
        // The counters and comparators are driven using `XTAL_CLK`. The average clock
        // frequency is fXTAL_CLK/2.5, which is 16 MHz. The timer counting is
        // incremented by 1/16 μs on each `CNT_CLK` cycle.

        Self {
            freq: HertzU64::MHz((clocks.xtal_clock.to_MHz() * 10 / 25) as u64),
        }
    }

    /// Builds a Delay struct directly
    pub fn from_freq(freq_mhz: u64) -> Self {
        Self {
            freq: HertzU64::MHz(freq_mhz),
        }
    }

    /// Delay for the specified number of microseconds
    pub fn delay(&self, us: u64) {
        let t0 = SystemTimer::now();
        let clocks = (us * self.freq.raw()) / HertzU64::MHz(1).raw();

        while SystemTimer::now().wrapping_sub(t0) <= clocks {}
    }

    pub fn delay_ms(&self, ms: u64) {
        let us = 1000 * ms;
        let t0 = SystemTimer::now();
        let clocks = (us * self.freq.raw()) / HertzU64::MHz(1).raw();

        while SystemTimer::now().wrapping_sub(t0) <= clocks {}
    }

    pub fn deadline(&self, us: u64) -> u64 {
        let clocks = (us * self.freq.raw()) / HertzU64::MHz(1).raw();
        SystemTimer::now().wrapping_add(clocks)
    }

    pub fn wait_until(&self, deadline: u64) {
        while SystemTimer::now() <= deadline {}
    }
}

#[inline]
pub fn delay(us: u64) {
    unsafe {
        if let Some(delay) = DELAY {
            delay.delay(us);
        }
    }
}

/// 16MHz timer clock
/// (16,000,000 cycles / sec) * (1 sec / 1,000,000 us) => 16 cycles / us
pub fn deadline(us: u64) -> u64 {
    unsafe {
        match DELAY {
            Some(delay) => delay.deadline(us),
            // Assume 16MHz for the clock if we haven't made one I guess
            None => SystemTimer::now().wrapping_add(us * 16),
        }
    }
}

pub fn wait_until(deadline: u64) {
    unsafe {
        match DELAY {
            Some(delay) => delay.wait_until(deadline),
            None => while SystemTimer::now() <= deadline {},
        }
    }
}

/// Initialize and disable Timer Group 0
pub fn configure_timer0(timg0: TIMG0, clocks: &Clocks) {
    let mut group0 = TimerGroup::new(timg0, clocks);
    let timer0 = group0.timer0;

    group0.wdt.disable();

    critical_section::with(|cs| unsafe {
        TIMER0.borrow(cs).replace(Some(timer0));
        DELAY.replace(Delay::new(clocks));
    });
}

pub fn enable_timer0_interrupt(priority: Priority) {
    interrupt::enable(peripherals::Interrupt::TG0_T0_LEVEL, priority).unwrap();

    critical_section::with(|cs| {
        if let Some(timer) = TIMER0.borrow(cs).borrow_mut().as_mut() {
            timer.listen();
        }
    });
}

/// Start timer zero set for t microseconds
pub fn start_timer0_callback(t: u64, callback: impl FnMut() + 'static) {
    critical_section::with(|cs| {
        if let Some(timer) = TIMER0.borrow(cs).borrow_mut().as_mut() {
            timer.start(t.micros())
        }

        unsafe {
            TIMER0_CALLBACK = Some(Box::new(callback));
        }
    })
}

#[allow(dead_code)]
pub fn start_repeat_timer0_callback(t: u64, mut callback: impl FnMut() + 'static) {
    critical_section::with(|cs| {
        if let Some(timer) = TIMER0.borrow(cs).borrow_mut().as_mut() {
            timer.start(t.micros())
        }

        let f = move || {
            callback();
            start_timer0(t);
        };

        unsafe {
            TIMER0_CALLBACK = Some(Box::new(f));
        }
    })
}

pub fn start_timer0(t: u64) {
    critical_section::with(|cs| {
        if let Some(timer) = TIMER0.borrow(cs).borrow_mut().as_mut() {
            timer.start(t.micros())
        }
    })
}

pub fn clear_timer0() {
    critical_section::with(|cs| {
        if let Some(timer) = TIMER0.borrow(cs).borrow_mut().as_mut() {
            timer.clear_interrupt();
        }
    }) // println!("+")
}

#[interrupt]
fn TG0_T0_LEVEL() {
    // println!("timer 0 interrupt!");
    clear_timer0();

    riscv::interrupt::free(|| unsafe {
        if let Some(callback) = &mut TIMER0_CALLBACK {
            callback();
        }

        // TIMER0_CALLBACK = None;
    });
}
