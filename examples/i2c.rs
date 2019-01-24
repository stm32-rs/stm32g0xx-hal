#![deny(warnings)]
#![deny(unsafe_code)]
#![no_main]
#![no_std]

extern crate cortex_m;
extern crate cortex_m_rt as rt;
extern crate cortex_m_semihosting as sh;
extern crate panic_semihosting;
#[macro_use]
extern crate stm32g0xx_hal as hal;

use hal::prelude::*;
use hal::stm32;
use rt::entry;

#[entry]
fn main() -> ! {
    hal::debug::init();

    let dp = stm32::Peripherals::take().expect("cannot take peripherals");
    let mut rcc = dp.RCC.constrain();
    let gpiob = dp.GPIOB.split(&mut rcc);

    let sda = gpiob.pb7.into_open_drain_output();
    let scl = gpiob.pb6.into_open_drain_output();

    let mut i2c = dp.I2C1.i2c(sda, scl, 10.khz(), &mut rcc);

    let mut buf: [u8; 1] = [0; 1];
    loop {
        match i2c.read(0x60, &mut buf) {
            Ok(_) => println!("buf: {:?}", buf),
            Err(err) => println!("error: {:?}", err),
        }
    }
}
