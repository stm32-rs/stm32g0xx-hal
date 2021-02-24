#![deny(warnings)]
#![deny(unsafe_code)]
#![no_main]
#![no_std]

extern crate cortex_m;
extern crate cortex_m_rt as rt;
extern crate panic_halt;
extern crate stm32g0xx_hal as hal;

use hal::analog::comparator::{Config, Hysteresis, RefintInput};
use hal::prelude::*;
use hal::stm32;
use rt::entry;

#[entry]
fn main() -> ! {
    let dp = stm32::Peripherals::take().expect("cannot take peripherals");
    let mut rcc = dp.RCC.constrain();

    let gpioa = dp.GPIOA.split(&mut rcc);

    let (comp1, comp2) = dp.COMP.split(&mut rcc);

    let pa1 = gpioa.pa1.into_analog();
    let pa0 = gpioa.pa0.into_analog();
    let comp1 = comp1.comparator(pa1, pa0, Config::default(), &rcc.clocks);
    let comp1 = comp1.enable();
    let mut led1 = gpioa.pa5.into_push_pull_output();

    let pa3 = gpioa.pa3.into_analog();
    let comp2 = comp2.comparator(
        pa3,
        RefintInput::VRefintM12,
        Config::default()
            .hysteresis(Hysteresis::High)
            .output_inverted(),
        &rcc.clocks,
    );
    let led2 = gpioa.pa2.into_push_pull_output();
    // Configure PA2 to the comparator's alternate function so it gets
    // changed directly by the comparator.
    comp2.output_pin(led2);
    let _comp2 = comp2.enable();

    loop {
        match comp1.output() {
            true => led1.set_high().unwrap(),
            false => led1.set_low().unwrap(),
        }
    }
}
