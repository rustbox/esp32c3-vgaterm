#![no_std]
#![no_main]

use esp32c3_hal::{gpio::IO, pac::Peripherals, prelude::*, Delay, RtcCntl, Timer};
use esp_hal_common::interrupt;
use panic_halt as _;
use riscv_rt::entry;

use vgaterm;
use vgaterm::sprintln;

#[entry]
fn main() -> ! {
    let peripherals = Peripherals::take().unwrap();

    // Disable the watchdog timers. For the ESP32-C3, this includes the Super WDT,
    // the RTC WDT, and the TIMG WDTs.
    let mut rtc_cntl = RtcCntl::new(peripherals.RTC_CNTL);
    let mut timer1 = Timer::new(peripherals.TIMG1);
    
    rtc_cntl.set_super_wdt_enable(false);
    rtc_cntl.set_wdt_enable(false);
    timer1.disable();
    
    vgaterm::configure(peripherals.UART0);
    vgaterm::configure_timer0(peripherals.TIMG0);

    vgaterm::enable_timer0_interrupt(
        &interrupt::CpuInterrupt::Interrupt1, 
        interrupt::Priority::Priority1
    );

    vgaterm::start_timer0(10_000_000);

    unsafe {
        riscv::interrupt::enable();
    }
    
    // Set GPIO5 as an output, and set its state high initially.
    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);
    let mut led = io.pins.gpio5.into_push_pull_output();

    led.set_high().unwrap();

    // Initialize the Delay peripheral, and use it to toggle the LED state in a
    // loop.
    let mut delay = Delay::new(peripherals.SYSTIMER);

    sprintln!("Hello, World!");
    sprintln!("Say hello: \"{}\"", vgaterm::hello());

    loop {
        led.toggle().unwrap();
        delay.delay_ms(500u32);
    }
}

#[no_mangle]
pub fn interrupt1() {
    riscv::interrupt::free(|_| {
        vgaterm::clear_timer0(interrupt::CpuInterrupt::Interrupt1);
        sprintln!("Interrupt 1");
        
        vgaterm::start_timer0(10_000_000);
    });
}

