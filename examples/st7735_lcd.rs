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

use embedded_graphics::image::Image16BPP;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::rectangle::Rectangle;
use hal::prelude::*;
use hal::rcc;
use hal::spi;
use hal::stm32;
use rt::entry;
use st7735_lcd::{self, Orientation};

#[entry]
fn main() -> ! {
    let dp = stm32::Peripherals::take().expect("cannot take peripherals");
    let cp = cortex_m::Peripherals::take().expect("cannot take core peripherals");
    let mut rcc = dp.RCC.freeze(rcc::Config::pll());

    let mut delay = cp.SYST.delay(&mut rcc);
    let gpioa = dp.GPIOA.split(&mut rcc);
    let dc = gpioa.pa1.into_push_pull_output();
    let rst = gpioa.pa10.into_push_pull_output();
    let spi = dp.SPI2.spi(
        (gpioa.pa0, gpioa.pa9, gpioa.pa4),
        spi::MODE_0,
        8.mhz(),
        &mut rcc,
    );

    let mut disp = st7735_lcd::ST7735::new(spi, dc, rst, false, true);
    disp.init(&mut delay).unwrap();
    disp.set_orientation(&Orientation::Landscape).unwrap();
    let black_backdrop =
        Rectangle::new(Coord::new(0, 0), Coord::new(160, 128)).fill(Some(0x0000u16.into()));
    disp.draw(black_backdrop.into_iter());
    let ferris = Image16BPP::new(include_bytes!("./ferris.raw"), 86, 64);
    let mut cnt = 0;
    loop {
        cnt = (cnt + 1) % 100;
        disp.draw(black_backdrop.into_iter());
        disp.draw(ferris.translate(Coord::new(cnt, cnt)).into_iter());
        delay.delay(200.ms());
    }
}
