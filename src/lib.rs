#![no_std]
#![allow(non_camel_case_types)]

#[cfg(not(any(feature = "stm32g070", feature = "stm32g071", feature = "stm32g081")))]
compile_error!(
    "This crate requires one of the following features enabled: stm32g070, stm32g071 or stm32g081"
);

extern crate bare_metal;
extern crate void;

pub extern crate cortex_m;
pub extern crate embedded_hal as hal;
pub extern crate nb;
pub extern crate stm32g0;

pub use nb::block;

#[cfg(feature = "stm32g070")]
pub use stm32g0::stm32g0x0 as stm32;

#[cfg(any(feature = "stm32g071", feature = "stm32g081"))]
pub use stm32g0::stm32g0x1 as stm32;

#[cfg(feature = "rt")]
pub use crate::stm32::interrupt;

#[macro_use]
pub mod debug;

pub mod adc;
pub mod crc;
pub mod dac;
pub mod delay;
pub mod dma;
pub mod exti;
pub mod gpio;
pub mod i2c;
pub mod prelude;
pub mod pwm;
pub mod qei;
pub mod rcc;
pub mod rng;
pub mod serial;
pub mod spi;
pub mod stopwatch;
pub mod time;
pub mod timer;
pub mod watchdog;
