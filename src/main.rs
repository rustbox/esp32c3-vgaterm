#![no_std]
#![no_main]
#![feature(panic_info_message)]

extern crate alloc;

use esp32c3_hal::clock::{ClockControl, CpuClock};
use esp32c3_hal::prelude::*;
use esp32c3_hal::timer::TimerGroup;
use esp32c3_hal::{gpio::IO, pac::Peripherals, Rtc};

use esp_hal_common::Priority;
use riscv_rt::entry;

use vgaterm;
use vgaterm::video;
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

    configure_counter_for_cpu_cycles();

    vgaterm::configure(peripherals.UART0);
    vgaterm::configure_timer0(peripherals.TIMG0, &clocks);
    vgaterm::enable_timer0_interrupt(Priority::Priority1);
    vgaterm::gpio::interrupt_enable(Priority::Priority2);

    unsafe {
        riscv::interrupt::enable();
    }



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

    // This is a debugging signal that goes high during the portion
    // of the visible frame, according to what the CPU believes
    let mut frame_signal = io.pins.gpio8.into_push_pull_output();
    let _ = frame_signal.set_low();

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
        40_000_000,
    );
    // White: 0xFF
    // Red: 0x03
    // Green: 0x1C
    // Blue: 0x60, 0xE0
    let mut pattern: [u8; 128] = [0; 128];
    for h in 0..8 {
        for l in 0..8 {
            for p in 0..2 {
                let value: u8 = h << 5 | p << 4 | l << 1 | p;
                let i: usize = (p + 2*l + 16*h).into();
                pattern[i] = value;
            }
        }
    }

    riscv::interrupt::free(|_| unsafe {
        for line in 0..video::HEIGHT {
            for p in 0..video::WIDTH {
                let i = line * video::WIDTH + p;
                
                if p < 160 {
                    video::BUFFER[i] = 255;
                }

                if p >= 160 && p < 320 {
                    video::BUFFER[i] = 0b0110_0110;
                }

                if p >= 320 && p < 480 {
                    video::BUFFER[i] = 0b0111_0111;
                }

                if p >= 480 && p < 640 {
                    video::BUFFER[i] = 0b0001_0001;
                }
            }
        }
    });
    // vgaterm::video::load_test_pattern(224, 224);
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
    sprintln!("Clock speed: {} Hz", measure_clock(delay));

    sprintln!("Transmitting pattern");
    vgaterm::kernel::start(io.pins.gpio3);


    loop {

        unsafe {
            riscv::asm::wfi();

        }
    }

}


///
/// Configure the esp32c2 custom Control and Status register
/// `mpcer` to count only CPU clock cycles.
/// 
/// Page 28, https://www.espressif.com/sites/default/files/documentation/esp32-c3_technical_reference_manual_en.pdf
#[no_mangle]
fn configure_counter_for_cpu_cycles() {
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

