#![deny(warnings)]
#![deny(unsafe_code)]
#![no_main]
#![no_std]

extern crate cortex_m;
extern crate cortex_m_rt as rt;
extern crate nb;
extern crate panic_semihosting;
extern crate stm32g0xx_hal as hal;

use hal::prelude::*;
use hal::stm32;
use nb::block;
use rt::entry;

#[entry]
fn main() -> ! {
    let dp = stm32::Peripherals::take().unwrap();
    let mut rcc = dp.RCC.constrain();

    let gpioa = dp.GPIOA.split(&mut rcc);
    let mut led = gpioa.pa5.into_push_pull_output();

    let mut timer = dp.TIM1.timer(2.hz(), &mut rcc);

    loop {
        led.toggle();
        block!(timer.wait()).unwrap();
    }
}
