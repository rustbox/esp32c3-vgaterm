#![no_std]
#![no_main]
#![feature(panic_info_message)]

extern crate alloc;

use riscv_rt::entry;

use esp32c3_hal::systimer::SystemTimer;
use esp32c3_hal::interrupt::Priority;
use esp32c3_hal::gpio::Event;
use esp32c3_hal::clock::CpuClock;
use esp32c3_hal::timer::TimerGroup;
use esp32c3_hal::{peripherals::Peripherals, clock::ClockControl};
use esp32c3_hal::{prelude::*, Rtc, IO};
use esp_println::{println, print};
use vgaterm::keyboard;

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

// static mut BUTTON: Mutex<RefCell<Option<Gpio9<Input<PullDown>>>>> = Mutex::new(RefCell::new(None));
// static mut BUTTON2: Mutex<RefCell<Option<Gpio10<Input<PullDown>>>>> = Mutex::new(RefCell::new(None));

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
    let mut system = peripherals.SYSTEM.split();
    let clocks = ClockControl::configure(system.clock_control, CpuClock::Clock160MHz).freeze();

    // Disable the watchdog timers. For the ESP32-C3, this includes the Super WDT,
    // the RTC WDT, and the TIMG WDTs.
    let mut rtc = Rtc::new(peripherals.RTC_CNTL);

    let timer_group1 = TimerGroup::new(peripherals.TIMG1, &clocks);
    let mut wdt1 = timer_group1.wdt;

    rtc.swd.disable();
    rtc.rwdt.disable();
    wdt1.disable();

    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);

    init_heap();

    vgaterm::configure_timer0(peripherals.TIMG0, &clocks);

    unsafe {
        riscv::interrupt::enable();
    }

    let _delay = vgaterm::Delay::new(&clocks);

    println!("Hello World");

    let clk = io.pins.gpio6.into_pull_up_input();
    let data = io.pins.gpio8.into_pull_up_input();
    let data_pin = data.number();

    vgaterm::gpio::interrupt_enable(Priority::Priority1);

    

    loop {
        unsafe {
            riscv::asm::wfi();
        }
    }
}
