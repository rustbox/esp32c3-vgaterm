use alloc::boxed::Box;
use esp32c3_hal::{interrupt, 
    pac, 
    gpio_types::{Event, InputPin}, 
};
use esp32c3_hal::prelude::*;
use riscv;

use crate::{sprint, sprintln};

pub const GPIO_MMIO_ADDRESS: usize = 0x6000_4000;
pub const GPIO_OUT: usize = GPIO_MMIO_ADDRESS + 0x0004;
pub const GPIO_OUT_W1TS: usize = GPIO_MMIO_ADDRESS + 0x0008;

const EMPTY_PIN: Option<Box<dyn InterruptPin>> = None;
const EMPTY_CB: Option<Box<dyn FnMut(&mut Box<dyn InterruptPin>) -> ()>> = None;
static mut PINS: [Option<Box<dyn InterruptPin>>; 32] = [EMPTY_PIN; 32];
static mut CALLBACKS: [Option<Box<Callback>>; 32] = [EMPTY_CB; 32];

type Callback = dyn FnMut(&mut Box<dyn InterruptPin>) -> ();

fn callback_pin(source: usize) {
    unsafe {
        if let Some(ref mut pin) = PINS[source] {
            if let Some(callback) = &mut CALLBACKS[source] {
                pin.clear_interrupt();
                callback(pin);
            }
        }
    }
}

pub trait InterruptPin {
    fn number(&self) -> u8;

    fn clear_interrupt(&mut self);

    fn value(&self) -> bool;

    fn listen(&mut self, event: Event);

    fn unlisten(&mut self);
}

impl<P: InputPin> InterruptPin for P {
    fn number(&self) -> u8 {
        self.number()
    }

    fn clear_interrupt(&mut self) {
        self.clear_interrupt()
    }

    fn value(&self) -> bool {
        self.is_input_high()
    }

    fn listen(&mut self, event: Event) {
        self.listen(event)
    }

    fn unlisten(&mut self) {
        self.unlisten();
    }
}

pub struct PinRef(usize);

pub fn interrupt_enable(priority: interrupt::Priority) {
    interrupt::enable(pac::Interrupt::GPIO, priority).unwrap();
}

pub fn pin_interrupt(
    mut input: impl InterruptPin + 'static,
    event: Event,
    callback: impl FnMut(&mut Box<dyn InterruptPin>) -> () + 'static ) -> PinRef {

    let n = input.number() as usize;
    
    riscv::interrupt::free(|_| unsafe {
        input.listen(event);

        PINS[n] = Some(Box::new(input));
        CALLBACKS[n] = Some(Box::new(callback));

        // if let Some(ref p) = PINS[n] {
        //     sprintln!("{:?}", p.number());
        // }
    });
    PinRef(n)
}

/// Stops the pin from listening for interrupt signals
/// and removes the callback
pub fn interrupt_disable(pin: PinRef) -> PinRef {
    riscv::interrupt::free(|_| unsafe {
        let n = pin.0;
        // SAFETY: unwrap the option is fine because we will only
        // grab the pin at array index given in the pin parameter
        // so we know there is Some pin
        PINS[n].take().unwrap().unlisten();
        CALLBACKS[n].take();
    });
    pin
}

pub fn pin_reenable(pin: PinRef, event: Event, callback: impl FnMut(&mut Box<dyn InterruptPin>) -> () + 'static) -> PinRef {
    riscv::interrupt::free(|_| unsafe {
        let n = pin.0;
        if let Some(ref mut p) = PINS[n] {
            p.listen(event);
        }
        CALLBACKS[n].replace(Box::new(callback));
    });
    pin
}

pub fn pin_pause(pin: PinRef) -> PinRef {
    let n = pin.0;
    riscv::interrupt::free(|_| unsafe {
        if let Some(ref mut p) = PINS[n] {
            p.unlisten();
        }
    });
    pin
}

pub fn pin_resume(pin: PinRef, event: Event) -> PinRef {
    let n = pin.0;
    riscv::interrupt::free(|_| unsafe {
        if let Some(ref mut p) = PINS[n] {
            p.listen(event);
        }
    });
    pin
}

fn check_gpio_source() -> u32 {
    riscv::interrupt::free(|_| unsafe {
        let periphs = pac::Peripherals::steal();
        
        let gpio_status = periphs.GPIO.status.read().bits();
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

#[interrupt]
fn GPIO() {
    // sprint!("-");
    riscv::interrupt::free(|_| {
        let source = check_gpio_source() as usize;
        callback_pin(source);
    });

}
