use esp32c3_hal::dma::private::*;
use esp32c3_hal::dma::DmaPriority;
use esp32c3_hal::gdma::private::*;
use esp32c3_hal::gdma::Gdma;
use esp32c3_hal::gpio::{Gpio10, Gpio2, Gpio4, Gpio5, Gpio6, Gpio7};
use esp32c3_hal::peripherals::{DMA, SPI2};
use esp32c3_hal::prelude::*;
use esp32c3_hal::spi::dma::SpiDma;
use esp32c3_hal::spi::{Spi, SpiMode};
use esp32c3_hal::{clock::Clocks, gpio::OutputSignal};
use esp32c3_hal::{
    gpio::{OutputPin, Unknown},
    system::{Peripheral, PeripheralClockControl},
};

use esp_println::println;
use riscv::interrupt;

static mut QSPI: Option<
    SpiDma<
        '_,
        SPI2,
        ChannelTx<'_, Channel0TxImpl, Channel0>,
        ChannelRx<'_, Channel0RxImpl, Channel0>,
        SuitablePeripheral0,
    >,
> = None;

static mut DESCRIPTORS: [u32; 8 * 3] = [0u32; 8 * 3];
static mut RX_DESCRIPTORS: [u32; 3] = [0u32; 3]; // should be zero, but dma will get mad

///
/// Configure and initialize the Quad SPI instance. Once configured
/// data may be transmitted with the `transmit()` function.
///
pub fn configure(
    spi2: SPI2,
    sio0: Gpio7<Unknown>,
    sio1: Gpio2<Unknown>,
    sio2: Gpio5<Unknown>,
    sio3: Gpio4<Unknown>,
    cs: Gpio10<Unknown>,
    clk: Gpio6<Unknown>,
    dma: DMA,
    peripheral_clock: &mut PeripheralClockControl,
    clocks: &Clocks,
    freq: u32,
) {
    let dma = Gdma::new(dma, peripheral_clock);
    let dma_channel = dma.channel0;

    let qspi = Spi::new_quad_send_only(
        spi2,
        clk,
        sio0,
        sio1,
        sio2,
        sio3,
        cs,
        freq.Hz(),
        SpiMode::Mode1,
        peripheral_clock,
        clocks,
    )
    .with_dma(dma_channel.configure(
        false,
        unsafe { &mut DESCRIPTORS },
        unsafe { &mut RX_DESCRIPTORS },
        DmaPriority::Priority0,
    ));

    interrupt::free(|| unsafe {
        QSPI.replace(qspi);
    });
}

///
/// PANIC: if free is called before configure
///
// pub fn free() -> (
//     SPI2,
//     Gpio7<Unknown>,
//     Gpio2<Unknown>,
//     Gpio5<Unknown>,
//     Gpio4<Unknown>,
//     Gpio10<Unknown>,
//     Gpio6<Unknown>,
// ) {
//     let qspi = interrupt::free(|| unsafe { QSPI.take() });
//     let q = qspi.expect("QSPI must be configured before freed");
//     q.free()
// }

///
/// Transmit data, a slice of u8, if the qspi instance has been configured.
/// The buffer should be a length divisible by 4, and no longer than 32,768.
///
pub fn transmit(data: &'static mut [u8]) {
    static mut RECV: [u8; 0] = [];
    unsafe {
        if let Some(qspi) = QSPI.take() {
            let transfer = qspi.dma_transfer(data, &mut RECV).unwrap();
            // here we could do something else while DMA transfer is in progress
            // the buffers and spi is moved into the transfer and we can get it back via
            // `wait`
            let (_, _, q) = transfer.wait();

            QSPI.replace(q);

            // qspi.transfer(data);
            // qspi.with_dma(..).transfer(...)
            // unimplemented!()
        }
    }
}

