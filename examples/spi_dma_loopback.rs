//! SPI loopback test using DMA
//!
//! Folowing pins are used:
//! SCLK    GPIO6
//! MISO    GPIO2
//! MOSI    GPIO7
//! CS      GPIO10
//!
//! Depending on your target and the board you are using you have to change the
//! pins.
//!
//! This example transfers data via SPI.
//! Connect MISO and MOSI pins to see the outgoing data is read as incoming
//! data.

#![no_std]
#![no_main]

use esp32c3_hal::{
    clock::ClockControl,
    clock::CpuClock,
    dma::DmaPriority,
    gdma::Gdma,
    gpio::IO,
    peripherals::Peripherals,
    prelude::*,
    spi::{Spi, SpiMode},
    timer::TimerGroup,
    Delay, Rtc,
};
use esp_backtrace as _;

use riscv_rt::entry;

#[entry]
fn main() -> ! {
    let peripherals = Peripherals::take();
    let mut system = peripherals.SYSTEM.split();
    let clocks = ClockControl::configure(system.clock_control, CpuClock::Clock160MHz).freeze();

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

    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);

    let sio0 = io.pins.gpio7;
    let sio1 = io.pins.gpio2;
    let sio2 = io.pins.gpio5;
    let sio3 = io.pins.gpio4;
    let cs = io.pins.gpio10;
    let sclk = io.pins.gpio6;

    // let mosi = io.pins.gpio7;
    // let miso = io.pins.gpio2;
    // let cs = io.pins.gpio10;
    // let sclk = io.pins.gpio6;

    let dma = Gdma::new(peripherals.DMA, &mut system.peripheral_clock_control);
    let dma_channel = dma.channel0;

    let mut descriptors = [0u32; 8 * 3];
    let mut rx_descriptors = [0u32; 3];

    let mut spi = Spi::new_quad_send_only(
        peripherals.SPI2,
        sclk,
        sio0,
        sio1,
        sio2,
        sio3,
        cs,
        40u32.MHz(),
        SpiMode::Mode0,
        &mut system.peripheral_clock_control,
        &clocks,
    )
    .with_dma(dma_channel.configure(
        false,
        &mut descriptors,
        &mut rx_descriptors,
        DmaPriority::Priority0,
    ));

    let mut delay = Delay::new(&clocks);

    // DMA buffer require a static life-time
    let mut send = buffer1();
    let mut receive = buffer2();
    let mut i = 0;

    for (i, v) in send.iter_mut().enumerate() {
        *v = (i % 255) as u8;
    }

    loop {
        send[0] = i;
        send[send.len() - 1] = i;
        i = i.wrapping_add(1);

        let transfer = spi.dma_transfer(send, receive).unwrap();
        // here we could do something else while DMA transfer is in progress
        // the buffers and spi is moved into the transfer and we can get it back via
        // `wait`
        (receive, send, spi) = transfer.wait();
        // println!(
        //     "{:x?} .. {:x?}",
        //     &receive[..10],
        //     &receive[receive.len() - 10..]
        // );

        delay.delay_ms(250u32);
    }
}

fn buffer1() -> &'static mut [u8; 32000] {
    static mut BUFFER: [u8; 32000] = [0u8; 32000];
    unsafe { &mut BUFFER }
}

fn buffer2() -> &'static mut [u8; 0] {
    static mut BUFFER: [u8; 0] = [0u8; 0];
    unsafe { &mut BUFFER }
}
