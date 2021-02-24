pub mod adc;
#[cfg(any(feature = "stm32g071", feature = "stm32g081"))]
pub mod comparator;
#[cfg(any(feature = "stm32g071", feature = "stm32g081"))]
pub mod dac;
