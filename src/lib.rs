#![no_std]
#![feature(array_chunks)]
#![feature(const_trait_impl)]
#![feature(iter_collect_into)]
#![feature(trait_alias)]
#![feature(round_char_boundary)]

extern crate alloc;

pub mod ansi;
pub mod channel;
pub mod color;
pub mod display;
pub mod gpio;
pub mod interrupt;
pub mod kernel;
pub mod keyboard;
pub mod perf;
pub mod spi;
pub mod terminal;
pub mod terminal_input;
pub mod text;
pub mod timer;
pub mod uart;
pub mod usb_keyboard;
pub mod video;

// pub use println::configure;
pub use timer::{
    clear_timer0, configure_timer0, deadline, delay, enable_timer0_interrupt, start_timer0,
    start_timer0_callback, wait_until, Delay,
};

use core::arch::asm;

pub static mut CHARACTER_DRAW_CYCLES: usize = 0;

// back compat
pub use perf::measure_cycle_count;
pub use perf::reset_cycle_count as start_cycle_count;

#[derive(Debug)]
pub enum Work<T> {
    Item(T), // implicitly: awaken immediately

    WouldBlock, // indefinitely
    WouldBlockUntil(u64),
}

pub fn hello() -> &'static str {
    "hello"
}

pub fn measure<O>(count: &mut usize, f: impl FnOnce() -> O) -> O {
    start_cycle_count();
    let r = f();
    *count = measure_cycle_count() as usize;

    r
}

#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "C" fn DefaultHandler(trap_frame: *mut esp32c3_hal::trapframe::TrapFrame) {
    panic!("unhandled exception: {:?}", *trap_frame)
}

mod mem {
    //! see: https://github.com/rust-lang/compiler-builtins/issues/339
    //! in our case, they're primarily unoptimized because they don't live in ram, but on flash,
    //! so they thrash the shit out of the cache

    #[no_mangle]
    #[link_section = ".rwtext"]
    pub unsafe extern "C" fn memcpy(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
        let r = dest;
        let (n, m) = (n / 4, n % 4);
        for i in 0..m {
            *dest.add(i) = *src.add(i);
        }
        let dest = dest.add(m).cast::<usize>();
        let src = src.add(m).cast::<usize>();
        for i in 0..n {
            *dest.add(i) = *src.add(i);
        }
        r
    }

    #[no_mangle]
    #[link_section = ".rwtext"]
    pub unsafe extern "C" fn memmove(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
        enum Idx {
            Forward(usize, usize),
            Backward(usize),
        }

        impl Iterator for Idx {
            type Item = usize;

            fn next(&mut self) -> Option<Self::Item> {
                match self {
                    Idx::Backward(0) => None,
                    Idx::Backward(n) => {
                        *n -= 1;
                        Some(*n)
                    }
                    Idx::Forward(a, b) if *a >= *b => None,
                    Idx::Forward(a, _) => {
                        let r = *a;
                        *a += 1;
                        Some(r)
                    }
                }
            }
        }

        let r = dest;
        let (n, m) = (n / 4, n % 4);

        // "[...] you don't have to worry about whether they overlap at all.
        // If src is less than dst, just copy from the end.
        // If src is greater than dst, just copy from the beginning."
        // — https://stackoverflow.com/a/3572519/151464
        let last;
        for i in if src < dest as *const u8 {
            last = 0;
            Idx::Backward(m)
        } else {
            last = m;
            Idx::Forward(0, m)
        } {
            *dest.add(i) = *src.add(i);
        }

        let dest = dest.add(last).cast::<usize>();
        let src = src.add(last).cast::<usize>();

        for i in if src < dest as *const usize {
            Idx::Backward(n)
        } else {
            Idx::Forward(0, n)
        } {
            *dest.add(i) = *src.add(i);
        }

        r
    }

    // in hot paths:
    //  called once with n=256 : something something interrupt handling (trap frame?)
    //  called once with n=96 : clearing 24*8 bytes of DMA descriptors
    #[no_mangle]
    #[link_section = ".rwtext"]
    pub unsafe extern "C" fn memset(
        p: *mut u8,
        c: i32, /* equivalent to c's int */
        n: usize,
    ) -> *mut u8 {
        let s = p;
        let (n, m) = (n / 4, n % 4);
        let b = c as u8;
        for i in 0..m {
            *p.add(i) = b
        }
        let p = p.add(m).cast::<usize>();

        let w = usize::from_ne_bytes([b; 4]);
        for i in 0..n {
            *p.add(i) = w;
        }
        s
    }

    #[no_mangle]
    #[link_section = ".rwtext"]
    pub unsafe extern "C" fn memcmp(a: *const u8, b: *const u8, n: usize) -> i32 {
        let (n, m) = (n / 4, n % 4);
        for i in 0..m {
            let d = (*a.add(i) as i32).wrapping_sub(*b.add(i) as i32);
            if d != 0 {
                return d;
            }
        }
        let a = a.add(m).cast::<usize>();
        let b = b.add(m).cast::<usize>();
        for i in 0..n {
            let d = (*a.add(i) as isize).wrapping_sub(*b.add(i) as isize);
            if d != 0 {
                return d as i32;
            }
        }

        0
    }
}

#[inline]
pub fn noops<const N: u8>() {
    if 0 < N {
        unsafe { asm!("nop") }
    }
    if 1 < N {
        unsafe { asm!("nop") }
    }
    if 2 < N {
        unsafe { asm!("nop") }
    }
    if 3 < N {
        unsafe { asm!("nop") }
    }
    if 4 < N {
        unsafe { asm!("nop") }
    }
    if 5 < N {
        unsafe { asm!("nop") }
    }
    if 6 < N {
        unsafe { asm!("nop") }
    }
    if 7 < N {
        unsafe { asm!("nop") }
    }
    if 8 < N {
        unsafe { asm!("nop") }
    }
    if 9 < N {
        unsafe { asm!("nop") }
    }
    if 10 < N {
        unsafe { asm!("nop") }
    }
    if 11 < N {
        unsafe { asm!("nop") }
    }
    if 12 < N {
        unsafe { asm!("nop") }
    }
    if 13 < N {
        unsafe { asm!("nop") }
    }
    if 14 < N {
        unsafe { asm!("nop") }
    }
    if 15 < N {
        unsafe { asm!("nop") }
    }
    if 16 < N {
        unsafe { asm!("nop") }
    }
    if 17 < N {
        unsafe { asm!("nop") }
    }
    if 18 < N {
        unsafe { asm!("nop") }
    }
    if 19 < N {
        unsafe { asm!("nop") }
    }
    if 20 < N {
        unsafe { asm!("nop") }
    }
    if 21 < N {
        unsafe { asm!("nop") }
    }
    if 22 < N {
        unsafe { asm!("nop") }
    }
    if 23 < N {
        unsafe { asm!("nop") }
    }
    if 24 < N {
        unsafe { asm!("nop") }
    }
    if 25 < N {
        unsafe { asm!("nop") }
    }
    if 26 < N {
        unsafe { asm!("nop") }
    }
    if 27 < N {
        unsafe { asm!("nop") }
    }
    if 28 < N {
        unsafe { asm!("nop") }
    }
    if 29 < N {
        unsafe { asm!("nop") }
    }
    if 30 < N {
        unsafe { asm!("nop") }
    }
    if 31 < N {
        unsafe { asm!("nop") }
    }

    for _ in 32..N {
        unsafe {
            asm!("nop");
        }
    }
}
