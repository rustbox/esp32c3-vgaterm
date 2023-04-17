#![no_std]
#![no_main]
#![feature(panic_info_message)]

extern crate alloc;

use alloc::{collections::VecDeque, string::String, vec::Vec};
use esp32c3_hal::clock::{ClockControl, CpuClock};
use esp32c3_hal::prelude::*;
use esp32c3_hal::timer::TimerGroup;
use esp32c3_hal::{gpio::IO, peripherals::Peripherals, Rtc};
use esp_backtrace as _;
use esp_println::println;
use vgaterm::{self, video};
use vgaterm::{interrupt::Priority, usb_keyboard::US_ENGLISH, Delay, Work};

use core::{arch::asm, fmt::Write};

core::arch::global_asm!(".global _heap_size; _heap_size = 0xB000");

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

#[no_mangle]
extern "C" fn stop() -> ! {
    loop {
        unsafe {
            riscv::asm::wfi();
        }
    }
}

static mut NUM_BYTES: usize = 0;

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

    // io.pins.gpio20.into_floating_input();

    init_heap();
    configure_counter_for_cpu_cycles();

    vgaterm::configure_timer0(peripherals.TIMG0, &clocks);
    // let mut host_recv = vgaterm::uart::configure0(peripherals.UART0);
    let mut serial0 = vgaterm::uart::make_uart0(peripherals.UART0);
    serial0.set_rx_fifo_full_threshold(1);
    serial0.listen_rx_fifo_full();
    vgaterm::enable_timer0_interrupt(Priority::Priority5);
    vgaterm::uart::interrupt_enable0(Priority::Priority6);
    vgaterm::gpio::interrupt_enable(Priority::max());

    unsafe {
        riscv::interrupt::enable();
    }

    vgaterm::timer::start_repeat_timer0_callback(1_000_000, || unsafe {
        if NUM_BYTES > 0 {
            println!("{} bytes", NUM_BYTES);
            println!(
                "{} draw cycles per byte",
                vgaterm::CHARACTER_DRAW_CYCLES as f32 / NUM_BYTES as f32
            );
            NUM_BYTES = 0;
        }
        vgaterm::CHARACTER_DRAW_CYCLES = 0;
    });

    // Initialize the Delay peripheral, and use it to toggle the LED state in a
    // loop.
    let _delay = vgaterm::Delay::new(&clocks);

    // println!("Starting timer");
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
        peripherals.DMA,
        &mut system.peripheral_clock_control,
        &clocks,
        40_000_000,
    );

    let image = include_bytes!("../../image.bin");
    video::load_from_slice(image);

    let mut display = vgaterm::display::Display::new();

    // println!("Done");
    // println!("Clock speed: {} Hz", measure_clock(delay));
    vgaterm::kernel::start(io.pins.gpio3);

    // let mut text_display = vgaterm::display::TextDisplay::new();
    let mut terminal = vgaterm::terminal::TextField::new();
    // text_display.write_text(0, vgaterm::display::COLUMNS / 2 - 4, " WELCOME!");
    // text_display.write_text(1, 0, " Welcome, Aly and Ilana, to Chez Douglass, where we will enjoy food, company, drink, and new friendships!");
    // text_display.draw_dirty(&mut display);
    // display.flush();

    // let mut cursor = (0, 0);
    // terminal
    let mut keyboard = vgaterm::keyboard::Keyboard::from_peripherals(
        US_ENGLISH,
        io.pins.gpio1,
        io.pins.gpio0,
        peripherals.UART1,
        &clocks,
    );

    let mut keyvents = VecDeque::new();
    let mut key_state = vgaterm::keyboard::PressedSet::new();
    let mut input = vgaterm::terminal_input::TerminalInput::new(300, 40);

    #[allow(unused)]
    enum ConnectMode {
        LocalEcho,
        ConnectHost,
        None,
    }

    let mode = ConnectMode::ConnectHost;

    loop {
        keyvents.extend(keyboard.flush_and_parse());
        if let Some(kevent) = keyvents.pop_front() {
            key_state.push(kevent);
        }

        let h = {
            let mut b = Vec::new();
            while let Ok(r) = serial0.read() {
                b.push(r);
            }
            unsafe {
                NUM_BYTES += b.len();
            }
            b
        };

        terminal.type_str(String::from_utf8_lossy(&h).as_ref());

        let last_char = input.key_char(&key_state);
        match last_char {
            Work::Item(ref c) => {
                match mode {
                    ConnectMode::ConnectHost => {
                        let _ = serial0.write_str(c);
                    }
                    ConnectMode::LocalEcho => {
                        terminal.type_str(c);
                    }
                    ConnectMode::None => {}
                };
            }
            Work::WouldBlock => {}
            Work::WouldBlockUntil(_) => {}
        }

        // Draw the characters on the frame
        terminal.draw(&mut display);
        // Flush the Display to the BUFFER
        display.flush();

        if !keyvents.is_empty() {
            println!("{:?}", keyvents);
            continue;
        }

        unsafe {
            // this will fire no less often than once per frame
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
