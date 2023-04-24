use esp32c3_hal::prelude::*;

use crate::{spi, timer};

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
#[inline(always)]
pub fn transmit_frame() {
    // static mut M1: crate::perf::Measure =
    //     crate::perf::Measure::new("first_block", fugit::HertzU32::Hz(240));
    // static mut M2: crate::perf::Measure =
    //     crate::perf::Measure::new("full_frame", fugit::HertzU32::Hz(240));

    // crate::perf::reset_cycle_count(); // no cycle count for you!
    riscv::interrupt::free(|| unsafe {
        // let (first_block, full_frame) = { (&mut M1, &mut M2) };
        // crate::perf::Measure::start([first_block, full_frame]);
        // crate::perf::pause_event_counter();

        spi::transmit(&mut BUFFER[0..32000]);
        // crate::perf::Measure::stop([first_block]);
        spi::transmit(&mut BUFFER[32000..64000]);
        spi::transmit(&mut BUFFER[64000..96000]);
        spi::transmit(&mut BUFFER[96000..128000]);
        spi::transmit(&mut BUFFER[128000..160000]);
        spi::transmit(&mut BUFFER[160000..192000]);
        spi::transmit(&mut BUFFER[192000..224000]);
        spi::transmit(&mut BUFFER[224000..256000]);
        // crate::perf::Measure::stop([full_frame]);

        // crate::perf::Measure::flush([first_block, full_frame]);
        // crate::perf::configure_counter_for_cpu_cycles();
    });
}

pub static mut OFFSET: usize = 0;
const CHUNK_SIZE: usize = 32000;
const LAST_CHUNK: usize = 224000;

// #[inline(always)]
#[link_section = ".rwtext"]
pub fn transmit_chunk() {
    static mut M1: crate::perf::Measure =
        crate::perf::Measure::new("start_xmit", fugit::HertzU32::Hz(240 * 8));
    static mut M2: crate::perf::Measure =
        crate::perf::Measure::new("tx_wait", fugit::HertzU32::Hz(240 * 8));

    let (xmit_chunk, tx_wait) = unsafe { (&mut M1, &mut M2) };

    if let Some(tx) = unsafe { &mut spi::SPI_DMA_TRANSFER }.take() {
        crate::perf::Measure::start([tx_wait]);
        let (_, spi) = tx.wait();
        crate::perf::Measure::stop([tx_wait]);
        unsafe { &mut spi::QSPI }.replace(spi);
    }

    let data = unsafe { &mut BUFFER[OFFSET..OFFSET + CHUNK_SIZE] };
    crate::perf::Measure::start([xmit_chunk]);
    spi::start_transmit(data);
    crate::perf::Measure::stop([xmit_chunk]);

    timer::start_timer0_callback(1565, timer_callback);
    crate::perf::Measure::flush([xmit_chunk, tx_wait]);
}

#[link_section = ".rwtext"]
fn timer_callback() {
    let offset = unsafe { &mut OFFSET };
    if *offset < LAST_CHUNK {
        *offset += CHUNK_SIZE;
        transmit_chunk();
    }
}

// #[interrupt]
// fn SYSTIMER_TARGET0() {
//     riscv::interrupt::free(|| unsafe {
//         timer::clear_alarm0();
//         if let Some(tx) = spi::SPI_DMA_TRANSFER.take() {
//             print!(".");
//             let (_, spi) = tx.wait();
//             spi::QSPI.replace(spi);
//         }
//         transmit_chunk();
//     });
// }

///
/// Split the frame buffer into 4 equally sized columns (160 pixels wide) each
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
        for (i, e) in BUFFER.iter_mut().enumerate() {
            if i & 1 == 0 {
                *e = val1;
            } else {
                *e = val2;
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
