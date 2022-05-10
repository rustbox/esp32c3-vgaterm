// hello

use embedded_hal::spi::Mode;
use esp32c3_hal::gpio::{Gpio7, Gpio10, Gpio6, Gpio2, Gpio5, Gpio4};
use esp_hal_common::{pac::{spi2::RegisterBlock, SYSTEM}, types::OutputSignal, Unknown, spi::Instance, OutputPin};
use crate::sprintln;

type System = SYSTEM;

pub trait QuadInstance {
    fn register_block(&self) -> &RegisterBlock;

    fn sclk_signal(&self) -> OutputSignal;

    fn sio0(&self) -> OutputSignal;

    fn sio1(&self) -> OutputSignal;

    fn sio2(&self) -> OutputSignal;

    fn sio3(&self) -> OutputSignal;

    fn cs(&self) -> OutputSignal;

    fn enable_peripheral(&self, system: &mut System);

    fn init(&mut self, full_duplex: bool, allow_read: bool) {
        let block = self.register_block();
        block.user.modify(|_, w| {
            w
                // Use all the buffer
                .usr_miso_highpart().clear_bit()
                .usr_mosi_highpart().clear_bit()
                .doutdin().bit(full_duplex)
                .usr_mosi().set_bit()
                .usr_miso().bit(allow_read)
                .cs_hold().set_bit()
                .usr_dummy().clear_bit()
                .usr_addr().clear_bit()
                .usr_command().clear_bit()
                .fwrite_quad().set_bit()
        });

        block.clk_gate.modify(|_, w| {
            w
                .clk_en().set_bit()
                .mst_clk_active().set_bit()
                .mst_clk_sel().set_bit()
        });

        block.ctrl.write(|w| unsafe { w.bits(0) });
        block.misc.write(|w| unsafe { w.bits(0) });
        // Master mode
        block.slave.write(|w| unsafe { w.bits(0) });
    }

    fn setup(&mut self) {
        // Use system clock as SPI clock
        self.register_block().clock.write(|w| unsafe { w.bits(1 << 31) })
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

    fn update(&self) {
        let block = self.register_block();
        block.cmd.modify(|_, w| w.update().set_bit());
        while block.cmd.read().update().bit_is_set() {
            // wait
        }
    }

    fn configure_datalen(&self, len: u32) {
        let block = self.register_block();

        block.ms_dlen.write(|w| unsafe { w.ms_data_bitlen().bits(len - 1) });
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

        sprintln!("Begin wait");
        while block.cmd.read().usr().bit_is_set() {
            // wait
        }
        sprintln!("Previous operation complete");

        self.configure_datalen(32);

        sprintln!("datalen configured");

        block.w0.write(|w| unsafe { w.bits(word) });

        sprintln!("store word");

        self.update();

        sprintln!("update clock");

        block.cmd.modify(| _, w | w.usr().set_bit());
        sprintln!("Done");
    }

    fn transfer(&mut self, data: &[u8]) {
        let block = self.register_block();

        let words_pointer: *const &[u32] = data.as_ptr().cast();
        let words: &[u32] = unsafe { words_pointer.as_ref().unwrap() };
        for chunk in words.chunks(16) {
            // Save words 16 at a time (64 bytes)
            // let buffer = unsafe { core::slice::from_raw_parts_mut(block.w0.as_ptr(), 16) };
            // for i in 0..chunk.len() {
            //     buffer[i] = chunk[i];
            // }
            block.w0.write(|w| unsafe { w.bits(chunk[0]) });
            block.w1.write(|w| unsafe { w.bits(chunk[1]) });
            block.w2.write(|w| unsafe { w.bits(chunk[2]) });
            block.w3.write(|w| unsafe { w.bits(chunk[3]) });
            block.w4.write(|w| unsafe { w.bits(chunk[4]) });
            block.w5.write(|w| unsafe { w.bits(chunk[5]) });
            block.w6.write(|w| unsafe { w.bits(chunk[6]) });
            block.w7.write(|w| unsafe { w.bits(chunk[7]) });
            block.w8.write(|w| unsafe { w.bits(chunk[8]) });
            block.w9.write(|w| unsafe { w.bits(chunk[9]) });
            block.w10.write(|w| unsafe { w.bits(chunk[10]) });
            block.w11.write(|w| unsafe { w.bits(chunk[11]) });
            block.w12.write(|w| unsafe { w.bits(chunk[12]) });
            block.w13.write(|w| unsafe { w.bits(chunk[13]) });
            block.w14.write(|w| unsafe { w.bits(chunk[14]) });
            block.w15.write(|w| unsafe { w.bits(chunk[15]) });
            // wait if currently in a write
            while block.cmd.read().usr().bit_is_set() {
                sprintln!("wait");
             }

            self.configure_datalen(chunk.len() as u32 * 32);
            self.update();

            block.cmd.modify(|_, w| w.usr().set_bit());
        }
    }
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
        system: &mut System,
    ) -> QSpi<I> {

        sio0.set_to_push_pull_output().connect_peripheral_to_output(spi.sio0());
        sio1.set_to_push_pull_output().connect_peripheral_to_output(spi.sio1());
        sio2.set_to_push_pull_output().connect_peripheral_to_output(spi.sio2());
        sio3.set_to_push_pull_output().connect_peripheral_to_output(spi.sio3());
        cs.set_to_push_pull_output().connect_peripheral_to_output(spi.cs());
        clk.set_to_push_pull_output().connect_peripheral_to_output(spi.sclk_signal());

        let mut qspi = QSpi {
            spi_instance: spi
        };
        
        qspi.spi_instance.enable_peripheral(system);
        qspi.spi_instance.setup();
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

    fn enable_peripheral(&self, system: &mut System) {
        system.perip_clk_en0.modify(|_, w| w.spi2_clk_en().set_bit());
        system.perip_rst_en0.modify(|_, w| w.spi2_rst().clear_bit());
    }
}
