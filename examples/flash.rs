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

#[allow(clippy::empty_loop)]
#[entry]
fn main() -> ! {
    let dp = stm32::Peripherals::take().expect("cannot take peripherals");
    let flash = dp.FLASH;

    match flash.unlock() {
        Ok(mut unlocked) => {
            let page = FlashPage(10);
            unlocked.erase_page(page).expect("cannot erase Page10");

            let addr = page.to_address();
            unlocked
                .write(addr, &0xCAFE_BABE_FACE_B00Cu64.to_le_bytes())
                .expect("cannot write to Page10");

            let mut buffer = [0; 8];
            unlocked.read(addr, &mut buffer);
            hprintln!(
                "{:02X?}",
                u64::from_le_bytes((&buffer[0..8]).try_into().expect("never fails"))
            )
            .ok();
        }
        Err(_) => hprintln!("Cannot unlock flash").unwrap(),
    }

    loop {}
}
