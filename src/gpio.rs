use core::{any::Any, marker::PhantomData};

use alloc::boxed::Box;
use esp32c3_hal::peripherals;
use esp32c3_hal::prelude::*;
use esp32c3_hal::{
    gpio::{Event, Pin},
    interrupt,
};

use riscv;

pub const GPIO_MMIO_ADDRESS: usize = 0x6000_4000;
pub const GPIO_OUT: usize = GPIO_MMIO_ADDRESS + 0x0004;
pub const GPIO_IN: usize = GPIO_MMIO_ADDRESS + 0x003C;
pub const GPIO_OUT_W1TS: usize = GPIO_MMIO_ADDRESS + 0x0008;

const INIT_INTERRUPT: Option<Box<dyn Irq>> = None;
static mut INTERRUPTS: [Option<Box<dyn Irq>>; 32] = [INIT_INTERRUPT; 32];

pub struct PinInterrupt<T>
where
    T: Pin + 'static,
{
    pub pin: T,
    pub event: Event,
    pub callback: Box<Callback<T>>,
}

// NB: all Irqs must be PinInterrupts
trait Irq: Any {
    fn apply(&mut self);

    fn as_any(self: Box<Self>) -> Box<dyn Any>;
    fn as_any_mut(self: &mut Self) -> &mut dyn Any;
}

// requires unstable #![feature(trait_upcasting)]
// impl dyn Irq {
//     fn as_any(self: Box<Self>) -> Box<dyn Any> {
//         self
//     }
//     fn as_any_mut(self: &mut Self) -> &mut dyn Any {
//         self as &mut dyn Any
//     }
// }

impl<T> Irq for PinInterrupt<T>
where
    T: Pin + 'static,
{
    fn apply(&mut self) {
        (self.callback)(&mut self.pin);
        self.pin.clear_interrupt();
    }

    fn as_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
    fn as_any_mut(self: &mut Self) -> &mut dyn Any {
        self as &mut dyn Any
    }
}

pub const LOW: bool = false;
pub const HIGH: bool = true;

mod example {
    use esp32c3_hal::prelude::*;
    use esp32c3_hal::{
        gpio::{
            BankGpioRegisterAccess, Event, GpioPin, GpioSignal, Input,
            InteruptStatusRegisterAccess, IsOutputPin, OpenDrain, Output, PinType, PullUp,
        },
        peripherals::Peripherals,
        IO,
    };

    #[allow(dead_code)]
    fn example() {
        let peripherals = Peripherals::take();
        let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);

        let pin = super::pin_interrupt(
            io.pins.gpio6.into_pull_up_input(),
            Event::FallingEdge,
            |_| {},
        );

        let (clk, event, callback) = super::interrupt_disable(pin);

        let mut clk_out = clk.into_open_drain_output();
        let _ = clk_out.set_high();

        super::pin_interrupt(W(clk_out).into(), event, callback);
    }

    // Wrapper type to allow us to implement From<GpioPin<..>>
    struct W<T>(T);

    impl<M, R, I, P, S, const N: u8> From<W<GpioPin<M, R, I, P, S, N>>>
        for GpioPin<Output<OpenDrain>, R, I, P, S, N>
    where
        R: BankGpioRegisterAccess,
        I: InteruptStatusRegisterAccess,
        P: PinType + IsOutputPin,
        S: GpioSignal,
    {
        fn from(value: W<GpioPin<M, R, I, P, S, N>>) -> Self {
            value.0.into_open_drain_output()
        }
    }

    impl<M, R, I, P, S, const N: u8> From<W<GpioPin<M, R, I, P, S, N>>>
        for GpioPin<Input<PullUp>, R, I, P, S, N>
    where
        R: BankGpioRegisterAccess,
        I: InteruptStatusRegisterAccess,
        P: PinType + IsOutputPin,
        S: GpioSignal,
    {
        fn from(value: W<GpioPin<M, R, I, P, S, N>>) -> Self {
            value.0.into_pull_up_input()
        }
    }

    #[allow(dead_code)]
    fn steal() {
        use esp32c3_hal::gpio::GpioExt;
        let peripherals = unsafe { Peripherals::steal() };

        let _z = peripherals.GPIO.split();
    }
}

pub struct PinRef<T>(usize, PhantomData<T>);

pub fn interrupt_enable(priority: interrupt::Priority) {
    interrupt::enable(peripherals::Interrupt::GPIO, priority).unwrap();
}

pub fn pin_interrupt<T: Pin + 'static>(
    mut input: T,
    event: Event,
    callback: impl FnMut(&mut T) -> () + 'static,
) -> PinRef<T> {
    let n = input.number() as usize;

    riscv::interrupt::free(|| {
        input.listen(event);

        unsafe { &mut INTERRUPTS[n] }.replace(Box::new(PinInterrupt {
            pin: input,
            event,
            callback: Box::new(callback),
        }))
    });
    PinRef(n, PhantomData)
}

type Callback<T> = dyn FnMut(&mut T) -> ();

/// Stops the pin from listening for interrupt signals
/// removes and returns the callback
pub fn interrupt_disable<T>(pin: PinRef<T>) -> (T, Event, Box<Callback<T>>)
where
    T: Pin + 'static,
{
    riscv::interrupt::free(|| {
        let n = pin.0;
        // SAFETY: we're in an interrupt::free block with only one hart,
        // so the static dereference is fine
        // also: unwrap the option is fine because we will only
        // grab the pin at array index given in the pin parameter
        // so we know there is Some pin
        let mut isr = unsafe { &mut INTERRUPTS[n] }
            .take()
            .unwrap()
            .as_any()
            .downcast::<PinInterrupt<T>>()
            .unwrap();

        isr.pin.unlisten();

        (isr.pin, isr.event, isr.callback)
    })
}

pub fn pin_pause<T: Pin + 'static>(pin: &mut PinRef<T>) {
    let n = pin.0;
    riscv::interrupt::free(|| {
        if let Some(irq) = unsafe { &mut INTERRUPTS[n] } {
            irq.as_any_mut()
                .downcast_mut::<PinInterrupt<T>>()
                .unwrap()
                .pin
                .unlisten();
        }
    });
}

pub fn pin_resume<T: Pin + 'static>(pin: &mut PinRef<T>) {
    riscv::interrupt::free(|| {
        let n = pin.0;
        if let Some(irq) = unsafe { &mut INTERRUPTS[n] } {
            let isr = irq.as_any_mut().downcast_mut::<PinInterrupt<T>>().unwrap();

            isr.pin.listen(isr.event);
        }
    });
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

pub fn read_pin(pin: u8) -> bool {
    let gpio_in = GPIO_IN as *mut u32;
    unsafe {
        let data = gpio_in.read_volatile();
        data & (1 << pin as u32) != 0
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
        if let Some(ref mut irq) = unsafe { &mut INTERRUPTS[source] } {
            irq.apply();
        };
    });
}
