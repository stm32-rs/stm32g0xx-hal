#![deny(warnings)]
#![deny(unsafe_code)]
#![no_main]
#![no_std]

extern crate cortex_m;
extern crate cortex_m_rt as rt;
extern crate panic_halt;
extern crate stm32g0xx_hal as hal;

use cortex_m_semihosting::hprintln;
use hal::prelude::*;
use hal::stm32;
use rt::entry;

#[entry]
fn main() -> ! {
    let dp = stm32::Peripherals::take().expect("cannot take peripherals");

    let mut rcc = dp.RCC.constrain();
    let gpioa = dp.GPIOA.split(&mut rcc);
    let gpioc = dp.GPIOC.split(&mut rcc);

    let mut switch = gpioc.pc5.into_pull_up_input();
    let qei = dp.TIM1.qei((gpioa.pa8, gpioa.pa9), &mut rcc);

    loop {
        let count = qei.count();
        if switch.is_low().unwrap() {
            hprintln!("Counter: {}", count);
        }
    }
}
