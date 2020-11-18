// #![deny(warnings)]
#![deny(unsafe_code)]
#![no_main]
#![no_std]

#[cfg(not(any(feature = "stm32g071", feature = "stm32g081")))]
compile_error!("Only stm32g071 and stm32g081 have the DAC peripheral");

extern crate cortex_m;
extern crate cortex_m_rt as rt;
extern crate panic_halt;
extern crate stm32g0xx_hal as hal;

use hal::analog::dac::GeneratorConfig;
use hal::hal::Direction;
use hal::prelude::*;
use hal::rcc::Config;
use hal::stm32;
use rt::entry;

#[entry]
fn main() -> ! {
    let dp = stm32::Peripherals::take().expect("cannot take peripherals");
    let cp = cortex_m::Peripherals::take().expect("cannot take core peripherals");

    let mut rcc = dp.RCC.freeze(Config::pll());
    let mut delay = cp.SYST.delay(&mut rcc);

    let gpioa = dp.GPIOA.split(&mut rcc);
    let (dac0, dac1) = dp.DAC.constrain((gpioa.pa4, gpioa.pa5), &mut rcc);

    let mut dac = dac0.calibrate_buffer(&mut delay).enable();
    let mut generator = dac1.enable_generator(GeneratorConfig::noise(11));

    let mut dir = Direction::Upcounting;
    let mut val = 0;

    loop {
        generator.trigger();
        dac.set_value(val);
        match val {
            0 => dir = Direction::Upcounting,
            4095 => dir = Direction::Downcounting,
            _ => (),
        };

        match dir {
            Direction::Upcounting => val += 1,
            Direction::Downcounting => val -= 1,
        }
    }
}
