#![deny(warnings)]
#![deny(unsafe_code)]
#![no_main]
#![no_std]

extern crate cortex_m;
extern crate cortex_m_rt as rt;
extern crate panic_halt;
extern crate stm32g0xx_hal as hal;

use cortex_m::asm;
use hal::prelude::*;
use hal::stm32;
use rt::{entry, exception, ExceptionFrame};

#[allow(clippy::empty_loop)]
#[entry]
fn main() -> ! {
    let dp = stm32::Peripherals::take().expect("cannot take peripherals");

    let mut rcc = dp.RCC.constrain();
    let gpioa = dp.GPIOA.split(&mut rcc);
    let mut pwm = dp.TIM1.pwm(10.khz(), &mut rcc);

    let mut pwm_ch1 = pwm.bind_pin(gpioa.pa8);
    let mut pwm_ch2 = pwm.bind_pin(gpioa.pa9);

    let max = pwm_ch1.get_max_duty();
    pwm_ch1.set_duty(max / 2);
    pwm_ch2.set_duty(max / 4);

    pwm_ch1.enable();
    pwm_ch2.enable();
    asm::bkpt();

    pwm_ch1.set_duty(max / 4);
    pwm_ch2.set_duty(max / 8);
    asm::bkpt();

    pwm_ch1.set_duty(max / 8);
    pwm_ch2.set_duty(max / 16);
    asm::bkpt();

    pwm.set_freq(20.khz());

    loop {}
}

#[exception]
fn HardFault(ef: &ExceptionFrame) -> ! {
    panic!("Hard fault {:#?}", ef);
}

#[exception]
fn DefaultHandler(irqn: i16) {
    panic!("Unhandled exception (IRQn = {})", irqn);
}
