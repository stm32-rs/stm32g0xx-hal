pub mod blocking;
pub mod config;

use crate::rcc::*;
pub use config::Config;

#[derive(Debug, Clone, Copy)]
pub enum SlaveAddressMask {
    MaskNone = 0,
    MaskOneBit,
    MaskTwoBits,
    MaskThreeBits,
    MaskFourBits,
    MaskFiveBits,
    MaskSixBits,
    MaskAllBits,
}

#[derive(Debug, Clone, Copy)]
pub enum I2cDirection {
    MasterReadSlaveWrite = 0,
    MasterWriteSlaveRead = 1,
}

/// I2C error
#[derive(Debug, Clone, Copy)]
pub enum Error {
    Overrun,
    Nack,
    PECError,
    BusError,
    ArbitrationLost,
    IncorrectFrameSize(usize),
}

/// I2C SDA pin
pub trait SDAPin<I2C> {
    fn setup(&self);
    fn release(self) -> Self;
}

/// I2C SCL pin
pub trait SCLPin<I2C> {
    fn setup(&self);
    fn release(self) -> Self;
}

pub trait I2cExt<I2C> {
    fn i2c<SDA, SCL>(
        self,
        sda: SDA,
        scl: SCL,
        config: impl Into<Config>,
        rcc: &mut Rcc,
    ) -> I2c<I2C, SDA, SCL>
    where
        SDA: SDAPin<I2C>,
        SCL: SCLPin<I2C>;
}

/// I2C abstraction
pub struct I2c<I2C, SDA, SCL> {
    i2c: I2C,
    sda: SDA,
    scl: SCL,
}
