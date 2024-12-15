#![deny(warnings)]
#![deny(unsafe_code)]
#![no_main]
#![no_std]

extern crate cortex_m;
extern crate cortex_m_rt as rt;
extern crate cortex_m_semihosting as sh;
extern crate panic_halt;
extern crate stm32g0xx_hal as hal;

use hal::i2c::Config;
use hal::prelude::*;
use hal::stm32;
use rt::entry;
use sh::hprintln;

#[entry]
fn main() -> ! {
    let dp = stm32::Peripherals::take().expect("cannot take peripherals");
    let mut rcc = dp.RCC.constrain();
    let gpiob = dp.GPIOB.split(&mut rcc);

    let sda = gpiob.pb7.into_open_drain_output_in_state(PinState::High);
    let scl = gpiob.pb6.into_open_drain_output_in_state(PinState::High);

    let mut i2c = dp
        .I2C1
        .i2c(sda, scl, Config::with_timing(0x2020_151b), &mut rcc);

    let buf: [u8; 1] = [0];
    loop {
        match i2c.write(0x3c, &buf) {
            Ok(_) => hprintln!("ok"),
            Err(err) => hprintln!("error: {:?}", err),
        }
    }
}
