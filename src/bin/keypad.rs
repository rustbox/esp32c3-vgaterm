#![no_std]
#![no_main]
#![feature(panic_info_message)]

extern crate alloc;

use alloc::vec::Vec;
use riscv_rt::entry;

use esp32c3_hal::clock::CpuClock;

use esp32c3_hal::interrupt::Priority;
use esp32c3_hal::prelude::*;

use esp32c3_hal::timer::TimerGroup;
use esp32c3_hal::{clock::ClockControl, peripherals::Peripherals};
use esp32c3_hal::{Rtc, IO};
use esp_println::{print, println};
use vgaterm::{
    ps2_keyboard,
    usb_keyboard::{self, KeyEvent, Parse, USBKeyboardDevice, END, START, US_ENGLISH},
};

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

    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);

    init_heap();

    vgaterm::configure_timer0(peripherals.TIMG0, &clocks);
    // let mut usb_report_channel0 = vgaterm::uart::configure0(peripherals.UART0);
    let mut usb_report_channel1 =
        vgaterm::uart::configure1(peripherals.UART1, io.pins.gpio2, io.pins.gpio3, &clocks);

    unsafe {
        riscv::interrupt::enable();
    }

    let _delay = vgaterm::Delay::new(&clocks);

    println!("Hello World");

    vgaterm::gpio::interrupt_enable(Priority::Priority1);
    vgaterm::uart::interrupt_enable1(Priority::Priority5);

    let mut keyboard = USBKeyboardDevice::new(US_ENGLISH);

    loop {
        'message: loop {
            while let Some(k) = usb_report_channel1.recv() {
                if let Parse::Finished(r) = keyboard.next_report_byte(k) {
                    match r {
                        Ok(m) => {
                            let events = keyboard.next_report(&m.message);
                            println!(
                                "Events: {:?}",
                                events
                                    .iter()
                                    .map(|c| {
                                        match c {
                                            KeyEvent::Pressed(k) => {
                                                KeyEvent::Pressed(keyboard.translate_keycode(*k))
                                            }
                                            KeyEvent::Released(k) => {
                                                KeyEvent::Released(keyboard.translate_keycode(*k))
                                            }
                                        }
                                    })
                                    .collect::<Vec<_>>()
                            );
                        }
                        Err(e) => {
                            println!("Error: {:?}", e);
                        }
                    }
                    break 'message;
                }
            }
            unsafe {
                riscv::asm::wfi();
            }
        }

        unsafe {
            riscv::asm::wfi();
        }
    }

    // let mut report = Vec::new();
    //     let mut collecting_report = false;
    //     'report: loop {
    //         while let Some(k) = usb_report_channel1.recv() {
    //             if k == START && !collecting_report {
    //                 print!("*");
    //                 collecting_report = true;
    //             }
    //             if collecting_report {
    //                 report.push(k);
    //                 print!(".");
    //             }
    //             if k == END {
    //                 print!("|");
    //                 break 'report;
    //             }
    //         }
    //         unsafe {
    //             print!("#");
    //             riscv::asm::wfi();
    //         }
    //     }

    //     if !report.is_empty() {
    //         match usb_keyboard::ReportHeader::from_bytes(report.as_slice()) {
    //             Ok(r) => {
    //                 let keys_pressed = keyboard.next_report(&r.message);
    //                 println!("Pressed {:?}", keys_pressed);
    //             }
    //             Err(s) => {
    //                 println!("ERROR: {:?}", s);
    //             }
    //         }
    //     }

    //     println!("Wfi");
}
