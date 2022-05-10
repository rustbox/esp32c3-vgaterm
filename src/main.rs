#![no_std]
#![no_main]

#![feature(panic_info_message)]

use esp32c3_hal::gpio::Gpio10;
use esp32c3_hal::{gpio::IO, 
    gpio::Gpio9, 
    pac::Peripherals, 
    prelude::*, 
    Delay, 
    RtcCntl, 
    Timer, 
    spi::Spi,};
use esp_hal_common::interrupt;
use esp_hal_common::spi::Instance;
use esp_hal_common::{Event,
    Input,
    Pin,
    PullDown,
};

// use esp_hal_common::clock_control::ClockControl;
// use panic_halt as _;
use riscv_rt::entry;

use vgaterm;
use vgaterm::{sprintln, sprint};
use vgaterm::spi::QuadInstance;

use bare_metal::Mutex;

use core::cell::RefCell;
use core::arch::asm;
use core::slice::from_raw_parts;

static mut BUTTON: Mutex<RefCell<Option<Gpio9<Input<PullDown>>>>> = Mutex::new(RefCell::new(None));
static mut BUTTON2: Mutex<RefCell<Option<Gpio10<Input<PullDown>>>>> = Mutex::new(RefCell::new(None));

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
    }
    else {
        sprintln!("no information available.");
    }
    stop();
}

#[no_mangle]
extern "C"
fn stop() -> ! {
    loop {
        unsafe {
            riscv::asm::wfi();
        }
    }
}

#[entry]
fn main() -> ! {
    let mut peripherals = Peripherals::take().unwrap();
   

    // Disable the watchdog timers. For the ESP32-C3, this includes the Super WDT,
    // the RTC WDT, and the TIMG WDTs.
    let mut rtc_cntl = RtcCntl::new(peripherals.RTC_CNTL);
    let mut timer1 = Timer::new(peripherals.TIMG1);

    vgaterm::configure_timer0(peripherals.TIMG0);
    
    rtc_cntl.set_super_wdt_enable(false);
    rtc_cntl.set_wdt_enable(false);
    timer1.disable();


    peripherals.SYSTEM.cpu_per_conf.write(|w| unsafe {
        // Set true for 480 MHz stepped down to 160
        w.pll_freq_sel().set_bit();
        w.cpuperiod_sel().bits(0b01)
    });

    peripherals.SYSTEM.sysclk_conf.write(|w| unsafe {
        w.soc_clk_sel().bits(1)
    });
    
    vgaterm::configure(peripherals.UART0);

    // vgaterm::configure_timer0(peripherals.TIMG0);

    // vgaterm::enable_timer0_interrupt(
    //     &interrupt::CpuInterrupt::Interrupt1, 
    //     interrupt::Priority::Priority1
    // );

    // vgaterm::start_timer0(10_000_000);

    unsafe {
        riscv::interrupt::enable();
    }
    
    // Set GPIO5 as an output, and set its state high initially.
    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);
    
    // Set GPIO9 as an input
    // let mut button = io.pins.gpio9.into_pull_down_input();
    // let mut button2 = io.pins.gpio10.into_pull_down_input();
    // button.listen(Event::FallingEdge);
    // button2.listen(Event::FallingEdge);

    // riscv::interrupt::free(|_cs| unsafe {
    //     BUTTON.get_mut().replace(Some(button));
    //     BUTTON2.get_mut().replace(Some(button2));
    // });

    // vgaterm::gpio::interrupt_enable(
    //     &interrupt::CpuInterrupt::Interrupt3, 
    //     interrupt::Priority::Priority1, 
    //     interrupt::InterruptKind::Level
    // );

    // led.set_high().unwrap();

    // Initialize the Delay peripheral, and use it to toggle the LED state in a
    // loop.
    let mut delay = Delay::new(peripherals.SYSTIMER);

    sprintln!("Hello, World!");
    sprintln!("Say hello: \"{}\"", vgaterm::hello());

    configure_count_cycles();
    let cnt = measure_clock(&mut delay);
    sprintln!("We counted {} cycles in 1 second", cnt);


    // let _ = vgaterm::video::ShortPixelGpios::new(
    //     io.pins.gpio0,
    //     io.pins.gpio1,
    //     io.pins.gpio2,
    //     io.pins.gpio3,
    //     io.pins.gpio4,
    //     io.pins.gpio5,
    //     io.pins.gpio6,
    //     io.pins.gpio7
    // );

    let mut sio0 = io.pins.gpio7;
    let mut sio1 = io.pins.gpio2;
    let mut sio2 = io.pins.gpio5;
    let mut sio3 = io.pins.gpio4;
    let mut cs = io.pins.gpio10;
    let mut clk = io.pins.gpio6;


    let mut qspi = vgaterm::spi::QSpi::new(
        peripherals.SPI2, 
        sio0, sio1, sio2, sio3, cs, clk, &mut peripherals.SYSTEM);
    

    // peripherals.SPI2.user.write(|w| {
    //     w.fwrite_quad().set_bit()
    // });
    
    // let mut spi = Spi::new(
    //     peripherals.SPI2,
    //     clk,
    //     d0,
    //     d1,
    //     cs,
    //     80_000_000,
    //     embedded_hal::spi::MODE_0,
    //     &mut peripherals.SYSTEM);

    vgaterm::video::load_test_pattern(0x00, 0xFF);

    let mut frames = 0;
    loop {
        // riscv::interrupt::free(|_| unsafe {
        //     // let _ = spi.transfer(&mut vgaterm::video::BUFFER);
        //     // for i in 0..vgaterm::video::BUFFER_SIZE {
        //     //     let b = vgaterm::video::BUFFER[i] as u32;
        //     //     vgaterm::gpio::write_byte(b);
        //     // }
        // });
        // vgaterm::start_cycle_count();
        // let cycles = vgaterm::video::display_frame();
        // let cycles = vgaterm::measure_cycle_count();
        // sprintln!("Cycles ({}) per pixel ({}) = {}", cycles, vgaterm::video::BUFFER_SIZE, cycles as f32 / (vgaterm::video::BUFFER_SIZE) as f32);
        frames += 1;
        // sprintln!("Written {}", frames);
        // delay.delay_ms(500u32);
        // sprintln!("hi");
        // unsafe {
        //     qspi.transfer(&vgaterm::video::BUFFER);
        // }
        unsafe {
            let buf32: *const &[u32] = vgaterm::video::BUFFER.as_ptr().cast();
            let buffer: &[u32] = buf32.as_ref().unwrap();
            for w in buffer {
                qspi.write_word(*w);
                delay.delay_us(1 as u32);
            }
        }
        sprintln!("Wrote frame {}", frames);
        
        // peripherals.SPI2.configure_datalen(32);
        // peripherals.SPI2.cmd.modify(|_, w| {
        //     w.usr().set_bit()
        // });
        // while peripherals.SPI2.cmd.read().usr().bit() { }
        // delay.delay_ms(1 as u32);
    }
}

