#![no_std]
#![allow(non_camel_case_types)]

#[cfg(not(any(
    feature = "stm32g07x",
    feature = "stm32g030",
    feature = "stm32g031",
    feature = "stm32g041",
    feature = "stm32g081"
)))]
compile_error!(
    "This crate requires one of the following features enabled: stm32g07x, stm32g030, stm32g031, stm32g041, stm32g081"
);

extern crate bare_metal;
extern crate void;

pub extern crate cortex_m;
pub extern crate embedded_hal as hal;
pub extern crate nb;
pub extern crate stm32g0;

pub use nb::block;

#[cfg(feature = "stm32g07x")]
pub use stm32g0::stm32g07x as stm32;

#[cfg(feature = "stm32g030")]
pub use stm32g0::stm32g030 as stm32;

#[cfg(feature = "stm32g031")]
pub use stm32g0::stm32g031 as stm32;

#[cfg(feature = "stm32g041")]
pub use stm32g0::stm32g041 as stm32;

#[cfg(feature = "stm32g081")]
pub use stm32g0::stm32g081 as stm32;

#[cfg(feature = "rt")]
pub use crate::stm32::interrupt;

pub mod adc;
#[cfg(any(feature = "stm32g07x", feature = "stm32g081"))]
pub mod comparator;
pub mod crc;
#[cfg(any(feature = "stm32g07x", feature = "stm32g081"))]
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
pub mod opm;
pub mod watchdog;
