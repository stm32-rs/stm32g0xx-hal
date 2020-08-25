#![deny(warnings)]
#![deny(unsafe_code)]
#![no_main]
#![no_std]

extern crate cortex_m;
extern crate cortex_m_rt as rt;
extern crate panic_halt;
extern crate stm32g0xx_hal as hal;

use cortex_m_semihosting::hprintln;
use hal::crc::*;
use hal::prelude::*;
use hal::stm32;
use rt::entry;

#[entry]
fn main() -> ! {
    let dp = stm32::Peripherals::take().expect("cannot take peripherals");
    let mut rcc = dp.RCC.constrain();

    let crc = dp.CRC.constrain(&mut rcc);
    let mut crc = crc
        .input_bit_reversal(Some(BitReversal::ByWord))
        .output_bit_reversal(true)
        .freeze();

    loop {
        crc.reset();
        crc.feed(b"123456789");

        let hash_sum = crc.result();
        hprintln!(
            "crc32: 0x{:x}, crc32b: 0x{:x}",
            hash_sum,
            hash_sum ^ 0xffff_ffff
        )
        .unwrap();

        crc.reset();
        crc.feed(b"The quick brown fox jumps over the lazy dog");
        let hash_sum = crc.result();
        hprintln!(
            "crc32: 0x{:x}, crc32b: 0x{:x}",
            hash_sum,
            hash_sum ^ 0xffff_ffff
        )
        .unwrap();
    }
}
