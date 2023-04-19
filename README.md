# esp32c3-vgaterm
VGA Driver and Serial Terminal crates, with embedded applications in mind, written in Rust.

This is targeted at the RISC-V architecture esp32-c3 processor from espressif. The ISA is `rv32imc`.

# Installation and Prerquisites

1. Activate rust nightly:

`rustup override set nightly`

2. Activate the risc-v target:

`rustup target add riscv32imc-unknown-none-elf`

3. Using Cargo, install the `espflash` tool which can upload and flash the code across a serial
port (through usb) to the esp32 chip:

`cargo install espflash`

> You may need `libudev-dev` as a dependency on linux: `sudo apt install libudev-dev`

At this point you are ready to build and upload

# Building

1. Build:

`cargo build`

2. Flash to the CPU

`espflash flash /dev/ttyUSB0 target/riscv32imac-unknown-none-elf/debug/vgaterm --monitor --format direct-boot`

Where `/dev/ttyUSB0` should be whatever serial port the esp32 is connected to. It's highly recommended that your user
is added to the `dialout` group which will allow you to interact with `/dev/ttyUSB0` without using sudo.

Also it's important to note the `--format direct-boot` in order to properly flash the code in our bare metal environment.

# Notes
* See https://github.com/esp-rs/esp-hal/tree/main/esp32c3-hal/examples for examples
* We use "direct boot": https://github.com/esp-rs/espflash/issues/53
    * https://github.com/espressif/esp32c3-direct-boot-example
* ESP32 setup: https://esp-rs.github.io/book/tooling/espflash.html
* ESP32-C3 HAL docs: https://docs.rs/esp32c3/latest/esp32c3/

