use core::convert::Infallible;

use esp32c3_hal::prelude::*;
use esp_hal_common::{Floating, Gpio6, Gpio8, Input, PullUp, Unknown, Event};
use esp_println::println;
use pc_keyboard::{layouts, HandleControl, KeyEvent, Keyboard, ScancodeSet2};

use crate::{
    gpio::{self, InterruptPin, LOW},
    timer,
};

static mut KEYBOARD: Option<Keyboard<layouts::Us104Key, ScancodeSet2>> = None;
static mut DATA: Option<Gpio8<Input<Floating>>> = None;
static mut CLOCK: Option<Gpio6<Input<Floating>>> = None;

pub fn configure(data: Gpio8<Unknown>, clock: Gpio6<Unknown>) {
    let k = Keyboard::new(HandleControl::Ignore);
    let clk = clock.into_floating_input();
    let data = data.into_floating_input();
    
    let data_pin = InterruptPin::number(&data);
    
    let r = crate::gpio::pin_interrupt(
        clk,
        Event::FallingEdge,
        move | clk | {
            // record current data pin state
            // let _dstart = vgaterm::gpio::read_pin(data_pin);
            // wait until clk goes high again
            let mut key_value = u16::from(crate::gpio::read_pin(data_pin));
            println!("clk: {}, data: {}", u16::from(clk.value()), key_value);
            // Wait to go high
            while !clk.value() { }
            
            for i in 1..11 {
                // Wait to go low
                while clk.value() { }
                // Read the pin
                let d = u16::from(crate::gpio::read_pin(data_pin));
                key_value |= d << i;
    
                // Wait to go high
                while !clk.value() { }
            }
    
            println!("Recieved {:x} total scancode", key_value);
    
            keys(key_value);
            // Data currently has the 0th bit
            // Wait a full cycle, sanity check that clk is low,
            //  measure next data bit
            //  repeat until clock we have 11 bits?
            //  (do we measure that the clock has stayed high?)
            // submit 11 bits to keyboard state machine
        }
    );

    unsafe {
        let _ = KEYBOARD.replace(k);
        DATA.replace(data.into_floating_input());
        // CLOCK.replace(clk.into_floating_input());
    }
}



pub fn send_reset() {
    // The unsafes in this are actually okay because we disallow interrupts
    // across the entire function
    riscv::interrupt::free(|| {
        if let (Some(clk), Some(data)) = unsafe { (CLOCK.take(), DATA.take()) } {
            InterruptPin::unlisten(&mut clk);
            let mut out_clk = clk.into_push_pull_output();
            let mut out_data = data.into_push_pull_output();
            let _ = out_clk.set_low();
            timer::delay(100);

            let _ = out_data.set_low();
            unsafe {
                CLOCK.replace(out_clk.into_floating_input());
            }

            // Clock is high, wait to go low
            while unsafe { &CLOCK }.as_ref().unwrap().is_high().unwrap() {}

            // Clock is low, so set data high
            let _ = out_data.set_high();

            for i in 1..11 {
                // wait for clk to go high
                while unsafe { &CLOCK }.as_ref().unwrap().is_low().unwrap() {}
                // wait for clk to go low, except for the last item
                if i < 10 {
                    while unsafe { &CLOCK }.as_ref().unwrap().is_high().unwrap() {}
                }
            }

            unsafe {
                DATA.replace(out_data.into_floating_input());
            }

            // Wait for one more clock low-high to not ignore the ACK
            while unsafe { &CLOCK }.as_ref().unwrap().is_high().unwrap() {}
            if let Some(data) = unsafe { &DATA } {
                if data.is_high().unwrap() {
                    panic!("We missed an ACK! AAAAACKKK!");
                }
            }
            while unsafe { &CLOCK }.as_ref().unwrap().is_low().unwrap() {}

            InterruptPin::listen(&mut clk, Event::FallingEdge);
        }
    })
}

pub fn keys(foo: u16) {
    let mut kb: Keyboard<layouts::Us104Key, ScancodeSet2> = Keyboard::new(HandleControl::Ignore);
    match kb.add_word(foo) {
        Ok(Some(x)) => {
            println!("{:?}", x);
            match kb.process_keyevent(x) {
                Some(y) => {
                    println!("{:?}", y);
                }
                None => {}
            }
        }
        Ok(None) => {}
        Err(e) => {
            println!("{:?}", e);
        }
    }
}
