#![deny(warnings)]
#![deny(unsafe_code)]
#![no_main]
#![no_std]

extern crate cortex_m;
extern crate cortex_m_rt as rt;
extern crate panic_semihosting;
#[macro_use]
extern crate stm32g0xx_hal as hal;

use hal::crc::*;
use hal::prelude::*;
use hal::stm32;
use rt::entry;

#[entry]
fn main() -> ! {
    hal::debug::init();
    let dp = stm32::Peripherals::take().expect("cannot take peripherals");
    let mut rcc = dp.RCC.constrain();

    let mut crc = dp.CRC.enable(&mut rcc);
    crc.reverse_input(InputReverse::Byte);
    crc.reverse_output(true);

    loop {
        crc.reset();
        let hash_sum = crc.digest("The quick brown fox jumps over the lazy dog");
        println!("crc32: 0x{:x}, crc32b: 0x{:x}", hash_sum, hash_sum ^ 0xffffffff);
    }
}
