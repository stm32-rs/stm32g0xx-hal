#![deny(warnings)]
#![deny(unsafe_code)]
#![no_main]
#![no_std]

extern crate cortex_m;
extern crate cortex_m_rt as rt;
extern crate cortex_m_semihosting as sh;
extern crate nb;
extern crate panic_semihosting;
extern crate stm32g0xx_hal as hal;

use core::fmt::Write;

use embedded_sdmmc::{Controller, SdMmcSpi, TimeSource, Timestamp, VolumeIdx};
use hal::hal::digital::v1_compat::OldOutputPin;
use hal::prelude::*;
use hal::rcc;
use hal::serial;
use hal::spi;
use hal::stm32;
use rt::entry;

#[allow(clippy::empty_loop)]
#[entry]
fn main() -> ! {
    let dp = stm32::Peripherals::take().expect("cannot take peripherals");
    let cp = cortex_m::Peripherals::take().expect("cannot take core peripherals");

    let mut rcc = dp.RCC.freeze(rcc::Config::pll());
    let mut _delay = cp.SYST.delay(&mut rcc);
    let gpioa = dp.GPIOA.split(&mut rcc);
    let gpiob = dp.GPIOB.split(&mut rcc);

    let mut uart = dp
        .USART2
        .usart(
            gpioa.pa2,
            gpioa.pa3,
            serial::FullConfig::default(),
            &mut rcc,
        )
        .unwrap();

    let sdmmc_spi = dp.SPI1.spi(
        (gpiob.pb3, gpiob.pb4, gpiob.pb5),
        spi::MODE_0,
        400.khz(),
        &mut rcc,
    );

    let sdmmc_cs = OldOutputPin::new(gpioa.pa8.into_push_pull_output());
    let mut controller = Controller::new(SdMmcSpi::new(sdmmc_spi, sdmmc_cs), FakeTime {});

    writeln!(uart, "Init SD card...\r").unwrap();
    match controller.device().init() {
        Ok(_) => {
            write!(uart, "Card size... ").unwrap();
            match controller.device().card_size_bytes() {
                Ok(size) => writeln!(uart, "{}\r", size).unwrap(),
                Err(e) => writeln!(uart, "Err: {:?}", e).unwrap(),
            }
            writeln!(uart, "Volume 0:\r").unwrap();
            match controller.get_volume(VolumeIdx(0)) {
                Ok(volume) => {
                    let root_dir = controller.open_root_dir(&volume).unwrap();
                    writeln!(uart, "Listing root directory:\r").unwrap();
                    controller
                        .iterate_dir(&volume, &root_dir, |x| {
                            writeln!(uart, "Found: {:?}\r", x.name).unwrap();
                        })
                        .unwrap();
                    writeln!(uart, "End of listing\r").unwrap();
                }
                Err(e) => writeln!(uart, "Err: {:?}", e).unwrap(),
            }
        }
        Err(e) => writeln!(uart, "{:?}!", e).unwrap(),
    }

    loop {}
}

struct FakeTime;

impl TimeSource for FakeTime {
    fn get_timestamp(&self) -> Timestamp {
        Timestamp::from_calendar(1019, 11, 24, 3, 40, 31).unwrap()
    }
}
