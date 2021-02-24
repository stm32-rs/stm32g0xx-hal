#![deny(warnings)]
#![deny(unsafe_code)]
#![no_main]
#![no_std]

extern crate cortex_m;
extern crate cortex_m_rt as rt;
extern crate panic_halt;
extern crate stm32g0xx_hal as hal;

use hal::analog::comparator::{self, Config, Hysteresis, RefintInput};
use hal::prelude::*;
use hal::stm32;
use rt::entry;

#[entry]
fn main() -> ! {
    let dp = stm32::Peripherals::take().expect("cannot take peripherals");
    let mut rcc = dp.RCC.constrain();

    let gpioa = dp.GPIOA.split(&mut rcc);

    let pa1 = gpioa.pa1.into_analog();

    let comp = comparator::window_comparator12(
        dp.COMP,
        pa1,
        RefintInput::VRefintM14,
        RefintInput::VRefintM34,
        Config::default().hysteresis(Hysteresis::Medium),
        &mut rcc,
    );
    let comp = comp.enable();

    let mut led1 = gpioa.pa5.into_push_pull_output();
    let mut led2 = gpioa.pa6.into_push_pull_output();

    loop {
        match comp.output() {
            true => led1.set_high().unwrap(),
            false => led1.set_low().unwrap(),
        }
        match comp.above_lower() {
            true => led2.set_high().unwrap(),
            false => led2.set_low().unwrap(),
        }
    }
}
