#![no_std]
#![allow(non_camel_case_types)]

#[cfg(not(feature = "device-selected"))]
compile_error!(
    "This crate requires one of the following features enabled: stm32g030, stm32g070, stm32g031, stm32g041, stm32g071, stm32g081"
);

extern crate bare_metal;
extern crate void;

pub extern crate cortex_m;
pub extern crate embedded_hal as hal;
pub extern crate nb;
pub extern crate stm32g0;

pub use nb::block;

#[cfg(feature = "stm32g030")]
pub use stm32g0::stm32g030 as stm32;

#[cfg(feature = "stm32g031")]
pub use stm32g0::stm32g031 as stm32;

#[cfg(feature = "stm32g041")]
pub use stm32g0::stm32g041 as stm32;

#[cfg(feature = "stm32g071")]
pub use stm32g0::stm32g071 as stm32;

#[cfg(feature = "stm32g081")]
pub use stm32g0::stm32g081 as stm32;

#[cfg(feature = "stm32g070")]
pub use stm32g0::stm32g070 as stm32;

#[cfg(feature = "rt")]
pub use crate::stm32::interrupt;

pub mod analog;
pub mod crc;
pub mod dma;
pub mod dmamux;
pub mod exti;
pub mod flash;
pub mod gpio;
pub mod i2c;
pub mod power;
pub mod prelude;
pub mod rcc;
#[cfg(any(feature = "stm32g041", feature = "stm32g081"))]
pub mod rng;
pub mod rtc;
pub mod serial;
pub mod spi;
pub mod time;
pub mod timer;
pub mod watchdog;

#[cfg(feature = "device-selected")]
mod sealed {
    pub trait Sealed {}
}
#[cfg(feature = "device-selected")]
pub(crate) use sealed::Sealed;
