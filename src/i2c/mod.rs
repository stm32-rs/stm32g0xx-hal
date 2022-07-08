#[cfg(feature = "i2c-blocking")]
pub mod blocking;

#[cfg(feature = "i2c-nonblocking")]
pub mod nonblocking;

#[cfg(feature = "i2c-nonblocking")]
pub use nonblocking::*;

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
pub enum I2cResult<'a> {
    Data(u16, I2cDirection, &'a [u8]), // contains address, direction and data slice reference
    Addressed(u16, I2cDirection),      // a slave is addressed by a master
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
#[cfg(feature = "i2c-blocking")]
pub struct I2c<I2C, SDA, SCL> {
    i2c: I2C,
    sda: SDA,
    scl: SCL,
}

#[cfg(feature = "i2c-nonblocking")]
pub struct I2c<I2C, SDA, SCL> {
    i2c: I2C,
    sda: SDA,
    scl: SCL,
    address: u16,
    watchdog: u16, // on each start set to 10, on each stop set to 0
    index: usize,
    length: usize,
    errors: usize,            // global error counter, reset on read
    length_write_read: usize, // for a master write_read operation this remembers the size of the read operation
    // for a slave device this must be 0
    data: [u8; 255], // during transfer the driver will be the owner of the buffer
}
