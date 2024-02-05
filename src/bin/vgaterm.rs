#![no_std]
#![no_main]
#![feature(panic_info_message)]

extern crate alloc;

use alloc::{boxed::Box, collections::VecDeque, string::String, vec::Vec};
use esp32c3_hal::prelude::*;
use esp32c3_hal::timer::TimerGroup;
use esp32c3_hal::{
    clock::{ClockControl, CpuClock},
    peripherals::RNG,
    Rng,
};
use esp32c3_hal::{gpio::IO, peripherals::Peripherals, Rtc};
use esp_backtrace as _;
use esp_println::println;
use riscv::interrupt::free;
use vgaterm::{
    self,
    album::Album,
    perf,
    video::{self},
};
use vgaterm::{interrupt::Priority, usb_keyboard::US_ENGLISH, Work};

use core::fmt::Write;

core::arch::global_asm!(".global _heap_size; _heap_size = 0xC000");

#[global_allocator]
static ALLOCATOR: esp_alloc::EspHeap = esp_alloc::EspHeap::empty();

extern "C" {
    static mut _heap_size: u32;
}

fn init_heap() {
    extern "C" {
        // static mut _heap_size: u32;
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

    let timer_group1 = TimerGroup::new(
        peripherals.TIMG1,
        &clocks,
        &mut system.peripheral_clock_control,
    );
    let mut wdt1 = timer_group1.wdt;

    rtc.swd.disable();
    rtc.rwdt.disable();
    wdt1.disable();

    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);

    init_heap();
    perf::configure_counter_for_cpu_cycles();

    vgaterm::configure_timer0(
        peripherals.TIMG0,
        &clocks,
        &mut system.peripheral_clock_control,
    );
    // vgaterm::timer::configure_timer1(peripherals.TIMG1, &clocks, &mut system.peripheral_clock_control,);
    vgaterm::timer::configure_systimer(peripherals.SYSTIMER);
    // let mut host_recv = vgaterm::uart::configure0(peripherals.UART0);
    let mut serial0 =
        vgaterm::uart::make_uart0(peripherals.UART0, &mut system.peripheral_clock_control);
    serial0.set_rx_fifo_full_threshold(1);
    serial0.listen_rx_fifo_full();
    vgaterm::enable_timer0_interrupt(Priority::Priority14);
    // vgaterm::timer::enable_timer1_interrupt(Priority::Priority14);
    vgaterm::uart::interrupt_enable0(Priority::Priority6);
    // vgaterm::timer::enable_alarm_interrupts(Priority::Priority14);
    vgaterm::gpio::interrupt_enable(Priority::max());

    unsafe {
        riscv::interrupt::enable();
    }

    // vgaterm::timer::start_repeat_timer0_callback(1_000_000, || unsafe {
    //     if NUM_BYTES > 0 {
    //         println!("{} bytes",  NUM_BYTES );
    //         // println!("{} draw cycles per byte", vgaterm::CHARACTER_DRAW_CYCLES as f32 / NUM_BYTES as f32);
    //         NUM_BYTES = 0;
    //     }
    //     // vgaterm::CHARACTER_DRAW_CYCLES = 0;
    // });

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

    let image1 = include_bytes!("../../image1.bin");
    let image2 = include_bytes!("../../image2.bin");
    let image3 = include_bytes!("../../image3.bin");
    let images = [image3, image2, image1];

    video::load_from_slice(image3);
    // video::color_fade_gradient();
    // let pattern = video::test_pattern();
    // for l in 0..video::HEIGHT {
    //     for h in 0..5 {
    //         for p in 0..pattern.len() {
    //             video::set_pixel(128 * h + p, l, pattern[p])
    //         }
    //     }
    // }
    // let c: Vec<_> = (128..=143).collect();
    // video::color_fade_gradient();
    // Blue, Cyan-blue, Green, Yellow, Orange, Red
    // video::vertical_columns(&[
    //     Rgb3::new(2, 2, 7).to_byte(),
    //     Rgb3::new(2, 7, 2).to_byte(),
    //     Rgb3::new(6, 6, 1).to_byte(),
    //     Rgb3::new(7, 2, 1).to_byte(),
    //     Rgb3::new(5, 5, 5).to_byte(),
    // ]);
    // video::vertical_columns(&[
    //     Rgb3::new(7, 7, 7).to_byte(),
    //     // Rgb3::new(5, 6, 6).to_byte(),
    //     Rgb3::new(5, 5, 5).to_byte(),
    //     Rgb3::new(4, 4, 4).to_byte(),
    //     Rgb3::new(3, 3, 3).to_byte(),
    //     // Rgb3::new(2, 2, 2).to_byte(),
    //     Rgb3::new(1, 1, 1).to_byte(),
    //     Rgb3::new(0, 0, 0).to_byte(),
    //     // Rgb3::new(7, 7, 7).to_byte(),
    // ]);
    // video::load_test_pattern(0b00000011, 0b00000011);
    // video::vertical_columns_rgb(&[(0xff, 0xdb, 0xb6), (0xfe, 0xb7, 0x93), (0xf2, 0x9c, 0x7a), (0xe7, 0x80, 0x61), (0xa2, 0x58, 0x2c)]);
    // let colors: Vec<_> = (0..16).map(|b| Rgb3::new(3, 3, 3).brightness(b).to_byte()).collect();
    // video::vertical_columns(&colors);
    // video::vertical_columns(&[Rgb3::new(0, 0, 0).to_byte(), Rgb3::new(0, 0, 2).to_byte(), Rgb3::new(0, 0, 4).to_byte(), Rgb3::new(0, 0, 6).to_byte(), Rgb3::new(0, 0, 7).to_byte()]);
    // video::vertical_columns(&[0b00000000, 0b00000001, 0b00000010, 0b00000011, 0b00000100]);
    // RGB(6, 2|1|0, 3) => 0b01101011 (107) | 0b01100111 (103) | 0b01100011 (99)

    let mut hist = [0; 256];
    free(|| unsafe {
        for b in video::BUFFER.iter() {
            let x = *b as usize;
            // println!("{x}");
            hist[x] += 1;
        }
    });
    println!("Video Buffer histogram:");
    for (h, count) in hist.iter().enumerate() {
        if *count == 0 {
            continue;
        }
        println!("{h} ==> {count}");
    }

    let mut display = vgaterm::display::Display::new();

    // let s = PrimitiveStyleBuilder::new().stroke_color();
    // r.draw_styled(, target)

    // println!("Done");
    // println!("Clock speed: {} Hz", measure_clock(delay));
    vgaterm::kernel::start(io.pins.gpio3);

    // let mut text_display = vgaterm::display::TextDisplay::new();
    let mut terminal = vgaterm::terminal::TextField::new();
    // terminal.type_str("Hello World!");
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
        &mut system.peripheral_clock_control,
    );

    let mut key_events = VecDeque::new();
    let mut key_state = vgaterm::keyboard::PressedSet::new();
    let mut input = vgaterm::terminal_input::TerminalInput::new(300, 40);

    #[allow(unused)]
    enum ConnectMode {
        LocalEcho,
        ConnectHost,
        None,
    }

    let mode = ConnectMode::LocalEcho;

    let mut frames = 0;
    let mut x: i32 = 0;
    let mut epoch: u32 = 0;
    let mut rng = Rng::new(peripherals.RNG);
    let mut next: u64 = 19;
    enum Displaying {
        Image(usize),
        Waiting(usize),
    }
    let mut displaying = Displaying::Waiting(video::BUFFER_SIZE);
    loop {
        key_events.extend(keyboard.flush_and_parse());
        if let Some(kevent) = key_events.pop_front() {
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
        // Flush the Display to the BUFFER
        // display.flush();
        terminal.draw_up_to(315, &mut display);

        if !key_events.is_empty() {
            continue;
        }

        terminal.draw(&mut display);

        unsafe {
            // this will fire no less often than once per frame
            // if frames == 30 {
            //     frames = 0;
            //     let heap_size = &_heap_size as *const _ as usize;
            //     let used = ALLOCATOR.used() * 100 / heap_size;

            //     for y in 0..100 {
            //         println!("{}%", used);
            //         if y == used {
            //             video::set_pixel(x as usize, video::HEIGHT - y - 1, 128 + (epoch % 128));
            //         } else {
            //             video::set_pixel(x as usize, video::HEIGHT - y - 1, 0x00);
            //         }
            //     }
            //     x = x.wrapping_add(1) % 640;
            //     if x == 0 {
            //         epoch = epoch.wrapping_add(1);
            //     }
            // }
            if frames == 20 {
                println!("done");
            }
            let img = images[(frames / 512000 + 1) % images.len()];
            //201731
            next = (frames as u64 * 257069 + 201731) % video::BUFFER_SIZE as u64;
            let i = next as usize;

            if frames % 512000 < 256000 {
                // We're fading
                video::BUFFER[i] = img[i];
            }

            // if frames < video::BUFFER_SIZE {
            //     video::BUFFER[i] = image2[i];
            // } else if ((2 * video::BUFFER_SIZE)..(3 * video::BUFFER_SIZE)).contains(&frames) {
            //     video::BUFFER[i] = image[i];
            // } else if ((2 * video::BUFFER_SIZE)..(3 * video::BUFFER_SIZE)).contains(&frames) {
            //     video::BUFFER[i] = image[i];
            // } else if frames > 4*video::BUFFER_SIZE {
            //     frames = 0;
            // }
            riscv::asm::wfi();
            frames = frames.wrapping_add(1);
        }
    }
}
