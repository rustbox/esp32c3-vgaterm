use esp32c3_hal::gpio::*;
use esp_hal_common::{Output, PushPull, Unknown};

use crate::gpio;

pub const WIDTH: usize = 640;
pub const HEIGHT: usize = 480;
pub const BUFFER_SIZE: usize = WIDTH * HEIGHT;
pub static mut BUFFER: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE];

pub struct LongPixelGpios {
    gpio0: Gpio0<Output<PushPull>>,
    gpio1: Gpio1<Output<PushPull>>,
    gpio2: Gpio2<Output<PushPull>>,
    gpio3: Gpio3<Output<PushPull>>,
    gpio4: Gpio4<Output<PushPull>>,
    gpio5: Gpio5<Output<PushPull>>,
    gpio6: Gpio6<Output<PushPull>>,
    gpio7: Gpio7<Output<PushPull>>,
    gpio8: Gpio8<Output<PushPull>>
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
            pin8: Gpio8<Unknown>) -> LongPixelGpios {
    
        LongPixelGpios { 
            gpio0: pin0.into_push_pull_output(), 
            gpio1: pin1.into_push_pull_output(),
            gpio2: pin2.into_push_pull_output(),
            gpio3: pin3.into_push_pull_output(),
            gpio4: pin4.into_push_pull_output(),
            gpio5: pin5.into_push_pull_output(),
            gpio6: pin6.into_push_pull_output(),
            gpio7: pin7.into_push_pull_output(),
            gpio8: pin8.into_push_pull_output() 
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
            pin7: Gpio7<Unknown>) -> ShortPixelGpios {

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
        crate::gpio::write_byte(p as u32);
        // unsafe { core::arch::asm!("nop") }
        crate::gpio::write_byte(0);
    }
}

#[no_mangle]
#[inline]
pub fn display_line(row: usize) {
    let mut i = row;

}
