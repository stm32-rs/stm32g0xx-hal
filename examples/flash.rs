#![deny(warnings)]
#![deny(unsafe_code)]
#![no_main]
#![no_std]

extern crate cortex_m;
extern crate cortex_m_rt as rt;
extern crate panic_halt;
extern crate stm32g0xx_hal as hal;

use core::convert::TryInto;
use cortex_m_semihosting::hprintln;
use hal::flash::*;
use hal::stm32;
use rt::entry;

#[entry]
fn main() -> ! {
    let dp = stm32::Peripherals::take().expect("cannot take peripherals");
    let mut flash = dp.FLASH;

    flash.unlock().expect("cannot unlock");

    let page = 10;
    flash.erase_page(page).expect("cannot erase Page10");

    let addr = FLASH_START + page * PAGE_SIZE;
    flash.write_double_word(addr, 0xCAFEBABEFACEB00C).expect("cannot write to Page10");

    let data = flash.read(addr, 8).unwrap();
    hprintln!("{:02X?}", u64::from_le_bytes((&data[0..8]).try_into().expect("never fails"))).ok();

    loop {}
}
