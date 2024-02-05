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
use esp32c3_hal::timer::{Timer0, TimerGroup};
use esp32c3_hal::{clock::Clocks, peripherals::SYSTIMER};
use esp32c3_hal::{interrupt, interrupt::Priority};
use esp32c3_hal::{peripherals::TIMG1, systimer::SystemTimer};
use esp32c3_hal::{
    peripherals::{self, TIMG0},
    system::PeripheralClockControl,
};
use esp32c3_hal::{prelude::*, timer::Timer};
use esp_println::print;
use fugit::HertzU64;

use alloc::boxed::Box;
use core::cell::RefCell;

static TIMER0: Mutex<RefCell<Option<Timer<Timer0<TIMG0>>>>> = Mutex::new(RefCell::new(None));
static mut TIMER0_CALLBACK: Option<Box<dyn FnMut()>> = None;

static TIMER1: Mutex<RefCell<Option<Timer<Timer0<TIMG1>>>>> = Mutex::new(RefCell::new(None));
static mut TIMER1_CALLBACK: Option<Box<dyn FnMut()>> = None;

static mut DELAY: Option<Delay> = None;

static mut SYSTIMER: Option<SystemTimer> = None;

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

    // pub fn now(&self) -> u64 {
    //    let f = self.freq.raw();

    // }
}

#[inline]
pub fn delay(us: u64) {
    unsafe {
        if let Some(delay) = DELAY {
            delay.delay(us);
        }
    }
}

// Would fugit::TimerInstant be useful ? It's not obvious how to convert bases
pub type TimerInstant = u64; // micros

/// 16MHz timer clock
/// (16,000,000 cycles / sec) * (1 sec / 1,000,000 us) => 16 cycles / us
///
/// Return the SystemTimer clock value that is `delta` microseconds
/// from now. When SystemTimer::now() is equal to the value output
/// by `deadline` then `delta` microseconds have elapsed.
pub fn deadline(delta: u64) -> TimerInstant {
    unsafe {
        match DELAY {
            Some(delay) => delay.deadline(delta),
            // Assume 16MHz for the clock if we haven't made one I guess
            None => SystemTimer::now().wrapping_add(delta * 16),
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

const TICKS_PER_US: u64 = SystemTimer::TICKS_PER_SECOND / 1_000_000;

pub fn configure_systimer(systimer: SYSTIMER) {
    unsafe {
        SYSTIMER.replace(SystemTimer::new(systimer));
    }
}

pub fn enable_alarm_interrupts(priority: Priority) {
    interrupt::enable(peripherals::Interrupt::SYSTIMER_TARGET0, priority).unwrap();
    unsafe {
        if let Some(systimer) = &SYSTIMER {
            systimer.alarm0.interrupt_enable(true);
        }
    }
}

pub fn set_alarm0(us: u64) {
    unsafe {
        if let Some(systimer) = &SYSTIMER {
            print!("a");
            systimer.alarm0.set_target(TICKS_PER_US * us);
        }
    }
}

pub fn clear_alarm0() {
    unsafe {
        if let Some(systimer) = &SYSTIMER {
            systimer.alarm0.clear_interrupt();
            print!("x");
            // systimer.alarm0.interrupt_enable(false);
        }
    }
}

/// Initialize and disable Timer Group 0
pub fn configure_timer0(timg0: TIMG0, clocks: &Clocks, clock_ctl: &mut PeripheralClockControl) {
    let mut group0 = TimerGroup::new(timg0, clocks, clock_ctl);
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
#[link_section = ".rwtext"]
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
pub fn start_repeat_timer0_callback(t_us: u64, mut callback: impl FnMut() + 'static) {
    critical_section::with(|cs| {
        if let Some(timer) = TIMER0.borrow(cs).borrow_mut().as_mut() {
            timer.start(t_us.micros())
        }

        let f = move || {
            callback();
            start_timer0(t_us);
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

/// Initialize and disable Timer Group 0
pub fn configure_timer1(timg1: TIMG1, clocks: &Clocks, clock_ctl: &mut PeripheralClockControl) {
    let mut group0 = TimerGroup::new(timg1, clocks, clock_ctl);
    let timer1 = group0.timer0;

    group0.wdt.disable();

    critical_section::with(|cs| unsafe {
        TIMER1.borrow(cs).replace(Some(timer1));
        DELAY.replace(Delay::new(clocks));
    });
}

pub fn enable_timer1_interrupt(priority: Priority) {
    interrupt::enable(peripherals::Interrupt::TG1_T0_LEVEL, priority).unwrap();

    critical_section::with(|cs| {
        if let Some(timer) = TIMER1.borrow(cs).borrow_mut().as_mut() {
            timer.listen();
        }
    });
}

/// Start timer zero set for t microseconds
pub fn start_timer1_callback<T>(t: u64, callback: impl FnMut() + 'static) {
    critical_section::with(|cs| {
        if let Some(timer) = TIMER1.borrow(cs).borrow_mut().as_mut() {
            timer.start(t.micros())
        }

        unsafe {
            TIMER1_CALLBACK = Some(Box::new(callback));
        }
    })
}

#[allow(dead_code)]
pub fn start_repeat_timer1_callback(t_us: u64, mut callback: impl FnMut() + 'static) {
    critical_section::with(|cs| {
        if let Some(timer) = TIMER0.borrow(cs).borrow_mut().as_mut() {
            timer.start(t_us.micros())
        }

        let f = move || {
            callback();
            start_timer0(t_us);
        };

        unsafe {
            TIMER0_CALLBACK = Some(Box::new(f));
        }
    })
}

pub fn start_timer1(t: u64) {
    critical_section::with(|cs| {
        if let Some(timer) = TIMER1.borrow(cs).borrow_mut().as_mut() {
            timer.start(t.micros())
        }
    })
}

pub fn clear_timer1() {
    critical_section::with(|cs| {
        if let Some(timer) = TIMER1.borrow(cs).borrow_mut().as_mut() {
            timer.clear_interrupt();
        }
    }) // println!("+")
}

#[link_section = ".rwtext"]
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

#[interrupt]
fn TG1_T0_LEVEL() {
    // println!("timer 0 interrupt!");
    clear_timer1();

    riscv::interrupt::free(|| unsafe {
        if let Some(callback) = &mut TIMER1_CALLBACK {
            callback();
        }

        // TIMER0_CALLBACK = None;
    });
}
