use esp32c3_hal::prelude::*;
use esp32c3_hal::Cpu;
use esp32c3_hal::{interrupt, peripherals};
use esp32c3_hal::{peripherals::UART0, Uart};

use crate::channel::{self, Receiver, Sender};
use crate::interrupt::which_priority;

static mut SENDER: Option<UartTransmitter> = None;

pub fn configure(uart: UART0) -> Receiver<char> {
    let serial0 = Uart::new(uart);
    let (tx, rx) = channel::channel();

    riscv::interrupt::free(|| unsafe {
        SENDER.replace(UartTransmitter {
            serial: serial0,
            tx,
        })
    });

    rx
}

pub fn start_uart_poll_timer(interval_us: u64) {
    riscv::interrupt::free(|| unsafe {
        if let Some(sender) = &mut SENDER {
            // let tx = sender.tx.clone();
            crate::timer::start_repeat_timer0_callback(interval_us, || {
                // print!(".");
                while let nb::Result::Ok(c) = sender.serial.read() {
                    sender.tx.send(c as char);
                }
            });
        }
    })
}

pub fn interrupt_enable(priority: interrupt::Priority) {
    interrupt::enable(peripherals::Interrupt::UART0, which_priority(&priority)).unwrap();

    use interrupt::CpuInterrupt::*;
    use interrupt::Priority::*;
    let cpu_int = match priority {
        Priority1 => Interrupt1,
        Priority2 => Interrupt2,
        Priority3 => Interrupt3,
        Priority4 => Interrupt4,
        Priority5 => Interrupt5,
        Priority6 => Interrupt6,
        Priority7 => Interrupt7,
        Priority8 => Interrupt8,
        Priority9 => Interrupt9,
        Priority10 => Interrupt10,
        Priority11 => Interrupt11,
        Priority12 => Interrupt12,
        Priority13 => Interrupt13,
        Priority14 => Interrupt14,
        Priority15 => Interrupt15,
        None => Interrupt1,
    };

    interrupt::set_kind(
        Cpu::ProCpu,
        cpu_int, // Interrupt x handles priority x interrupts
        interrupt::InterruptKind::Edge,
    );

    riscv::interrupt::free(|| unsafe {
        if let Some(sender) = &mut SENDER {
            sender.serial.set_rx_fifo_full_threshold(1);
            sender.serial.listen_rx_fifo_full();
        }
    })
}

struct UartTransmitter<'a> {
    serial: Uart<'a, UART0>,
    tx: Sender<char>,
}

#[interrupt]
fn UART0() {
    riscv::interrupt::free(|| {
        if let Some(uart_transmitter) = unsafe { &mut SENDER } {
            while let nb::Result::Ok(c) = uart_transmitter.serial.read() {
                // print!("{}", c as char);
                uart_transmitter.tx.send(c as char);
            }
            uart_transmitter.serial.reset_rx_fifo_full_interrupt();
        }
    });
}
