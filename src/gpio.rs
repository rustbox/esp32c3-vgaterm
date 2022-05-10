
// hello

use esp_hal_common::InterruptKind;
use esp_hal_common::{interrupt::CpuInterrupt, interrupt::Priority, pac};
use riscv;

pub const GPIO_MMIO_ADDRESS: usize = 0x6000_4000;
pub const GPIO_OUT: usize = GPIO_MMIO_ADDRESS + 0x0004;
pub const GPIO_OUT_W1TS: usize = GPIO_MMIO_ADDRESS + 0x0008;


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

#[inline]
pub fn write_byte(d: u32) {
    unsafe {
        let gpio_out = GPIO_OUT as *mut u32;
        gpio_out.write_volatile(d)
    }
}

#[inline]
pub fn write_byte_w1(d: u32) {
    unsafe {
        let gpio_out = GPIO_OUT_W1TS as *mut u32;
        *gpio_out = d;
    }
}