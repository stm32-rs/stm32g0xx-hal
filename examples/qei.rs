#![deny(warnings)]
#![deny(unsafe_code)]
#![no_main]
#![no_std]

extern crate cortex_m;
extern crate cortex_m_rt as rt;
extern crate panic_semihosting;

#[macro_use]
extern crate stm32g0xx_hal as hal;

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
    let qei = dp.TIM1.qei((gpioa.pa8, gpioa.pa9), &mut rcc);

    loop {
        let before = qei.count();
        delay.delay(100.ms());
        let after = qei.count();

        let elapsed = after.wrapping_sub(before) as i16;
        println!("Δ: {}", elapsed);
    }
}