///
/// Compute the closest available SPI clock frequency and corresponding register value
/// given the requested frequency. The clock speed equation is specified in chapter 26.7 of
/// https://www.espressif.com/sites/default/files/documentation/esp32-c3_technical_reference_manual_en.pdf
///
/// The SPI clock frequency is  given by:
///
/// Input Clock / ((Pre + 1) * (N + 1))
///
/// Where `Pre` and `N` are register values, and the Input Clock is the
/// APB_CLOCK, at 80 MHz.
///
fn clock_register_value(frequency: u32, clocks: &Clocks) -> u32 {
    // FIXME: this might not be always true
    let apb_clk_freq: u32 = clocks.apb_clock.to_Hz();
    println!("apb clock freq is {} MHz", apb_clk_freq / 1_000_000);

    let reg_val: u32;

    if frequency > (apb_clk_freq / 2) {
        // Using APB frequency directly will give us the best result here.
        reg_val = 1 << 31;
        println!("Using apb clock");
    } else {
        let mut best_n = 1;
        let mut best_pre = 0;
        for n in 1..64 {
            best_n = n;
            // This is at least 2, frequency must be less than apb_clock_freq
            let f_ratio = apb_clk_freq / frequency;
            best_pre = f_ratio / (n + 1) - 1;

            // pre is 4 bits
            if best_pre < 16 {
                break;
            }
        }
        let f = apb_clk_freq / ((best_pre + 1) * (best_n + 1));
        println!("using spi clock at {} MHz", f / 1_000_000);
        reg_val = clock_register(best_pre, best_n)
    }

    reg_val
}

///
/// Generates the SPI Clock register value given the PRE and N
/// input values
///
fn clock_register(pre: u32, n: u32) -> u32 {
    let l = n;
    let h = ((n + 1) / 2).saturating_sub(1);

    let reg_value = l | h << 6 | n << 12 | pre << 18;

    reg_value
}

pub trait QuadInstance {
    fn register_block(&self) -> &SPI2;

    fn sclk_signal(&self) -> OutputSignal;

    fn sio0(&self) -> OutputSignal;

    fn sio1(&self) -> OutputSignal;

    fn sio2(&self) -> OutputSignal;

    fn sio3(&self) -> OutputSignal;

    fn cs(&self) -> OutputSignal;

    fn enable_peripheral(&self, system: &mut PeripheralClockControl);

    fn init(&mut self, full_duplex: bool, allow_read: bool) {
        let block = self.register_block();
        block.user.write(|w| {
            w
                // Use all the buffer
                .usr_miso_highpart()
                .clear_bit()
                .usr_mosi_highpart()
                .clear_bit()
                .doutdin()
                .bit(full_duplex)
                .usr_mosi()
                .set_bit()
                .usr_miso()
                .bit(allow_read)
                .cs_hold()
                .set_bit()
                .usr_dummy()
                .clear_bit()
                .usr_addr()
                .clear_bit()
                .usr_command()
                .clear_bit()
                .fwrite_quad()
                .set_bit()
        });
        // println!("user: {:x}", block.user.read().bits());

        block.clk_gate.modify(|_, w| {
            w.clk_en()
                .set_bit()
                .mst_clk_active()
                .set_bit()
                .mst_clk_sel()
                .set_bit()
        });

        // println("dma_conf: {:x}", block.dma_conf.read().bits());
        // println!("dma_int_raw: {:x}", block.dma_int_raw.read().bits());
        // block.dma_conf.reset();
        // block.dma_int_raw.reset();
        // block.dma_int_.reset();
        // block.dma_int.reset();

        block.ctrl.write(|w| w.wr_bit_order().set_bit());
        block.misc.write(|w| unsafe { w.bits(0) });
        // Master mode
        block.slave.write(|w| unsafe { w.bits(0) });
    }

    fn setup(&mut self, target_frequency: u32, clocks: &Clocks) {
        let reg_value = clock_register_value(target_frequency, clocks);
        self.register_block()
            .clock
            .write(|w| unsafe { w.bits(reg_value) });
    }

    fn set_data_mode(&mut self, data_mode: embedded_hal::spi::Mode) -> &mut Self {
        let reg_block = self.register_block();

        match data_mode {
            embedded_hal::spi::MODE_0 => {
                reg_block.misc.modify(|_, w| w.ck_idle_edge().clear_bit());
                reg_block.user.modify(|_, w| w.ck_out_edge().clear_bit());
            }
            embedded_hal::spi::MODE_1 => {
                reg_block.misc.modify(|_, w| w.ck_idle_edge().clear_bit());
                reg_block.user.modify(|_, w| w.ck_out_edge().set_bit());
            }
            embedded_hal::spi::MODE_2 => {
                reg_block.misc.modify(|_, w| w.ck_idle_edge().set_bit());
                reg_block.user.modify(|_, w| w.ck_out_edge().set_bit());
            }
            embedded_hal::spi::MODE_3 => {
                reg_block.misc.modify(|_, w| w.ck_idle_edge().set_bit());
                reg_block.user.modify(|_, w| w.ck_out_edge().clear_bit());
            }
        }
        self
    }

