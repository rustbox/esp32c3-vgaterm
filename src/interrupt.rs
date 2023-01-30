pub use esp32c3_hal::interrupt::TrapFrame;

pub mod peripherals /* crate::interrupt::peripherals */ {
    // crate::peripherals::Interrupt::GPIO;
    pub use esp32c3_hal::peripherals::Interrupt;
}


pub use esp32c3_hal::interrupt::{CpuInterrupt, InterruptKind, Priority};
// pub use esp32c3_hal::{
//     interrupt,
//     peripherals::{
//         Interrupt::{self, GPIO as peripherals::Interrupt::GPIO},
//         Peripherals,
//     },
//     Cpu,
// };

/// Grab the Interrupt enum value from a reference.
///
/// This is needed because Interrupt is not Copy nor Clone
pub fn which_interrupt(interrupt: &CpuInterrupt) -> CpuInterrupt {
    use CpuInterrupt::*;
    match interrupt {
        Interrupt1 => Interrupt1,
        Interrupt2 => Interrupt2,
        Interrupt3 => Interrupt3,
        Interrupt4 => Interrupt4,
        Interrupt5 => Interrupt5,
        Interrupt6 => Interrupt6,
        Interrupt7 => Interrupt7,
        Interrupt8 => Interrupt8,
        Interrupt9 => Interrupt9,
        Interrupt10 => Interrupt10,
        Interrupt11 => Interrupt11,
        Interrupt12 => Interrupt12,
        Interrupt13 => Interrupt13,
        Interrupt14 => Interrupt14,
        Interrupt15 => Interrupt15,
        Interrupt16 => Interrupt16,
        Interrupt17 => Interrupt17,
        Interrupt18 => Interrupt18,
        Interrupt19 => Interrupt19,
        Interrupt20 => Interrupt20,
        Interrupt21 => Interrupt21,
        Interrupt22 => Interrupt22,
        Interrupt23 => Interrupt23,
        Interrupt24 => Interrupt24,
        Interrupt25 => Interrupt25,
        Interrupt26 => Interrupt26,
        Interrupt27 => Interrupt27,
        Interrupt28 => Interrupt28,
        Interrupt29 => Interrupt29,
        Interrupt30 => Interrupt30,
        Interrupt31 => Interrupt31,
    }
}

pub fn which_priority(priority: &Priority) -> Priority {
    use Priority::*;
    match priority {
        Priority1 => Priority1,
        Priority2 => Priority2,
        Priority3 => Priority3,
        Priority4 => Priority4,
        Priority5 => Priority5,
        Priority6 => Priority6,
        Priority7 => Priority7,
        Priority8 => Priority8,
        Priority9 => Priority9,
        Priority10 => Priority10,
        Priority11 => Priority11,
        Priority12 => Priority12,
        Priority13 => Priority13,
        Priority14 => Priority14,
        Priority15 => Priority15,
        None => None
    }
}

// pub fn enable(source: Interrupt, handler: &CpuInterrupt, kind: InterruptKind, priority: Priority) {
//     interrupt::enable(source, priority).unwrap();

//     interrupt::set_kind(Cpu::ProCpu, which_interrupt(handler), kind);
// }

// pub fn clear(handler: CpuInterrupt) {
//     interrupt::clear(Cpu::ProCpu, handler)
// }

// pub fn disable(source: Interrupt) {
//     interrupt::disable(Cpu::ProCpu, source);
// }

// pub fn source() -> Option<Interrupt> {
//     riscv::interrupt::free(|| unsafe {
//         let periphs = Peripherals::steal();
//         let status0 = &periphs.INTERRUPT_CORE0.intr_status_reg_0.read().bits();
//         let int_num = if *status0 & 0x7FFF == 0 {
//             // this checks if the status0 register has anything set. If nothing set
//             // Then let's check the status1 register
//             // We zero out bits 0-14 since those are reserved (aka first int starts at 15)
//             let status1 = &periphs.INTERRUPT_CORE0.intr_status_reg_1.read().bits();
//             31 - status1.leading_zeros() + 32
//         } else {
//             31 - status0.leading_zeros()
//         };
//         Interrupt::try_from(int_num as u8).ok()
//     })
// }