struct Mems<'a>(usize, &'a [usize]);

fn show_registers<'a>(addr_start: usize, end_offset: usize) -> Mems<'a> {
    let raw = addr_start as *const usize;
    sprintln!("Made raw pointer");
    let slice = unsafe {
        core::slice::from_raw_parts(raw, end_offset / 4)
    };
    sprintln!("Made slice");
    Mems(addr_start, slice)
}

impl<'a> core::fmt::Display for Mems<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        for (i, reg) in self.1.iter().enumerate() {
            let _ = write!(f, "{:#04x}: {:#032b}", self.0 + 4*i, reg);
        }
        Ok(())
    }
}

#[no_mangle]
fn configure_count_cycles() {
    unsafe {
        // Set count event to clock cycles
        // Enable counting events and set overflow to rollover
        asm!(
            "csrwi 0x7E0, 0x1",
            "csrwi 0x7E1, 0x1"
        );
    }
}

#[no_mangle]
fn measure_clock(delay: &mut Delay) -> u32 {
    unsafe {
        // Set event counter to 0
        asm!(
            "csrwi 0x7E2, 0x00",
        )
    }
    let d: u32;
    delay.delay_ms(1000 as u32);
    unsafe {
        asm!(
            "csrr {}, 0x7E2",
            out(reg) d
        );
    }
    d
}



#[no_mangle]
pub fn interrupt1() {
    riscv::interrupt::free(|_| {
        vgaterm::clear_timer0(interrupt::CpuInterrupt::Interrupt1);
        // sprintln!("Interrupt 1");
        
        vgaterm::start_timer0(10_000_000);
    });
}

#[no_mangle]
pub fn interrupt3() {
    riscv::interrupt::free(|cs| unsafe {
        sprintln!("Some GPIO interrupt!");
        
        let mut button = BUTTON.borrow(*cs).borrow_mut();
        let button = button.as_mut().unwrap();

        let mut button2 = BUTTON2.borrow(*cs).borrow_mut();
        let button2 = button2.as_mut().unwrap();

        sprintln!("Interrupt source: {:?}", vgaterm::interrupt::source());
        sprintln!("GPIO Pin: {}", vgaterm::check_gpio_source());

        vgaterm::interrupt::clear(interrupt::CpuInterrupt::Interrupt3);
        
        button.clear_interrupt();
        button2.clear_interrupt();
    });
}

