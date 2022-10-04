// hello

use esp32c3_hal::gpio::{Gpio10, Gpio2, Gpio4, Gpio5, Gpio6, Gpio7};
use esp32c3_hal::clock::Clocks;
use esp_hal_common::pac::SPI2;
use esp_hal_common::{
    pac::spi2::RegisterBlock,
    types::OutputSignal,
    OutputPin, Unknown, system::{PeripheralClockControl, Peripheral},
};
use riscv::interrupt;

use crate::{sprintln, sprint};

static mut QSPI: Option<QSpi<SPI2>> = None;

pub fn configure(
    spi2: SPI2,
    sio0: Gpio7<Unknown>,
    sio1: Gpio2<Unknown>,
    sio2: Gpio5<Unknown>,
    sio3: Gpio4<Unknown>,
    cs: Gpio10<Unknown>,
    clk: Gpio6<Unknown>,
    peripheral_clock: &mut PeripheralClockControl,
    clocks: &Clocks,
    freq: u32) {

    let qspi = QSpi::new(
        spi2,
        sio0,
        sio1,
        sio2,
        sio3,
        cs,
        clk,
        peripheral_clock,
        clocks,
        freq
    );

    interrupt::free(|_| unsafe {
        QSPI.replace(qspi);
    });
}

pub fn transmit(data: &[u8]) {
    unsafe {
        if let Some(qspi) = QSPI.as_mut() {
            qspi.transfer(data);
        }
    }
}

fn clock_register_value(frequency: u32, clocks: &Clocks) -> u32 {
    // FIXME: this might not be always true
    let apb_clk_freq: u32 = clocks.apb_clock.to_Hz();
    sprintln!("apb clock freq is {} MHz", apb_clk_freq / 1_000_000);

    let reg_val: u32;
    let duty_cycle = 128;

    // In HW, n, h and l fields range from 1 to 64, pre ranges from 1 to 8K.
    // The value written to register is one lower than the used value.

    if frequency > ((apb_clk_freq / 4) * 3) {
        // Using APB frequency directly will give us the best result here.
        reg_val = 1 << 31;
        sprintln!("Using apb clock");
    } else {
        /* For best duty cycle resolution, we want n to be as close to 32 as
         * possible, but we also need a pre/n combo that gets us as close as
         * possible to the intended frequency. To do this, we bruteforce n and
         * calculate the best pre to go along with that. If there's a choice
         * between pre/n combos that give the same result, use the one with the
         * higher n.
         */

        let mut pre: i32;
        let mut bestn: i32 = -1;
        let mut bestpre: i32 = -1;
        let mut besterr: i32 = 0;
        let mut errval: i32;

        /* Start at n = 2. We need to be able to set h/l so we have at least
         * one high and one low pulse.
         */

        for n in 2..64 {
            /* Effectively, this does:
             *   pre = round((APB_CLK_FREQ / n) / frequency)
             */

            pre = ((apb_clk_freq as i32 / n) + (frequency as i32 / 2))
                / frequency as i32;

            if pre <= 0 {
                pre = 1;
            }

            if pre > 16 {
                pre = 16;
            }

            errval = (apb_clk_freq as i32 / (pre as i32 * n as i32)
                - frequency as i32)
                .abs();
            if bestn == -1 || errval <= besterr {
                besterr = errval;
                bestn = n as i32;
                bestpre = pre as i32;
            }
        }

        let n: i32 = bestn;
        pre = bestpre as i32;
        let l: i32 = n;

        /* Effectively, this does:
         *   h = round((duty_cycle * n) / 256)
         */

        let mut h: i32 = (duty_cycle * n + 127) / 256;
        if h <= 0 {
            h = 1;
        }

        reg_val = (l as u32 - 1)
            | ((h as u32 - 1) << 6)
            | ((n as u32 - 1) << 12)
            | ((pre as u32 - 1) << 18);
        
        // f is system/(spi_clkdiv_pre+1)/(spi_clkcnt_N+1)
        let f = apb_clk_freq / ((pre + 1) as u32 * (n + 1) as u32);
        sprintln!("using spi clock at {} MHz", f / 1_000_000);
    }

    reg_val
}


pub trait QuadInstance {
    fn register_block(&self) -> &RegisterBlock;

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
        // sprintln!("user: {:x}", block.user.read().bits());

        block.clk_gate.modify(|_, w| {
            w.clk_en()
                .set_bit()
                .mst_clk_active()
                .set_bit()
                .mst_clk_sel()
                .set_bit()
        });

        // sprintln!("dma_conf: {:x}", block.dma_conf.read().bits());
        // sprintln!("dma_int_raw: {:x}", block.dma_int_raw.read().bits());
        // block.dma_conf.reset();
        // block.dma_int_raw.reset();
        // block.dma_int_.reset();
        // block.dma_int.reset();

