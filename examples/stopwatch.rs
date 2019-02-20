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
use hal::rcc::{Config, SysClockSrc};
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
    let mut stopwatch = dp.TIM2.stopwatch(&mut rcc);

    let elapsed_us = stopwatch.trace(|| {
        delay.delay(10.us());
    });
    println!("Delay: 10us -> {}us", elapsed_us.0);

    timer.start(10.us());
    let elapsed_us = stopwatch.trace(|| {
        block!(timer.wait()).unwrap();
    });
    println!("Timer: 10us -> {}us", elapsed_us.0);

    let elapsed_us = stopwatch.trace(|| {
        let x = calc_something();
        assert!(x > 0);
    });
    println!("Calc @ 16MHz: {}us", elapsed_us.0);

    let rcc = rcc.freeze(Config::new(SysClockSrc::PLL));
    stopwatch.set_clock(rcc.clocks.apb_tim_clk);

    let elapsed_us = stopwatch.trace(|| {
        let x = calc_something();
        assert!(x > 0);
    });
    println!("Calc @ 64MHz: {}us", elapsed_us.0);

    loop {}
}

fn calc_something() -> u32 {
    let mut result = 0;
    for i in 1..1_000 {
        result = (result + i) / 3
    }
    result
}
