use core::cell::RefCell;

use critical_section::Mutex;
use esp32c3_hal::{
    clock::Clocks,
    gpio::{Gpio3, Unknown, Gpio1},
    uart::{
        config::{Config, DataBits, Parity, StopBits},
        TxRxPins,
    },
    Cpu,
};
use esp32c3_hal::{interrupt, peripherals};
use esp32c3_hal::{peripherals::UART0, Uart};
use esp32c3_hal::{peripherals::UART1, prelude::*};
use esp_println::print;

use crate::channel::{self, Receiver, Sender};
use crate::interrupt::which_priority;

static SENDER0: Mutex<RefCell<Option<UartTransmitter<UART0, char>>>> =
    Mutex::new(RefCell::new(None));
static SENDER1: Mutex<RefCell<Option<UartTransmitter<UART1, u8>>>> = Mutex::new(RefCell::new(None));

pub fn configure0(uart: UART0) -> Receiver<char> {
    let serial0 = Uart::new(uart);
    let (tx, rx) = channel::channel();

    critical_section::with(|cs| {
        SENDER0.borrow_ref_mut(cs).replace(UartTransmitter {
            serial: serial0,
            tx,
        })
    });

    rx
}

pub fn configure1(
    uart: UART1,
    tx: Gpio1<Unknown>,
    rx: Gpio3<Unknown>,
    clocks: &Clocks,
) -> Receiver<u8> {
    let config = Config {
        baudrate: 400_000,
        data_bits: DataBits::DataBits8,
        parity: Parity::ParityNone,
        stop_bits: StopBits::STOP1,
    };

    let pins = TxRxPins::new_tx_rx(tx.into_push_pull_output(), rx.into_floating_input());
    let serial1 = Uart::new_with_config(uart, Some(config), Some(pins), clocks);
    let (tx, rx) = channel::channel();

    critical_section::with(|cs| {
        SENDER1.borrow_ref_mut(cs).replace(UartTransmitter {
            serial: serial1,
            tx,
        })
    });

    rx
}

// pub fn start_uart_poll_timer(interval_us: u64) {
//     riscv::interrupt::free(|| unsafe {
//         if let Some(sender) = &mut SENDER0 {
//             // let tx = sender.tx.clone();
//             crate::timer::start_repeat_timer0_callback(interval_us, || {
//                 // print!(".");
//                 while let nb::Result::Ok(c) = sender.serial.read() {
//                     sender.tx.send(c as char);
//                 }
//             });
//         }
//     })
// }

pub fn interrupt_enable0(priority: interrupt::Priority) {
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

    critical_section::with(|cs| {
        if let Some(sender) = SENDER0.borrow_ref_mut(cs).as_mut() {
            sender.serial.set_rx_fifo_full_threshold(1);
            sender.serial.listen_rx_fifo_full();
        }
    });
}

pub fn interrupt_enable1(priority: interrupt::Priority) {
    interrupt::enable(peripherals::Interrupt::UART1, which_priority(&priority)).unwrap();

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

    critical_section::with(|cs| {
        if let Some(sender) = SENDER1.borrow_ref_mut(cs).as_mut() {
            sender.serial.set_rx_fifo_full_threshold(1);
            sender.serial.listen_rx_fifo_full();
        }
    });
}

struct UartTransmitter<'a, S, Tx> {
    serial: Uart<'a, S>,
    tx: Sender<Tx>,
}

#[interrupt]
fn UART0() {
    critical_section::with(|cs| {
        if let Some(uart_transmitter) = SENDER0.borrow_ref_mut(cs).as_mut() {
            while let nb::Result::Ok(c) = uart_transmitter.serial.read() {
                // print!("{}", c as char);
                uart_transmitter.tx.send(c as char);
            }
            uart_transmitter.serial.reset_rx_fifo_full_interrupt();
        }
    });
}

#[interrupt]
fn UART1() {
    critical_section::with(|cs| {
        print!("x");
        if let Some(uart_transmitter) = SENDER1.borrow_ref_mut(cs).as_mut() {
            while let nb::Result::Ok(c) = uart_transmitter.serial.read() {
                // print!(".");
                uart_transmitter.tx.send(c);
            }
            uart_transmitter.serial.reset_rx_fifo_full_interrupt();
        }
    });
}
