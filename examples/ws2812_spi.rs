#![deny(warnings)]
#![deny(unsafe_code)]
#![no_main]
#![no_std]

extern crate cortex_m;
extern crate cortex_m_rt as rt;
extern crate cortex_m_semihosting as sh;
extern crate nb;
extern crate panic_halt;
extern crate stm32g0xx_hal as hal;

use hal::prelude::*;
use hal::rcc::{self, PllConfig};
use hal::spi;
use hal::stm32;
use rt::entry;
use smart_leds::{SmartLedsWrite, RGB};
use ws2812_spi as ws2812;

#[entry]
fn main() -> ! {
    let dp = stm32::Peripherals::take().expect("cannot take peripherals");
    let cp = cortex_m::Peripherals::take().expect("cannot take core peripherals");

    // Configure APB bus clock to 48MHz, cause ws2812 requires 3Mbps SPI
    let pll_cfg = PllConfig::with_hsi(4, 24, 2);
    let rcc_cfg = rcc::Config::pll().pll_cfg(pll_cfg);
    let mut rcc = dp.RCC.freeze(rcc_cfg);

    let mut delay = cp.SYST.delay(&mut rcc);
    let gpioa = dp.GPIOA.split(&mut rcc);
    let spi = dp.SPI2.spi(
        (spi::NoSck, spi::NoMiso, gpioa.pa10),
        ws2812::MODE,
        3.mhz(),
        &mut rcc,
    );
    let mut ws = ws2812::Ws2812::new(spi);

    let mut cnt: usize = 0;
    let mut data: [RGB<u8>; 64] = [RGB::default(); 64];
    loop {
        for (idx, color) in data.iter_mut().enumerate() {
            *color = match (cnt + idx) % 8 {
                0 => RGB { r: 8, g: 0, b: 0 },
                1 => RGB { r: 0, g: 4, b: 0 },
                2 => RGB { r: 0, g: 0, b: 2 },
                _ => RGB { r: 0, g: 0, b: 0 },
            };
        }
        ws.write(data.iter().cloned()).unwrap();
        cnt += 1;
        delay.delay(50.ms());
    }
}
