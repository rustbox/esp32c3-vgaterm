#![no_std]
#![no_main]
#![feature(panic_info_message)]

extern crate alloc;

use alloc::collections::VecDeque;
use esp32c3_hal::prelude::*;
use esp32c3_hal::timer::TimerGroup;
use esp32c3_hal::{clock::ClockControl, peripherals::Peripherals};
use esp32c3_hal::{clock::CpuClock, systimer::SystemTimer};
use esp32c3_hal::{
    interrupt::{self, Priority},
    systimer::Alarm,
};
use esp32c3_hal::{Rtc, IO};
use esp_println::{print, println};
use vgaterm::usb_keyboard::US_ENGLISH;

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

    unsafe {
        riscv::interrupt::enable();
    }

    let _delay = vgaterm::Delay::new(&clocks);

    println!("Hello World");

    // // vgaterm::gpio::interrupt_enable(Priority::Priority1);
    let mut keyboard = vgaterm::keyboard::Keyboard::from_peripherals(
        US_ENGLISH,
        io.pins.gpio1,
        io.pins.gpio0,
        peripherals.UART1,
        &clocks,
    );

    let alarm0 = SystemTimer::new(peripherals.SYSTIMER).alarm0;

    alarm0.interrupt_enable(true);
    interrupt::enable(
        esp32c3_hal::peripherals::Interrupt::SYSTIMER_TARGET0,
        Priority::Priority4,
    )
    .unwrap();

    let mut kevents = VecDeque::new();
    let mut key_state = vgaterm::keyboard::PressedSet::new();

    let mut input = vgaterm::terminal_input::TerminalInput::new(300, 40);

    // Setup a timer interrupt every 16 ms
    // vgaterm::timer::enable_timer0_interrupt(Priority::Priority5);
    // vgaterm::timer::start_repeat_timer0_callback(16 * 1000, || {});

    // let (_, mut key_input_rx) = vgaterm::channel::channel::<u8>();
    // let (mut host_tx, mut host_rcv) = vgaterm::channel::channel::<u8>();

    // let mut terminal = vgaterm::terminal::TextField::new();
    // let mut display = Display::new();
    // print!(".");
    loop {
        // // Get characters from keyboard input
        // let key_in = key_input_rx.recv_all();
        // // Get characters from host
        // let host_in = host_rcv.recv_all();

        // // Send all characters from keyboard to the host
        // host_tx.send_all(key_in);

        // // Update terminal with the host output
        // terminal.send(host_in);
        // terminal.draw(&mut display);
        // display.flush();
        kevents.extend(keyboard.flush_and_parse());

        if let Some(kevent) = kevents.pop_front() {
            key_state.push(kevent);
        }

        use vgaterm::Work::*;
        let last_char = input.key_char(&key_state);

        match last_char {
            Item(ref c) => print!("{}", c),
            WouldBlock => {
                println!("\nwaiting for keyboard....");
                alarm0.set_target(u64::MAX) /* wait for keyboard input */
            }
            WouldBlockUntil(inst) => alarm0.set_target(inst),
        }

        if !kevents.is_empty() || !matches!(last_char, WouldBlock | WouldBlockUntil(_)) {
            // don't sleep while there's work to do
            continue;
        }

        unsafe {
            riscv::asm::wfi();
        }
    }
}

#[interrupt]
fn SYSTIMER_TARGET0() {
    use esp32c3_hal::systimer::Target;
    let hax: Alarm<Target, 0> = unsafe { core::mem::transmute(()) };

    hax.clear_interrupt();
}
