#![no_std]
#![no_main]
#![feature(panic_info_message)]

extern crate alloc;

use esp32c3_hal::prelude::*;
use esp32c3_hal::{
    clock::ClockControl,
    gpio::{Event, IO},
    interrupt,
    peripherals::{self, Peripherals},
    timer::TimerGroup,
    Rtc,
};
use esp_backtrace as _;
use esp_println::println;
use vgaterm::gpio::{interrupt_disable, pin_interrupt};

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

#[entry]
fn main() -> ! {
    let peripherals = Peripherals::take();
    let mut system = peripherals.SYSTEM.split();
    let clocks = ClockControl::boot_defaults(system.clock_control).freeze();

    // Disable the watchdog timers. For the ESP32-C3, this includes the Super WDT,
    // the RTC WDT, and the TIMG WDTs.
    let mut rtc = Rtc::new(peripherals.RTC_CNTL);
    let timer_group0 = TimerGroup::new(
        peripherals.TIMG0,
        &clocks,
        &mut system.peripheral_clock_control,
    );
    let mut wdt0 = timer_group0.wdt;
    let timer_group1 = TimerGroup::new(
        peripherals.TIMG1,
        &clocks,
        &mut system.peripheral_clock_control,
    );
    let mut wdt1 = timer_group1.wdt;

    rtc.swd.disable();
    rtc.rwdt.disable();
    wdt0.disable();
    wdt1.disable();

    init_heap();

    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);

    let mut led = io.pins.gpio5.into_push_pull_output();

    // fn callback<T>(_: T) {
    //     esp_println::println!("GPIO interrupt");
    //     // led.toggle().unwrap();
    // }

    // let mut pref = vgaterm::gpio::pin_interrupt(
    //     io.pins.gpio9.into_pull_down_input(),
    //     Event::FallingEdge,
    //     callback,
    // );

    interrupt::enable(peripherals::Interrupt::GPIO, interrupt::Priority::Priority3).unwrap();

    unsafe {
        riscv::interrupt::enable();
    }

    loop {
        unsafe { riscv::asm::wfi() };

        // let (input, event, callback) = interrupt_disable(pref);
        // println!("I now hold pin {}", input.number());
        // pref = pin_interrupt(input, event, callback);
    }
}
