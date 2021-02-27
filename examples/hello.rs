#![deny(warnings)]
#![deny(unsafe_code)]
#![no_main]
#![no_std]

extern crate cortex_m;
extern crate cortex_m_rt as rt;
extern crate panic_halt;
extern crate stm32g0xx_hal as hal;

use cortex_m_semihosting::hprintln;
use rt::entry;

#[allow(clippy::empty_loop)]
#[entry]
fn main() -> ! {
    hprintln!("Hello, STM32G0!").unwrap();

    loop {}
}
