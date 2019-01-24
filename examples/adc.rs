#![deny(warnings)]
#![deny(unsafe_code)]
#![no_main]
#![no_std]

extern crate cortex_m;
extern crate cortex_m_rt as rt;
extern crate panic_semihosting;
#[macro_use]
extern crate stm32g0xx_hal as hal;

use hal::adc::{SampleTime, VTemp};
use hal::prelude::*;
use hal::stm32;
use rt::entry;

#[entry]
fn main() -> ! {
    hal::debug::init();

    let dp = stm32::Peripherals::take().expect("cannot take peripherals");
    let mut rcc = dp.RCC.constrain();
    let gpioa = dp.GPIOA.split(&mut rcc);

    let mut adc = dp.ADC.adc(&mut rcc);
    adc.set_sample_time(SampleTime::T_160);

    let mut adc_pin = gpioa.pa0.into_analog();
    let mut vtemp = VTemp::new();
    vtemp.enable(&mut adc);

    loop {
        let u: u32 = adc.read(&mut adc_pin).expect("adc read failed");
        let temp: u32 = adc.read(&mut vtemp).expect("temperature read failed");

        let u = 3300 * u / 4096;
        let temp = temp / 42;
        println!("u: {:?} mV | t: {:?}°C", u, temp);
    }
}
