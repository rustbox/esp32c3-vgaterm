use esp32c3_hal::{
    clock::Clocks,
    gpio::{Gpio0, Gpio1, Gpio3, Unknown},
    uart::{
        config::{Config, DataBits, Parity, StopBits},
        TxRxPins,
    },
    Cpu,
};
use esp32c3_hal::{interrupt, peripherals};
use esp32c3_hal::{peripherals::UART0, Uart};
use esp32c3_hal::{peripherals::UART1, prelude::*};

use crate::channel::{self, Receiver, Sender};
use crate::interrupt::which_priority;

static mut SENDER0: Option<UartTransmitter<UART0, char>> = None;
static mut SENDER1: Option<UartTransmitter<UART1, u8>> = None;

#[must_use]
pub fn configure0(uart: UART0) -> Receiver<char> {
    let serial0 = Uart::new(uart);
    let (tx, rx) = channel::channel();

    critical_section::with(|_cs| {
        unsafe { &mut SENDER0 }.replace(UartTransmitter {
            serial: serial0,
            tx,
        })
    });

    rx
}

pub fn configure1(
    uart: UART1,
    tx: Gpio1<Unknown>,
    rx: Gpio0<Unknown>,
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

    critical_section::with(|_cs| {
        unsafe { &mut SENDER1 }.replace(UartTransmitter {
            serial: serial1,
            tx,
        })
    });

    rx
}

pub fn make_uart0<'a>(uart: UART0) -> Uart<'a, UART0> {
    uart.flow_conf.write(|w| {
        w.sw_flow_con_en().set_bit()
    });
    uart.swfc_conf0.write(|w| {
        w.xoff_threshold().variant(64)
        .xoff_char().variant(0x13)
    });
    uart.swfc_conf1.write(|w| {
        w.xon_threshold().variant(64)
        .xon_char().variant(0x11)
    });
    let serial0: Uart<UART0> = Uart::new(uart);

    serial0
}

pub fn make_uart1<'a>(
    uart: UART1,
    tx: Gpio1<Unknown>,
    rx: Gpio3<Unknown>,
    clocks: &Clocks,
) -> Uart<'a, UART1> {
    let config = Config {
        baudrate: 400_000,
        data_bits: DataBits::DataBits8,
        parity: Parity::ParityNone,
        stop_bits: StopBits::STOP1,
    };
    let pins = TxRxPins::new_tx_rx(tx.into_push_pull_output(), rx.into_floating_input());
    let serial1 = Uart::new_with_config(uart, Some(config), Some(pins), clocks);

    serial1
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

    critical_section::with(|_cs| {
        if let Some(sender) = unsafe { &mut SENDER0 } {
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
        interrupt::InterruptKind::Level,
    );

    critical_section::with(|_cs| {
        if let Some(sender) = unsafe { &mut SENDER1 } {
            sender.serial.set_rx_fifo_full_threshold(1);
            sender.serial.listen_rx_fifo_full();
        }
    });
}

pub struct UartTransmitter<'a, S, Tx> {
    serial: Uart<'a, S>,
    tx: Sender<Tx>,
}

#[link_section = ".rwtext"] // #[ram] without #[inline(never)]
#[interrupt]
fn UART0() {
    crate::interrupt::theshold_mask(|| {
        // SAFETY: we've set the priority to one greater than our own, in threshold_mask, so we're not concurrently modifying SENDER0
        // (this code is not re-entrant), assuming nothing reconfigures the priority lol
        if let Some(uart_transmitter) = unsafe { &mut SENDER0 } {
            while let nb::Result::Ok(c) = uart_transmitter.serial.read() {
                // print!("{}", c as char);
                uart_transmitter.tx.send(c as char);
            }
        }
        // Reset the "fifo full" interrupt
        unsafe { (*UART0::PTR).int_clr.write(|w| w.rxfifo_full_int_clr().set_bit()) }
    });
}

#[interrupt]
fn UART1() {
    crate::interrupt::theshold_mask(|| {
        if let Some(uart_transmitter) = unsafe { &mut SENDER1 } {
            while let nb::Result::Ok(c) = uart_transmitter.serial.read() {
                uart_transmitter.tx.send(c);
            }
            uart_transmitter.serial.reset_rx_fifo_full_interrupt();
        }
    });
}
