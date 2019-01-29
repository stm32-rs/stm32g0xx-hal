#![deny(warnings)]
#![deny(unsafe_code)]
#![no_main]
#![no_std]

extern crate cortex_m;
extern crate cortex_m_rt as rt;
extern crate cortex_m_semihosting as sh;
extern crate panic_semihosting;
extern crate stm32g0xx_hal as hal;

use hal::prelude::*;
use hal::{spi, stm32};
use rt::entry;

#[entry]
fn main() -> ! {
    hal::debug::init();

    let dp = stm32::Peripherals::take().expect("cannot take peripherals");
    let mut rcc = dp.RCC.constrain();
    let gpiob = dp.GPIOB.split(&mut rcc);

    let sck = gpiob.pb3;
    let miso = gpiob.pb4;
    let mosi = gpiob.pb5;

    let mut spi = dp
        .SPI1
        .spi((sck, miso, mosi), spi::MODE_0, 100.khz(), &mut rcc);

    loop {
        spi.write(&[0, 1]).unwrap();
    }
}