        block.ctrl.write(|w|  w.wr_bit_order().set_bit() );
        block.misc.write(|w| unsafe { w.bits(0) });
        // Master mode
        block.slave.write(|w| unsafe { w.bits(0) });
    }

    fn setup(&mut self, target_frequency: u32, clocks: &Clocks) {
        let reg_value = clock_register_value(target_frequency, clocks);
        self.register_block()
            .clock
            .write(|w| unsafe { w.bits(reg_value) } );
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
                                   // sprintln!("W");
                                   // sprintln!("{:x}", block.dma_int_st.read().bits());
                                   // sprintln!("dma_int_raw: {:x}", block.dma_int_raw.read().bits());
                                   // sprintln!("dma_int_ena: {:x}", block.dma_int_ena.read().bits());
                                   // sprintln!("dma_int_st: {:x}", block.dma_int_st.read().bits());
                                   // block.dma_int_clr
                                   // block.dma_
        while block.cmd.read().usr().bit_is_set() {
            // wait
            // sprintln!("{:x}", block.dma_int_raw.read().bits());
            // // sprintln!("{:?}", block.dma;
            // unsafe {
            //     delay(160211074);
            // }
        }
        // sprintln!("op complete");

        self.configure_datalen(32);

        // sprintln!("datalen configured");

        block.w0.write(|w| unsafe { w.bits(word) });

        // sprintln!("store word");

        self.update();

        // sprintln!("update clock");

        block.cmd.modify(|_, w| w.usr().set_bit());
        // sprintln!("Done");
    }

    fn transfer(&mut self, data: &[u8]) {
        self.configure_datalen(data.len() as u32 * 32);
        
        let block = self.register_block();
        // Wait until we know SPI is ready to proceed
        while block.cmd.read().usr().bit_is_set() { }
        
        let words: &[u32] =
            unsafe { core::slice::from_raw_parts(data.as_ptr().cast(), data.len() / 4) };
        
        // Before we take off, load the upper and lower parts of the buffer
        
        load_first_half(self.register_block(), &words[0..=7]);
        load_second_half(self.register_block(), &words[8..=15]);
        
        // Start!
        self.update();
        block.cmd.modify(|_, w| w.usr().set_bit());
        // crate::start_cycle_count();

        let waits = 42;
        let rest_words = &words[16..];
        for chunk in rest_words.array_chunks::<16>() {
            // sprint!("*");
            let first = &chunk[0..=7];
            let second = &chunk[8..=15];

            // Wait half the time it takes for SPI to consume the buffer
            // (8 words ) * 4 bytes in 1 cpu cycle per 1 byte in 1 spi cycle * ratio of Fcpu to Fspi
            // cpu is 160, spi is 40, so x4
            // SPI actually only puts half a byte at a time
            crate::noops(waits);
            // we've passed the first half, so load that chunk now
            load_first_half(self.register_block(), first);
            // Wait for the second half to be done emitted from SPI
            crate::noops(waits);
            // Load the second half
            load_second_half(self.register_block(), second);
        }
        
        while block.cmd.read().usr().bit_is_set() { }
        // let m = crate::measure_cycle_count();
        // sprintln!("SPI took {} cycles for {}", m, data.len());
        // for chunk in words.chunks(16) {
        //     // wait if currently in a write
            

        //     block.w0.write(|w| unsafe { w.bits(chunk[0]) });
        //     block.w1.write(|w| unsafe { w.bits(chunk[1]) });
        //     block.w2.write(|w| unsafe { w.bits(chunk[2]) });
        //     block.w3.write(|w| unsafe { w.bits(chunk[3]) });
        //     block.w4.write(|w| unsafe { w.bits(chunk[4]) });
        //     block.w5.write(|w| unsafe { w.bits(chunk[5]) });
        //     block.w6.write(|w| unsafe { w.bits(chunk[6]) });
        //     block.w7.write(|w| unsafe { w.bits(chunk[7]) });
        //     block.w8.write(|w| unsafe { w.bits(chunk[8]) });
        //     block.w9.write(|w| unsafe { w.bits(chunk[9]) });
        //     block.w10.write(|w| unsafe { w.bits(chunk[10]) });
        //     block.w11.write(|w| unsafe { w.bits(chunk[11]) });
        //     block.w12.write(|w| unsafe { w.bits(chunk[12]) });
        //     block.w13.write(|w| unsafe { w.bits(chunk[13]) });
        //     block.w14.write(|w| unsafe { w.bits(chunk[14]) });
        //     block.w15.write(|w| unsafe { w.bits(chunk[15]) });

        //     self.configure_datalen(chunk.len() as u32 * 32);
        //     self.update();

        //     block.cmd.modify(|_, w| w.usr().set_bit());
        // }
    }
}

#[inline]
fn load_first_half(block: &RegisterBlock, data: &[u32]) {
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
fn load_second_half(block: &RegisterBlock, data: &[u32]) {
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
        freq: u32
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

        let mut qspi = QSpi { spi_instance: spi };

        qspi.spi_instance.setup(freq, clocks);
        qspi.spi_instance.enable_peripheral(system);
        qspi.spi_instance.init(false, false);
        qspi.spi_instance.set_data_mode(embedded_hal::spi::MODE_0);

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
}

impl QuadInstance for esp32c3_hal::pac::SPI2 {
    #[inline]
    fn register_block(&self) -> &RegisterBlock {
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
