#![deny(warnings)]
#![no_main]
#![no_std]

extern crate cortex_m;
extern crate cortex_m_rt as rt;
extern crate panic_probe;
extern crate stm32g0xx_hal as hal;

#[cfg(not(any(feature = "stm32g041", feature = "stm32g081")))]
compile_error!("Only stm32g041 and stm32g081 have the AES peripheral");

use defmt_rtt as _;
use hal::aes::Key;
use hal::prelude::*;
use hal::stm32;
use rt::entry;

#[entry]
fn main() -> ! {
    let dp = stm32::Peripherals::take().expect("cannot take peripherals");
    let mut rcc = dp.RCC.constrain();
    let aes = dp.AES.constrain(&mut rcc);
    let message = b"The quick brown ";
    let key = Key::try_from_slice(&[01; 32]).unwrap();

    let mut aes_ecb_encrypt = aes.ecb_encrypt(key);
    let encrypted = aes_ecb_encrypt.process(&message).unwrap();
    defmt::info!("encrypred: {:02x}, check: [c9, ca, 52, 28, e5, f8, f2, 7e, ce, b9, 5b, 4d, 3c, 51, 77, 10]", encrypted);

    let mut aes_ecb_decrypt = aes_ecb_encrypt.disable().ecb_decrypt(key);
    let decrypted = aes_ecb_decrypt.process(&encrypted).unwrap();
    defmt::info!(
        "decrypted: \"{}\"",
        core::str::from_utf8(&decrypted).unwrap()
    );

    loop {}
}
