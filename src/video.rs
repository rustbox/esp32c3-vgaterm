use esp32c3_hal::gpio::*;
use esp_hal_common::{Output, PushPull, Unknown};

use crate::spi;

pub const WIDTH: usize = 640;
pub const HEIGHT: usize = 400;
pub const BUFFER_SIZE: usize = WIDTH * HEIGHT;
pub static mut BUFFER: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE];

///
/// Transmit the contents of the buffer via SPI to the monitor
/// control hardware.
///
/// The SPI can only transmit up to 32,768 bytes at a time, so
/// here we break apart the frame buffer into eight parts, each to
/// be sent one by one in order.
///
pub fn transmit_frame() {
    riscv::interrupt::free(|_| unsafe {
        let a = &BUFFER[0..32000];
        let b = &BUFFER[32000..64000];
        let c = &BUFFER[64000..96000];
        let d = &BUFFER[96000..128000];
        let e = &BUFFER[128000..160000];
        let f = &BUFFER[160000..192000];
        let g = &BUFFER[192000..224000];
        let h = &BUFFER[224000..256000];

        spi::transmit(a);
        spi::transmit(b);
        spi::transmit(c);
        spi::transmit(d);
        spi::transmit(e);
        spi::transmit(f);
        spi::transmit(g);
        spi::transmit(h);
    });
}

pub struct LongPixelGpios {
    gpio0: Gpio0<Output<PushPull>>,
    gpio1: Gpio1<Output<PushPull>>,
    gpio2: Gpio2<Output<PushPull>>,
    gpio3: Gpio3<Output<PushPull>>,
    gpio4: Gpio4<Output<PushPull>>,
    gpio5: Gpio5<Output<PushPull>>,
    gpio6: Gpio6<Output<PushPull>>,
    gpio7: Gpio7<Output<PushPull>>,
    gpio8: Gpio8<Output<PushPull>>,
}

impl LongPixelGpios {
    pub fn new(
        pin0: Gpio0<Unknown>,
        pin1: Gpio1<Unknown>,
        pin2: Gpio2<Unknown>,
        pin3: Gpio3<Unknown>,
        pin4: Gpio4<Unknown>,
        pin5: Gpio5<Unknown>,
        pin6: Gpio6<Unknown>,
        pin7: Gpio7<Unknown>,
        pin8: Gpio8<Unknown>,
    ) -> LongPixelGpios {
        LongPixelGpios {
            gpio0: pin0.into_push_pull_output(),
            gpio1: pin1.into_push_pull_output(),
            gpio2: pin2.into_push_pull_output(),
            gpio3: pin3.into_push_pull_output(),
            gpio4: pin4.into_push_pull_output(),
            gpio5: pin5.into_push_pull_output(),
            gpio6: pin6.into_push_pull_output(),
            gpio7: pin7.into_push_pull_output(),
            gpio8: pin8.into_push_pull_output(),
        }
    }
}

pub struct ShortPixelGpios {
    gpio0: Gpio0<Output<PushPull>>,
    gpio1: Gpio1<Output<PushPull>>,
    gpio2: Gpio2<Output<PushPull>>,
    gpio3: Gpio3<Output<PushPull>>,
    gpio4: Gpio4<Output<PushPull>>,
    gpio5: Gpio5<Output<PushPull>>,
    gpio6: Gpio6<Output<PushPull>>,
    gpio7: Gpio7<Output<PushPull>>,
}

impl ShortPixelGpios {
    pub fn new(
        pin0: Gpio0<Unknown>,
        pin1: Gpio1<Unknown>,
        pin2: Gpio2<Unknown>,
        pin3: Gpio3<Unknown>,
        pin4: Gpio4<Unknown>,
        pin5: Gpio5<Unknown>,
        pin6: Gpio6<Unknown>,
        pin7: Gpio7<Unknown>,
    ) -> ShortPixelGpios {
        ShortPixelGpios {
            gpio0: pin0.into_push_pull_output(),
            gpio1: pin1.into_push_pull_output(),
            gpio2: pin2.into_push_pull_output(),
            gpio3: pin3.into_push_pull_output(),
            gpio4: pin4.into_push_pull_output(),
            gpio5: pin5.into_push_pull_output(),
            gpio6: pin6.into_push_pull_output(),
            gpio7: pin7.into_push_pull_output(),
        }
    }
}

pub fn set_pixel(col: usize, row: usize, data: u8) {
    let i = row * WIDTH + col;
    riscv::interrupt::free(|_| unsafe {
        BUFFER[i] = data;
    })
}

pub fn load_test_pattern(val1: u8, val2: u8) {
    riscv::interrupt::free(|_| unsafe {
        for i in 0..BUFFER_SIZE {
            if i & 1 == 0 {
                BUFFER[i] = val1;
            } else {
                BUFFER[i] = val2;
            }
        }
    })
}

pub fn rgb_from_byte(color: u16) -> (u8, u8, u8) {
    let lower8 = color & 0xFF;
    let shifted = lower8 << 1;
    let highest = shifted >> 7;
    let rgb = shifted + highest;

    let red3 = (rgb & 0b000_000_111) as u8;
    let green3 = ((rgb >> 3) & 0b000_000_111) as u8;
    let blue3 = ((rgb >> 6) & 0b000_000_111) as u8;

    let red = if red3 >> 2 == 1 {
        32 * red3 + 0b11111
    } else {
        32 * red3
    };

    let green = if green3 >> 2 == 1 {
        32 * green3 + 0b11111
    } else {
        32 * green3
    };

    let blue = if blue3 >> 2 == 1 {
        32 * blue3 + 0b11111
    } else {
        32 * blue3
    };

    (red, green, blue)
}

#[no_mangle]
pub fn display_frame() -> u32 {
    crate::start_cycle_count();
    let mut i = 0;
    while i < crate::video::BUFFER_SIZE {
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
        write_pixel(i);
        i += 1;
    }
    crate::measure_cycle_count()
}

#[no_mangle]
#[inline]
pub fn write_pixel(i: usize) {
    unsafe {
        let p = *BUFFER.get_unchecked(i);
        crate::gpio::write_word(p as u32);
        // unsafe { core::arch::asm!("nop") }
        crate::gpio::write_word(0);
    }
}

#[no_mangle]
#[inline]
pub fn display_line(row: usize) {
    let _i = row;
}
