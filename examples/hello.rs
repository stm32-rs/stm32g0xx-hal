#![deny(warnings)]
#![deny(unsafe_code)]
#![no_main]
#![no_std]

extern crate cortex_m;
extern crate cortex_m_rt as rt;
extern crate panic_semihosting;

#[macro_use]
extern crate stm32g0xx_hal as hal;

use rt::entry;

#[entry]
fn main() -> ! {
    hal::debug::init();
    println!("Hello, STM32G0!");

    loop {}
}
