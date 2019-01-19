#![deny(warnings)]
#![deny(unsafe_code)]
#![no_main]
#![no_std]

extern crate cortex_m;
extern crate cortex_m_rt as rt;
extern crate cortex_m_semihosting;
extern crate panic_semihosting;

#[macro_use]
extern crate stm32g0xx_hal as hal;

use hal::prelude::*;
use hal::stm32;
use rt::{entry, exception, ExceptionFrame};

#[entry]
fn main() -> ! {
    let dp = stm32::Peripherals::take().unwrap();
    let cp = cortex_m::Peripherals::take().unwrap();
    hal::debug::init();

    let mut rcc = dp.RCC.constrain();
    let mut delay = cp.SYST.delay(&rcc.clocks);

    println!("Starting watchdog");

    let mut watchdog = dp.WWDG.watchdog(&mut rcc);
    //let mut watchdog = dp.IWDG.watchdog();

    watchdog.start(100.ms());

    delay.delay(90.ms());
    //delay.delay(110.ms());

    cortex_m::asm::bkpt();

    loop {}
}

#[exception]
fn HardFault(ef: &ExceptionFrame) -> ! {
    panic!("Hard fault {:#?}", ef);
}
