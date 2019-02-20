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
use hal::rcc::{Config, LSCOSrc, MCOSrc, Prescaler};
use hal::stm32;
use rt::entry;

#[entry]
fn main() -> ! {
    let dp = stm32::Peripherals::take().expect("cannot take peripherals");
    let mut rcc = dp.RCC.freeze(Config::lsi());
    let gpioa = dp.GPIOA.split(&mut rcc);

    let lsco = gpioa.pa2.lsco(LSCOSrc::LSI, &mut rcc);
    let mco = gpioa.pa9.mco(MCOSrc::SysClk, Prescaler::Div2, &mut rcc);

    lsco.enable();
    mco.enable();

    loop {}
}