    #[inline]
    fn update(&self) {
        let block = self.register_block();
        block.cmd.modify(|_, w| w.update().set_bit());
        while block.cmd.read().update().bit_is_set() {
            // wait
        }
    }

    #[inline]
    fn configure_datalen(&self, len: u32) {
        let block = self.register_block();

        block
            .ms_dlen
            .write(|w| unsafe { w.ms_data_bitlen().bits(len - 1) });
    }

    fn write_byte(&mut self, data: u8) {
        let block = self.register_block();

        while block.cmd.read().usr().bit_is_set() {
            // wait
        }

        self.configure_datalen(8);

        block.w0.write(|w| unsafe { w.bits(data as u32) });

        self.update();

        block.cmd.modify(|_, w| w.usr().set_bit());
    }

    fn write_word(&mut self, word: u32) {
        let block = self.register_block();

        block.dma_int_raw.reset(); // https://github.com/espressif/esp-idf/blob/7fb3e06c69ef150bbe6209b856be887cdb6cd5e9/components/hal/spi_hal_iram.c#L58
                                   // println!("W");
                                   // println("{:x}", block.dma_int_st.read().bits());
                                   // println!("dma_int_raw: {:x}", block.dma_int_raw.read().bits());
                                   // println!("dma_int_ena: {:x}", block.dma_int_ena.read().bits());
                                   // println!("dma_int_st: {:x}", block.dma_int_st.read().bits());
                                   // block.dma_int_clr
                                   // block.dma_
        while block.cmd.read().usr().bit_is_set() {
            // wait
            // println!("{:x}", block.dma_int_raw.read().bits());
            // // println!("{:?}", block.dma;
            // unsafe {
            //     delay(160211074);
            // }
        }
        // println!("op complete");

        self.configure_datalen(32);

        // println!("datalen configured");

        block.w0.write(|w| unsafe { w.bits(word) });

        // println!("store word");

        self.update();

        // println!("update clock");

        block.cmd.modify(|_, w| w.usr().set_bit());
        // println!("Done");
    }

    #[inline(always)]
    fn transfer(&mut self, data: &[u8]) {
        if data.len() % 4 != 0 {
            panic!("Data Length for SPI must be a multiple of 4");
        }
        self.configure_datalen(data.len() as u32 * 8);

        let block = self.register_block();
        // Wait until we know SPI is ready to proceed
        while block.cmd.read().usr().bit_is_set() {}

        let words: &[u32] =
            unsafe { core::slice::from_raw_parts(data.as_ptr().cast(), data.len() / 4) };

        // Before we take off, load the upper and lower parts of the buffer

        load_first_half(self.register_block(), &words[0..=7]);
        load_second_half(self.register_block(), &words[8..=15]);

        // Start!
        self.update();
        block.cmd.modify(|_, w| w.usr().set_bit());
        crate::start_cycle_count();

        // const WAITS: u8 = 51;
        let rest_words = &words[16..];
        for chunk in rest_words.array_chunks::<16>() {
            // print!("*");
            let first = &chunk[0..=7];
            let second = &chunk[8..=15];

            // Each half of the SPI buffer will take
            // (Fcpu / Fspi) * 8 spi cycles / 32 bit register * 8 registers in each half
            // So at 40 MHz, that's 256 cpu cycles
            const WAIT: u32 = 256;
            while crate::measure_cycle_count() < WAIT {}
            // Once we're done waiting for the first half to complete
            // Reset the CPU counter to 0
            crate::start_cycle_count();

            // we've passed the first half, so load that chunk now
            load_first_half(self.register_block(), first);

            // Wait for the second half to be done emitted from SPI
            while crate::measure_cycle_count() < WAIT {}
            crate::start_cycle_count();
            // Load the second half
            load_second_half(self.register_block(), second);
        }
    }
}

#[inline]
fn load_first_half(block: &SPI2, data: &[u32]) {
    block.w0.write(|w| unsafe { w.bits(data[0]) });
    block.w1.write(|w| unsafe { w.bits(data[1]) });
    block.w2.write(|w| unsafe { w.bits(data[2]) });
    block.w3.write(|w| unsafe { w.bits(data[3]) });
    block.w4.write(|w| unsafe { w.bits(data[4]) });
    block.w5.write(|w| unsafe { w.bits(data[5]) });
    block.w6.write(|w| unsafe { w.bits(data[6]) });
    block.w7.write(|w| unsafe { w.bits(data[7]) });
}

