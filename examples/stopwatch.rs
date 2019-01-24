#![deny(warnings)]
#![deny(unsafe_code)]
#![no_main]
#![no_std]

extern crate cortex_m;
extern crate cortex_m_rt as rt;
extern crate nb;
extern crate panic_semihosting;
#[macro_use]
extern crate stm32g0xx_hal as hal;

use hal::prelude::*;
use hal::stm32;
use rt::entry;

#[entry]
fn main() -> ! {
    hal::debug::init();

    let cp = cortex_m::Peripherals::take().expect("cannot take core peripherals");
    let dp = stm32::Peripherals::take().expect("cannot take peripherals");
    let mut rcc = dp.RCC.constrain();

    let mut delay = cp.SYST.delay(&rcc.clocks);
    let mut timer = dp.TIM17.timer(&mut rcc);
    let stopwatch = dp.TIM2.stopwatch(&mut rcc);

    let elapsed_us = stopwatch.trace(|| {
        delay.delay(200.us());
    });
    println!("Delay: 200us -> {}us", elapsed_us.0);

    timer.start(200.us());
    let elapsed_us = stopwatch.trace(|| {
        block!(timer.wait()).unwrap();
    });
    println!("Timer: 200us -> {}us", elapsed_us.0);

    loop {}
}
