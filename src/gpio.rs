
// hello

use esp_hal_common::InterruptKind;
use esp_hal_common::{interrupt::CpuInterrupt, interrupt::Priority, pac};
use riscv;

pub fn interrupt_enable(route_to: &CpuInterrupt, priority: Priority, kind: InterruptKind) {
    crate::interrupt::enable(
        pac::Interrupt::GPIO,
        route_to,
        kind,
        priority
    );
}

pub fn check_gpio_source() -> u32 {
    riscv::interrupt::free(|_| unsafe {
        let gpio_status = pac::Peripherals::steal().GPIO.status.read().bits();
        31 - gpio_status.leading_zeros()
    })
}

