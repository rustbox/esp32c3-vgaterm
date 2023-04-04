#![no_std]
#![no_main]
#![feature(panic_info_message)]

extern crate alloc;

use esp32c3_hal::prelude::*;
use esp32c3_hal::{
    clock::{ClockControl, CpuClock},
    peripherals::Peripherals,
    timer::TimerGroup,
    Rtc, IO,
};
use esp_println::{print, println};
use riscv::asm::wfi;
use vgaterm::ansi::{OpChar, TerminalEsc};

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

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    print!("Aborting: ");
    if let Some(p) = info.location() {
        println!(
            "line {}, file {}: {}",
            p.line(),
            p.file(),
            info.message().unwrap()
        );
    } else {
        println!("no information available.");
    }
    stop();
}

#[no_mangle]
extern "C" fn stop() -> ! {
    loop {
        unsafe {
            riscv::asm::wfi();
        }
    }
}

#[entry]
fn main() -> ! {
    let peripherals = Peripherals::take();
    let system = peripherals.SYSTEM.split();
    let clocks = ClockControl::configure(system.clock_control, CpuClock::Clock160MHz).freeze();

    // Disable the watchdog timers. For the ESP32-C3, this includes the Super WDT,
    // the RTC WDT, and the TIMG WDTs.
    let mut rtc = Rtc::new(peripherals.RTC_CNTL);

    let timer_group1 = TimerGroup::new(peripherals.TIMG1, &clocks);
    let mut wdt1 = timer_group1.wdt;

    rtc.swd.disable();
    rtc.rwdt.disable();
    wdt1.disable();

    let _io = IO::new(peripherals.GPIO, peripherals.IO_MUX);

    init_heap();

    vgaterm::configure_timer0(peripherals.TIMG0, &clocks);
    let mut rx = vgaterm::uart::configure0(peripherals.UART0);
    vgaterm::uart::interrupt_enable0(vgaterm::interrupt::Priority::Priority5);

    unsafe {
        riscv::interrupt::enable();
    }

    let mut escape = TerminalEsc::new();

    loop {
        while let Some(c) = rx.recv() {
            match escape.push(c) {
                None => print!("{}", c.escape_default()),
                Some(OpChar::Char(h)) => println!("> {h}"),
                Some(OpChar::Op(p)) => println!("{c}\nOp: {p:?}"),
            }
        }

        unsafe {
            wfi();
        }
    }
}
