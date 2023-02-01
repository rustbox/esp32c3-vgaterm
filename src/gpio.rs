use alloc::boxed::Box;
use esp32c3_hal::peripherals;
use esp32c3_hal::prelude::*;
use esp32c3_hal::{
    gpio::{Event, InputPin},
    interrupt,
};
use riscv;

pub const GPIO_MMIO_ADDRESS: usize = 0x6000_4000;
pub const GPIO_OUT: usize = GPIO_MMIO_ADDRESS + 0x0004;
pub const GPIO_IN: usize = GPIO_MMIO_ADDRESS + 0x003C;
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
                callback(pin);
                pin.clear_interrupt();
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
    interrupt::enable(peripherals::Interrupt::GPIO, priority).unwrap();
}

pub fn pin_interrupt(
    mut input: impl InterruptPin + 'static,
    event: Event,
    callback: impl FnMut(&mut Box<dyn InterruptPin>) -> () + 'static,
) -> PinRef {
    let n = input.number() as usize;

    riscv::interrupt::free(|| unsafe {
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
    riscv::interrupt::free(|| unsafe {
        let n = pin.0;
        // SAFETY: unwrap the option is fine because we will only
        // grab the pin at array index given in the pin parameter
        // so we know there is Some pin
        PINS[n].take().unwrap().unlisten();
        CALLBACKS[n].take();
    });
    pin
}

pub fn pin_reenable(
    pin: PinRef,
    event: Event,
    callback: impl FnMut(&mut Box<dyn InterruptPin>) -> () + 'static,
) -> PinRef {
    riscv::interrupt::free(|| unsafe {
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
    riscv::interrupt::free(|| unsafe {
        if let Some(ref mut p) = PINS[n] {
            p.unlisten();
        }
    });
    pin
}

pub fn pin_resume(pin: PinRef, event: Event) -> PinRef {
    let n = pin.0;
    riscv::interrupt::free(|| unsafe {
        if let Some(ref mut p) = PINS[n] {
            p.listen(event);
        }
    });
    pin
}

fn check_gpio_source() -> u32 {
    riscv::interrupt::free(|| unsafe {
        let periphs = peripherals::Peripherals::steal();

        let gpio_status = periphs.GPIO.status.read().bits();
        31 - gpio_status.leading_zeros()
    })
}

///
/// Write 32 bit word to the GPIO MMIO register. Each bit in the word
/// corresponds to the GPIO number. So the Nth bit in the word being
/// a 1 or 0 will correspond to GPIO N going high or low respectively.
///
#[inline]
pub fn write_word(d: u32) {
    // Writing to a raw memory address is unsafe. This is a write to
    // a MMIO, going out ot the GPIO pins. This memory address won't
    // be read from. Reading GPIOs is accessed using a different MMIO
    // register.
    unsafe {
        let gpio_out = GPIO_OUT as *mut u32;
        gpio_out.write_volatile(d)
    }
}

///
///
/// Write 32 bit word to the GPIO MMIO register. Each bit in the word
/// corresponds to the GPIO number. So the Nth bit in the word being
/// a 1 or 0 will correspond to GPIO N going high or low respectively.
///
#[inline]
pub fn write_word_w1(d: u32) {
    // Writing to a raw memory address is unsafe. This is a write to
    // a MMIO, going out ot the GPIO pins. This memory address won't
    // be read from. Reading GPIOs is accessed using a different MMIO
    // register.
    unsafe {
        let gpio_out = GPIO_OUT_W1TS as *mut u32;
        *gpio_out = d;
    }
}

///
/// Set a single GPIO pin to high or low. True for high, False for low.
/// This sets the corresponding bit of the GPIO output register to a
/// 1 or 0 to set the value of the GPIO pin high or low.
///
/// So for pin value of 3, we set bit 3 of GPIO_OUT register to the
/// given value. All other bit values will be 0.
///
#[inline]
pub fn gpio_pin_out(pin: u8, value: bool) {
    let v: u32 = value.into();
    let gpio_value = v << pin;
    write_word(gpio_value);
}

pub fn read_byte_mask(mask: u32) -> u8 {
    unsafe {
        let gpio_in = GPIO_IN as *mut u32;
        let data = gpio_in.read_volatile();
        let masked = data & mask;

        let mut n_bit = 0;
        let mut value = 0;
        for b in 0..32 {
            if (1 << b) & mask == 0 {
                continue;
            }
            // We know there is a bit set at bit b in the mask
            // Grab  the bit value of the masked data at bit b
            // Shift the value right however many spaces from the mask to
            // the nth bit in the output value.
            let nth_value_bit = ((1 << b) & masked) >> (b - n_bit);
            value = value | nth_value_bit;

            n_bit += 1;
        }
        value as u8
    }
}

///
/// Read from GPIO 0-7 as a single byte
///
pub fn read_low() -> u8 {
    unsafe {
        let gpio_in = GPIO_IN as *mut u32;
        let data = gpio_in.read_volatile();
        data as u8
    }
}

#[interrupt]
fn GPIO() {
    // sprint!("-");
    riscv::interrupt::free(|| {
        let source = check_gpio_source() as usize;
        callback_pin(source);
    });
}
