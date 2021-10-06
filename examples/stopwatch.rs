#![deny(warnings)]
#![deny(unsafe_code)]
#![no_main]
#![no_std]

extern crate cortex_m;
extern crate cortex_m_rt as rt;
#[macro_use]
extern crate nb;
extern crate panic_halt;
extern crate stm32g0xx_hal as hal;

use cortex_m_semihosting::hprintln;
use hal::prelude::*;
use hal::rcc::{Config, SysClockSrc};
use hal::stm32;
use rt::entry;

#[allow(clippy::empty_loop)]
#[entry]
fn main() -> ! {
    let cp = cortex_m::Peripherals::take().expect("cannot take core peripherals");
    let dp = stm32::Peripherals::take().expect("cannot take peripherals");
    let mut rcc = dp.RCC.constrain();

    let mut delay = cp.SYST.delay(&mut rcc);
    let mut timer = dp.TIM17.timer(&mut rcc);

    #[cfg(feature = "stm32g0x1")]
    let mut stopwatch = dp.TIM2.stopwatch(&mut rcc);
    #[cfg(feature = "stm32g0x0")]
    let mut stopwatch = dp.TIM3.stopwatch(&mut rcc);

    let elapsed_us = stopwatch.trace(|| {
        delay.delay(100.us());
    });
    hprintln!("Delay: 100 us -> {} us", elapsed_us.0).unwrap();

    timer.start(100.us());
    let elapsed_us = stopwatch.trace(|| {
        block!(timer.wait()).unwrap();
    });
    hprintln!("Timer: 100 us -> {} us", elapsed_us.0).unwrap();

    let elapsed_us = stopwatch.trace(calc_something);
    hprintln!("Calc @ 16 MHz: {} us", elapsed_us.0).unwrap();

    let rcc = rcc.freeze(Config::new(SysClockSrc::PLL));
    stopwatch.set_clock(rcc.clocks.apb_tim_clk);

    let elapsed_us = stopwatch.trace(calc_something);
    hprintln!("Calc @ 64 MHz: {} us", elapsed_us.0).unwrap();

    loop {}
}

fn calc_something() {
    for _ in 1..100_500 {
        cortex_m::asm::nop();
    }
}
