use esp32c3_hal::prelude::*;
use esp_println::println;

use crate::{
    color::{byte_to_rgb3, color3_to_byte, rgb_from_byte, Rgb3},
    spi::{
        self,
        Instance::{ReadyToSend, TxInProgress},
    },
    timer,
};

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

#[link_section = ".rwtext"]
pub fn transmit_chunk() {
    // static mut M1: crate::perf::Measure =
    //     crate::perf::Measure::new("xmit_chunk", fugit::HertzU32::Hz(240 * 8));
    // static mut M2: crate::perf::Measure =
    //     crate::perf::Measure::new("start_xmit", fugit::HertzU32::Hz(240 * 8));
    // static mut M3: crate::perf::Measure =
    //     crate::perf::Measure::new("tx_wait", fugit::HertzU32::Hz(240 * 8));

    // let (xmit_chunk, start_xmit, tx_wait) = unsafe { (&mut M1, &mut M2, &mut M3) };
    // crate::perf::Measure::start([xmit_chunk]);

    unsafe { &mut spi::QSPI }.replace_with(|i| match i {
        TxInProgress(tx) => {
            // crate::perf::Measure::start([tx_wait]);
            let (_, spi) = tx.wait();
            // crate::perf::Measure::stop([tx_wait]);
            ReadyToSend(spi)
        }
        i => i,
    });

    let data = unsafe { &mut BUFFER[OFFSET..OFFSET + CHUNK_SIZE] };
    // crate::perf::Measure::start([start_xmit]);
    spi::start_transmit(data);
    // crate::perf::Measure::stop([start_xmit]);

    timer::start_timer0_callback(1200, timer_callback);
    // crate::perf::Measure::stop([xmit_chunk]);
    // crate::perf::Measure::flush([start_xmit, tx_wait, xmit_chunk]);
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

pub fn vertical_columns(colors: &[u8]) {
    for c in colors {
        let (r, g, b) = rgb_from_byte(*c);
        println!("Color byte {c} => ({}, {}, {})", r / 36, g / 36, b / 36);
    }
    let width = WIDTH / colors.len();
    riscv::interrupt::free(|| unsafe {
        for line in 0..HEIGHT {
            for p in 0..WIDTH {
                let i = line * WIDTH + p;

                let c = (p / width).clamp(0, colors.len() - 1);
                BUFFER[i] = colors[c];
            }
        }
    });
}

#[allow(clippy::needless_range_loop)]
pub fn clear(color: Rgb3) {
    for p in 0..BUFFER_SIZE {
        riscv::interrupt::free(|| unsafe {
            BUFFER[p] = color.to_byte();
        });
    }
}

pub fn vertical_columns_rgb(colors: &[(u8, u8, u8)]) {
    for (r, g, b) in colors {
        let rgb = Rgb3::from_rgb(*r, *g, *b);
        let b = rgb.to_byte();
        let (er, eg, eb) = {
            let (rr, gg, bb) = byte_to_rgb3(b);
            (color3_to_byte(rr), color3_to_byte(gg), color3_to_byte(bb))
        };
        println!(
            "({}, {}, {}): {} goes to byte {}, to RGB => ({}, {}, {})",
            r, g, b, rgb, b, er, eg, eb
        );
    }
    let width = WIDTH / colors.len();
    riscv::interrupt::free(|| unsafe {
        for line in 0..HEIGHT {
            for p in 0..WIDTH {
                let i = line * WIDTH + p;

                let c = (p / width).clamp(0, colors.len() - 1);
                let (r, g, b) = colors[c];
                BUFFER[i] = Rgb3::from_rgb(r, g, b).to_byte();
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

pub fn color_fade_gradient() {
    let mut colors = [Rgb3::new(0, 0, 0); 60];
    #[allow(clippy::needless_range_loop)]
    for i in 0..8 {
        colors[i] = Rgb3::new(7, i as u8, 0);
    }
    for i in 0..8 {
        colors[i + 8] = Rgb3::new(7 - i as u8, 7, 0);
    }
    for i in 0..8 {
        colors[i + 16] = Rgb3::new(0, 7, i as u8);
    }
    for i in 0..8 {
        colors[i + 24] = Rgb3::new(0, 7 - i as u8, 7);
    }
    for i in 0..8 {
        colors[i + 32] = Rgb3::new(i as u8, 0, 7);
    }
    for i in 0..8 {
        colors[i + 40] = Rgb3::new(7, 0, 7 - i as u8);
    }
    for i in 0..8 {
        colors[i + 48] = Rgb3::new(7 - i as u8 / 2, i as u8 / 2, i as u8 / 2);
    }
    for i in 0..4 {
        colors[i + 56] = Rgb3::new(3, 3, 3);
    }

    for x in 0..WIDTH {
        let color_band = x / (WIDTH / 58);
        let c: Rgb3 = colors[color_band];
        // println!("hello");
        for l in (0..HEIGHT).rev() {
            let line = HEIGHT - l; // Reverse so that the bottom line is considered 0
            let chunk = (line / (HEIGHT / 16)) as u8;

            let i = l * WIDTH + x;

            riscv::interrupt::free(|| unsafe {
                BUFFER[i] = c.brightness(chunk).to_byte();
            })
        }
    }
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
