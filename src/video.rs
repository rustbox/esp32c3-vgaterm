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
#[inline]
pub fn transmit_frame() {
    riscv::interrupt::free(|| unsafe {
        spi::transmit(&mut BUFFER[0..32000]);
        spi::transmit(&mut BUFFER[32000..64000]);
        spi::transmit(&mut BUFFER[64000..96000]);
        spi::transmit(&mut BUFFER[96000..128000]);
        spi::transmit(&mut BUFFER[128000..160000]);
        spi::transmit(&mut BUFFER[160000..192000]);
        spi::transmit(&mut BUFFER[192000..224000]);
        spi::transmit(&mut BUFFER[224000..256000]);
    });
}

///
/// Split the frame buffer into 4 equallay sized columns (160 pixels wide) each
/// of the color given in the arguments. Color a corresponds to the first column,
/// b to the second, etc.
///
pub fn four_vertical_columns(a: u8, b: u8, c: u8, d: u8) {
    riscv::interrupt::free(|| unsafe {
        for line in 0..HEIGHT {
            for p in 0..WIDTH {
                let i = line * WIDTH + p;

                if p < 160 {
                    BUFFER[i] = a;
                }

                if (160..320).contains(&p) {
                    BUFFER[i] = b;
                }

                if (320..480).contains(&p) {
                    BUFFER[i] = c;
                }

                if (480..640).contains(&p) {
                    BUFFER[i] = d;
                }
            }
        }
    });
}

///
/// Sets the frame buffer to a 16x16 set of rectangles of all 256 displayable
/// colors. Offset refers to what color we start with when making each rectangle.
/// x_off is the offset along the x-axis the rectangles should be drawn.
///
pub fn all_colors_rectangles(offset: u8, x_off: usize) {
    let box_w = WIDTH / 16;
    let box_h = HEIGHT / 16;
    for y in 0..16 {
        for x in 0..16 {
            for l in (y * box_h)..(y * box_h + box_h) {
                for p in (x * box_w + x_off)..(x * box_w + box_w + x_off) {
                    let i = l * WIDTH + (p % WIDTH);
                    riscv::interrupt::free(|| unsafe {
                        BUFFER[i] = offset.wrapping_add((y * 16 + x) as u8);
                    });
                }
            }
        }
    }
}

pub fn test_pattern() -> [u8; 128] {
    let mut pattern: [u8; 128] = [0; 128];
    for h in 0..8 {
        for l in 0..8 {
            for p in 0..2 {
                let value: u8 = h << 5 | p << 4 | l << 1 | p;
                let i: usize = (p + 2 * l + 16 * h).into();
                pattern[i] = value;
            }
        }
    }
    pattern
}

pub fn load_from_slice(s: &[u8]) {
    riscv::interrupt::free(|| unsafe {
        for (i, p) in s.iter().enumerate() {
            if i >= BUFFER.len() {
                break;
            }
            BUFFER[i] = *p;
        }
    })
}

pub fn set_pixel(col: usize, row: usize, data: u8) {
    let i = row * WIDTH + col;
    riscv::interrupt::free(|| unsafe {
        BUFFER[i] = data;
    })
}

pub fn load_test_pattern(val1: u8, val2: u8) {
    riscv::interrupt::free(|| unsafe {
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
