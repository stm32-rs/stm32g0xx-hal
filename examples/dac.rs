#![deny(warnings)]
#![deny(unsafe_code)]
#![no_main]
#![no_std]

extern crate cortex_m;
extern crate cortex_m_rt as rt;
extern crate panic_semihosting;
extern crate stm32g0xx_hal as hal;

use hal::hal::Direction;
use hal::prelude::*;
use hal::stm32;
use rt::entry;

#[entry]
fn main() -> ! {
    hal::debug::init();

    let dp = stm32::Peripherals::take().unwrap();
    let cp = cortex_m::Peripherals::take().unwrap();

    let mut rcc = dp.RCC.constrain();
    let mut delay = cp.SYST.delay(&rcc.clocks);

    let gpioa = dp.GPIOA.split(&mut rcc);
    let mut dac = dp.DAC.dac(gpioa.pa4, &mut rcc);

    dac.calibrate(&mut delay);
    dac.enable();

    let mut dir = Direction::Upcounting;
    let mut val: u16 = 0;

    dac.set_value(2058_u16);
    cortex_m::asm::bkpt();

    dac.set_value(4095_u16);
    cortex_m::asm::bkpt();

    loop {
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
