#![deny(warnings)]
#![deny(unsafe_code)]
#![no_main]
#![no_std]

extern crate cortex_m;
extern crate cortex_m_rt as rt;
extern crate panic_semihosting;
extern crate stm32g0xx_hal as hal;

use cortex_m_semihosting::hprintln;
use hal::prelude::*;
use hal::stm32;
use rt::entry;

#[entry]
fn main() -> ! {
    let dp = stm32::Peripherals::take().expect("cannot take peripherals");
    let cp = cortex_m::Peripherals::take().expect("cannot take core peripherals");

    let mut rcc = dp.RCC.constrain();
    let mut delay = cp.SYST.delay(&rcc.clocks);

    let gpioa = dp.GPIOA.split(&mut rcc);
    let switch = gpioa.pa2.into_pull_up_input();
    let qei = dp.TIM2.qei((gpioa.pa0, gpioa.pa1), &mut rcc);

    loop {
        if switch.is_low() {
            hprintln!("{:?}", qei.count()).unwrap();
        }
    }
}
