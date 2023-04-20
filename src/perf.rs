use crate::Delay;
use core::{arch::asm, num::NonZeroUsize};

///
/// Configure the esp32c2 custom Control and Status register
/// `mpcer` to count only CPU clock cycles.
///
/// Page 28, https://www.espressif.com/sites/default/files/documentation/esp32-c3_technical_reference_manual_en.pdf
#[no_mangle]
#[inline]
pub fn configure_counter_for_cpu_cycles() {
    unsafe {
        // Set count event to clock cycles
        // Enable counting events and set overflow to rollover
        asm!("csrwi 0x7E0, 0x1", "csrwi 0x7E1, 0x1");
    }
}

#[no_mangle]
#[inline]
pub fn pause_event_counter() {
    unsafe {
        // Disable counting events
        asm!("csrwi 0x7E1, 0x0");
    }
}

#[no_mangle]
pub fn measure_clock(delay: Delay) -> usize {
    unsafe {
        // Set event counter to 0
        asm!("csrwi 0x7E2, 0x00",)
    }
    let d: usize;
    delay.delay_ms(1000);
    unsafe {
        asm!(
            "csrr {}, 0x7E2",
            out(reg) d
        );
    }
    d
}

#[no_mangle]
#[inline]
pub fn reset_cycle_count() {
    unsafe {
        // Set event counter to 0
        asm!("csrwi 0x7E2, 0x00",)
    }
}

#[no_mangle]
#[inline]
pub fn measure_cycle_count() -> u32 {
    register::mpccr::read() as u32
}

pub mod register {
    pub mod mpccr {
        use core::arch::asm;

        #[inline(always)]
        pub fn read() -> usize {
            let d: usize;
            unsafe {
                asm!(
                    "csrr {}, 0x7E2",
                    out(reg) d
                );
            }
            d
        }
    }
}

pub struct Measure {
    started: Option<NonZeroUsize>,

    accum: u64,
    ticks: u32,
    baseline: f64,
    last: f64,

    // consts
    name: &'static str,
    freq: fugit::HertzU32,
}

impl Measure {
    pub const fn new(name: &'static str, freq: fugit::HertzU32) -> Self {
        Measure {
            name,
            freq,

            started: None,
            accum: 0,
            ticks: 0,
            baseline: f64::MAX,
            last: f64::MAX,
        }
    }

    #[inline]
    pub fn start<const N: usize>(measures: [&mut Measure; N]) {
        let now = register::mpccr::read();

        for m in measures {
            // a very Rust way to spell `if now == 0 { 1 } else { now }`
            m.started
                .replace(NonZeroUsize::try_from(now).unwrap_or(NonZeroUsize::MIN));
        }
    }

    #[inline]
    pub fn stop<const N: usize>(measures: [&mut Measure; N]) {
        let now = register::mpccr::read();

        for m in measures {
            if let Some(started) = m.started.take() {
                m.ticks += 1;
                m.accum += (now - usize::from(started)) as u64;
            }
        }
    }

    #[link_section = ".rwtext"]
    #[inline(never)]
    pub fn flush<const N: usize>(measures: [&mut Measure; N]) {
        for m in measures {
            if m.ticks < m.freq.to_Hz() {
                continue;
            }

            let &mut Measure {
                name,
                accum,
                ticks,
                last,
                ..
            } = m;
            m.ticks = 0;
            m.accum = 0;

            let avg = accum as f64 / ticks as f64;
            // let baseline = f64::min(m.baseline, avg);
            let baseline = if m.baseline > avg { avg } else { m.baseline };
            m.baseline = baseline;
            m.last = avg;

            let base_diff = avg - baseline;
            let last_diff = avg - last;

            // silence unused warnings when the println is commented out
            let _ = base_diff;
            let _ = last_diff;
            let _ = name;

            // esp_println::println!(
            //     "perf: {name:>12}: {accum:>15} / {ticks} ≈ {avg:15.4} (vs. baseline: {base_diff:+12.3} last: {last_diff:+12.3})"
            // );
        }
    }
}
