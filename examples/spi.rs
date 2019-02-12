//#![deny(warnings)]
#![deny(unsafe_code)]
#![no_main]
#![no_std]

extern crate cortex_m;
extern crate cortex_m_rt as rt;
extern crate cortex_m_semihosting as sh;
extern crate panic_semihosting;
extern crate stm32g0xx_hal as hal;

use hal::prelude::*;
use hal::rcc::{RccConfig, SysClkSource};
use hal::spi;
use hal::stm32;
use rt::entry;

#[entry]
fn main() -> ! {
    hal::debug::init();

    let dp = stm32::Peripherals::take().expect("cannot take peripherals");
    let mut rcc = dp.RCC.freeze(RccConfig::new(SysClkSource::PLL));
    let gpioa = dp.GPIOA.split(&mut rcc);

    let sck = gpioa.pa1;
    let mosi = gpioa.pa2;
    let miso = gpioa.pa6;

    let mut spi = dp
        .SPI1
        .spi((sck, miso, mosi), spi::MODE_0, 3.mhz(), &mut rcc);

    loop {
        spi.send(128).unwrap_or_else(|_| ());
    }
}
