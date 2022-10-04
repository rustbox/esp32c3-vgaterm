#![no_std]
#![no_main]
#![feature(panic_info_message)]

extern crate alloc;

use alloc::format;
use alloc::vec;

use esp32c3_hal::clock::{ClockControl, CpuClock};
use esp32c3_hal::prelude::*;
use esp32c3_hal::timer::TimerGroup;
use esp32c3_hal::{gpio::IO, pac::Peripherals, Rtc};

use esp_hal_common::{Event, Priority};
use riscv_rt::entry;

use vgaterm;
use vgaterm::video::BUFFER;
use vgaterm::{sprint, sprintln, Delay};

use core::arch::asm;

#[global_allocator]
static ALLOCATOR: esp_alloc::EspHeap = esp_alloc::EspHeap::empty();

fn init_heap() {
    const HEAP_SIZE: usize = 32 * 1024;

    extern "C" {
        static mut _heap_start: u32;
    }

    unsafe {
        let heap_start = &_heap_start as *const _ as usize;
        ALLOCATOR.init(heap_start as *mut u8, HEAP_SIZE);
    }
}

// static mut BUTTON: Mutex<RefCell<Option<Gpio9<Input<PullDown>>>>> = Mutex::new(RefCell::new(None));
// static mut BUTTON2: Mutex<RefCell<Option<Gpio10<Input<PullDown>>>>> = Mutex::new(RefCell::new(None));

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    sprint!("Aborting: ");
    if let Some(p) = info.location() {
        sprintln!(
            "line {}, file {}: {}",
            p.line(),
            p.file(),
            info.message().unwrap()
        );
    } else {
        sprintln!("no information available.");
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
    let peripherals = Peripherals::take().unwrap();
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
    configure_count_cycles();
    vgaterm::configure_timer0(peripherals.TIMG0, &clocks);
    vgaterm::enable_timer0_interrupt(Priority::Priority1);
    vgaterm::configure(peripherals.UART0);
    vgaterm::gpio::interrupt_enable(Priority::Priority2);

    unsafe {
        riscv::interrupt::enable();
    }

    // vgaterm::gpio::interrupt_disable(enabled);

    // led.set_high().unwrap();

    // Initialize the Delay peripheral, and use it to toggle the LED state in a
    // loop.
    let delay = vgaterm::Delay::new(&clocks);

    // sprintln!("Starting timer");
    // vgaterm::start_timer0(1_000);

    let sio0 = io.pins.gpio7;
    let sio1 = io.pins.gpio2;
    let sio2 = io.pins.gpio5;
    let sio3 = io.pins.gpio4;
    let cs = io.pins.gpio10;
    let clk = io.pins.gpio6;

    vgaterm::spi::configure(
        peripherals.SPI2,
        sio0,
        sio1,
        sio2,
        sio3,
        cs,
        clk,
        &mut system.peripheral_clock_control,
        &clocks,
        80_000_000,
    );
    // White: 0xFF
    // Red: 0x03
    // Green: 0x1C
    // Blue: 0x60, 0xE0
    // vgaterm::video::load_test_pattern(0x1C, 0x1C);
    riscv::interrupt::free(|_| unsafe {
        for l in 0..vgaterm::video::HEIGHT {
            for p in 0..vgaterm::video::WIDTH {
                let i = vgaterm::video::WIDTH * l + p;
                // if l <= 6  {
                //     BUFFER[i] = 0xFF;
                // } else if (l - 2) % 10 < 5  {
                //     // Yellow
                //     BUFFER[i] = 0x1F;
                // } else {
                //     BUFFER[i] = 0x00;
                // }
                // if p < 20 {
                //     BUFFER[i] = 0x03;
                // }
                if p == 320 {
                    BUFFER[i] = 0xFF;
                } else {
                    BUFFER[i] = 0x00;
                }
            }
        }
    });
    // vgaterm::gpio::pin_interrupt(io.pins.gpio3.into_floating_input(), Event::FallingEdge, |_| {
    //     sprint!(".");
    //     vgaterm::start_timer0_callback(1000, || {
    //         sprintln!("*");
    //         // let d = Delay::new(&clocks);
    //         vgaterm::kernel::frame();
    //     })
    // });

    // vgaterm::start_timer0_callback(1_000_000, || {
    //     sprintln!("one second!")
    // });
    // vgaterm::kernel::start(io.pins.gpio3);
    // let mut hi = io.pins.gpio3.into_push_pull_output();
    // let mut count = 0;
    // vgaterm::kernel::start(io.pins.gpio3);

    loop {
        unsafe {
            sprintln!("hello");

            vgaterm::start_cycle_count();
            vgaterm::kernel::frame();
            let m = vgaterm::measure_cycle_count();

            sprintln!("Frame took {} cycles", m);
            // let _ = hi.set_high();
            // vgaterm::spi::transmit(&[0xff; 64]);
            // let _ = hi.set_low();
            // count += 1;
            // sprintln!("Waiting {}", count);
            delay.delay_ms(1000);
            // riscv::asm::wfi();
        }
    }
}

struct Mems<'a>(usize, &'a [usize]);

fn show_registers<'a>(addr_start: usize, end_offset: usize) -> Mems<'a> {
    let raw = addr_start as *const usize;
    sprintln!("Made raw pointer");
    let slice = unsafe { core::slice::from_raw_parts(raw, end_offset / 4) };
    sprintln!("Made slice");
    Mems(addr_start, slice)
}

impl<'a> core::fmt::Display for Mems<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        for (i, reg) in self.1.iter().enumerate() {
            let _ = write!(f, "{:#04x}: {:#032b}", self.0 + 4 * i, reg);
        }
        Ok(())
    }
}

#[no_mangle]
fn configure_count_cycles() {
    unsafe {
        // Set count event to clock cycles
        // Enable counting events and set overflow to rollover
        asm!("csrwi 0x7E0, 0x1", "csrwi 0x7E1, 0x1");
    }
}

#[no_mangle]
fn measure_clock(delay: Delay) -> u32 {
    unsafe {
        // Set event counter to 0
        asm!("csrwi 0x7E2, 0x00",)
    }
    let d: u32;
    delay.delay_ms(1000);
    unsafe {
        asm!(
            "csrr {}, 0x7E2",
            out(reg) d
        );
    }
    d
}

// #[interrupt]
// pub unsafe fn TG0_T0_LEVEL() {
//     riscv::interrupt::free(|_| {
//         vgaterm::clear_timer0(interrupt::CpuInterrupt::Interrupt1);
//         // sprintln!("Interrupt 1");

//         vgaterm::start_timer0(10_000_000);
//     });
// }

// #[interrupt]
// pub fn interrupt3() {
//     riscv::interrupt::free(|cs| unsafe {
//         sprintln!("Some GPIO interrupt!");

//         let mut button = BUTTON.borrow(*cs).borrow_mut();
//         let button = button.as_mut().unwrap();

//         let mut button2 = BUTTON2.borrow(*cs).borrow_mut();
//         let button2 = button2.as_mut().unwrap();

//         sprintln!("Interrupt source: {:?}", vgaterm::interrupt::source());
//         sprintln!("GPIO Pin: {}", vgaterm::check_gpio_source());

//         vgaterm::interrupt::clear(interrupt::CpuInterrupt::Interrupt3);

//         button.clear_interrupt();
//         button2.clear_interrupt();
//     });
// }
