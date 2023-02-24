#![no_std]
#![no_main]
#![feature(panic_info_message)]

extern crate alloc;

use core::{borrow::Borrow, cell::RefCell};

use critical_section::Mutex;
use esp_println::println;

use esp32c3_hal::{
    clock::ClockControl,
    gpio::{Event, IO},
    interrupt,
    peripherals::{self, Peripherals, TIMG1},
    timer::{Timer, TimerGroup},
    Delay, Rtc,
};
use esp32c3_hal::{
    gpio::{Gpio9, Input, PullDown},
    peripherals::TIMG0,
    prelude::*,
    timer::Timer0,
};
use esp_backtrace as _;

core::arch::global_asm!(".global _heap_size; _heap_size = 0x8000");

#[global_allocator]
static ALLOCATOR: esp_alloc::EspHeap = esp_alloc::EspHeap::empty();

fn init_heap() {
    extern "C" {
        static mut _heap_size: u32;
        static mut _sheap: u32;
    }

    unsafe {
        let heap_start = &_sheap as *const _ as usize;
        let heap_size = &_heap_size as *const _ as usize;
        ALLOCATOR.init(heap_start as *mut u8, heap_size);
    }
}

static GPIO9: Mutex<RefCell<Option<Gpio9<Input<PullDown>>>>> = Mutex::new(RefCell::new(None));
static TIMER0: Mutex<RefCell<Option<Timer<Timer0<TIMG0>>>>> = Mutex::new(RefCell::new(None));
static TIMER1: Mutex<RefCell<Option<Timer<Timer0<TIMG1>>>>> = Mutex::new(RefCell::new(None));

static mut DELAY: Option<Delay> = None;

#[entry]
fn main() -> ! {
    let peripherals = Peripherals::take();
    let system = peripherals.SYSTEM.split();
    let clocks = ClockControl::boot_defaults(system.clock_control).freeze();

    // Disable the watchdog timers. For the ESP32-C3, this includes the Super WDT,
    // the RTC WDT, and the TIMG WDTs.
    let mut rtc = Rtc::new(peripherals.RTC_CNTL);
    let timer_group0 = TimerGroup::new(peripherals.TIMG0, &clocks);
    let mut wdt0 = timer_group0.wdt;
    let timer_group1 = TimerGroup::new(peripherals.TIMG1, &clocks);
    let mut wdt1 = timer_group1.wdt;

    rtc.swd.disable();
    rtc.rwdt.disable();
    wdt0.disable();
    wdt1.disable();

    init_heap();

    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);

    let mut gpio9 = io.pins.gpio9.into_pull_down_input();
    gpio9.listen(Event::FallingEdge);
    interrupt::enable(peripherals::Interrupt::GPIO, interrupt::Priority::Priority3).unwrap();

    let mut timer0 = timer_group0.timer0;
    let mut timer1 = timer_group1.timer0;

    interrupt::enable(
        peripherals::Interrupt::TG0_T0_LEVEL,
        interrupt::Priority::Priority1,
    )
    .unwrap();

    timer0.start(500u64.millis());
    timer0.listen();

    interrupt::enable(
        peripherals::Interrupt::TG1_T0_LEVEL,
        interrupt::Priority::Priority2,
    )
    .unwrap();
    timer1.start(1u64.secs());
    timer1.listen();

    critical_section::with(|cs| {
        GPIO9.borrow_ref_mut(cs).replace(gpio9);
        TIMER0.borrow_ref_mut(cs).replace(timer0);
        TIMER1.borrow_ref_mut(cs).replace(timer1);
    });

    unsafe { &mut DELAY }.replace(Delay::new(&clocks));

    unsafe {
        riscv::interrupt::enable();
    }

    loop {
        unsafe { riscv::asm::wfi() }
    }
}

#[interrupt]
fn GPIO() {
    // interrupt::TrapFrame

    // riscv::asm::ebreak()

    static DEPTH: Mutex<RefCell<u16>> = Mutex::new(RefCell::new(0u16));

    let d = critical_section::with(|cs| {
        let mut d = DEPTH.borrow_ref_mut(cs);

        let r = *d;

        *d += 1;

        r
    });

    esp_println::println!("!--{}-- GPIO interrupt {d}", "--".repeat(d.into()));

    critical_section::with(|cs| {
        let mut gpio9 = GPIO9.borrow_ref_mut(cs);
        let gpio9 = gpio9.as_mut().unwrap();

        gpio9.clear_interrupt();
    });

    unsafe {
        riscv::interrupt::enable();
    }

    // safety: Delay is just wrapping a (frozen) cycle count and won't be changed
    unsafe { &DELAY }.as_ref().unwrap().delay(250_000u32);

    esp_println::println!(" --{}-- GPIO interrupt {d} ---!", "--".repeat(d.into()));

    // TODO ?
    // unsafe {
    //     riscv::interrupt::disable();
    // }

    critical_section::with(|cs| {
        *DEPTH.borrow_ref_mut(cs) -= 1;
    });
}

#[interrupt]
fn TG0_T0_LEVEL() {
    esp_println::println!("Timer 0 isr start");

    critical_section::with(|cs| {
        let mut timer0 = TIMER0.borrow_ref_mut(cs);
        let timer0 = timer0.as_mut().unwrap();

        timer0.clear_interrupt();

        critical_section::with(|cs| {
            TIMER1
                .borrow_ref_mut(cs)
                .as_mut()
                .unwrap()
                .start(500u64.millis());
        });
    });

    unsafe {
        riscv::interrupt::enable();
    }

    // safety: Delay is just wrapping a (frozen) cycle count and won't be changed
    unsafe { &DELAY }.as_ref().unwrap().delay(2_500_000u32);

    critical_section::with(|cs| {
        TIMER0
            .borrow_ref_mut(cs)
            .as_mut()
            .unwrap()
            .start(2u64.secs())
    });

    // safety: Delay is just wrapping a (frozen) cycle count and won't be changed
    unsafe { &DELAY }.as_ref().unwrap().delay(500_000u32);

    esp_println::println!("Timer 0 isr end");
}

#[interrupt]
fn TG1_T0_LEVEL() {
    esp_println::println!("-- Timer 1 isr start");

    critical_section::with(|cs| {
        let mut timer1 = TIMER1.borrow_ref_mut(cs);
        let timer1 = timer1.as_mut().unwrap();

        timer1.clear_interrupt();
    });

    unsafe {
        riscv::interrupt::enable();
    }

    // safety: Delay is just wrapping a (frozen) cycle count and won't be changed
    unsafe { &DELAY }.as_ref().unwrap().delay(2_000_000u32);

    esp_println::println!("-- Timer 1 isr end");
}