#[inline]
fn load_second_half(block: &SPI2, data: &[u32]) {
    block.w8.write(|w| unsafe { w.bits(data[0]) });
    block.w9.write(|w| unsafe { w.bits(data[1]) });
    block.w10.write(|w| unsafe { w.bits(data[2]) });
    block.w11.write(|w| unsafe { w.bits(data[3]) });
    block.w12.write(|w| unsafe { w.bits(data[4]) });
    block.w13.write(|w| unsafe { w.bits(data[5]) });
    block.w14.write(|w| unsafe { w.bits(data[6]) });
    block.w15.write(|w| unsafe { w.bits(data[7]) });
}

pub struct QSpi<I: QuadInstance> {
    spi_instance: I,
    sio0: Gpio7<Unknown>,
    sio1: Gpio2<Unknown>,
    sio2: Gpio5<Unknown>,
    sio3: Gpio4<Unknown>,
    cs: Gpio10<Unknown>,
    clk: Gpio6<Unknown>,
}

impl<I: QuadInstance> QSpi<I> {
    pub fn new(
        spi: I,
        mut sio0: Gpio7<Unknown>,
        mut sio1: Gpio2<Unknown>,
        mut sio2: Gpio5<Unknown>,
        mut sio3: Gpio4<Unknown>,
        mut cs: Gpio10<Unknown>,
        mut clk: Gpio6<Unknown>,
        system: &mut PeripheralClockControl,
        clocks: &Clocks,
        freq: u32,
    ) -> QSpi<I> {
        sio0.set_to_push_pull_output()
            .connect_peripheral_to_output(spi.sio0());
        sio1.set_to_push_pull_output()
            .connect_peripheral_to_output(spi.sio1());
        sio2.set_to_push_pull_output()
            .connect_peripheral_to_output(spi.sio2());
        sio3.set_to_push_pull_output()
            .connect_peripheral_to_output(spi.sio3());
        cs.set_to_push_pull_output()
            .connect_peripheral_to_output(spi.cs());
        clk.set_to_push_pull_output()
            .connect_peripheral_to_output(spi.sclk_signal());

        let mut qspi = QSpi {
            spi_instance: spi,
            sio0,
            sio1,
            sio2,
            sio3,
            cs,
            clk,
        };

        qspi.spi_instance.setup(freq, clocks);
        qspi.spi_instance.enable_peripheral(system);
        qspi.spi_instance.init(false, false);
        qspi.spi_instance.set_data_mode(embedded_hal::spi::MODE_1);

        qspi
    }

    pub fn write(&mut self, data: u8) {
        self.spi_instance.write_byte(data)
    }

    pub fn write_word(&mut self, word: u32) {
        self.spi_instance.write_word(word)
    }

    pub fn transfer(&mut self, data: &[u8]) {
        self.spi_instance.transfer(data)
    }

    pub fn free(
        self,
    ) -> (
        I,
        Gpio7<Unknown>,
        Gpio2<Unknown>,
        Gpio5<Unknown>,
        Gpio4<Unknown>,
        Gpio10<Unknown>,
        Gpio6<Unknown>,
    ) {
        (
            self.spi_instance,
            self.sio0,
            self.sio1,
            self.sio2,
            self.sio3,
            self.cs,
            self.clk,
        )
    }
}

impl QuadInstance for esp32c3_hal::peripherals::SPI2 {
    #[inline]
    fn register_block(&self) -> &SPI2 {
        self
    }

    fn sio0(&self) -> OutputSignal {
        OutputSignal::FSPID
    }

    fn sio1(&self) -> OutputSignal {
        OutputSignal::FSPIQ
    }

    fn sio2(&self) -> OutputSignal {
        OutputSignal::FSPIWP
    }

    fn sio3(&self) -> OutputSignal {
        OutputSignal::FSPIHD
    }

    fn sclk_signal(&self) -> OutputSignal {
        OutputSignal::FSPICLK_MUX
    }

    fn cs(&self) -> OutputSignal {
        OutputSignal::FSPICS0
    }

    fn enable_peripheral(&self, system: &mut PeripheralClockControl) {
        system.enable(Peripheral::Spi2);
    }
}
