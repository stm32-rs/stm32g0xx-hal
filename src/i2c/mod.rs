#[cfg(feature = "i2c-blocking")]
pub mod blocking;

#[cfg(feature = "i2c-nonblocking")]
pub mod nonblocking;

#[cfg(feature = "i2c-nonblocking")]
pub use nonblocking::*;

pub mod config;

use crate::rcc::{self, Rcc};
pub use config::Config;
use hal::i2c::{ErrorKind, NoAcknowledgeSource};

#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
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

/// Denotes which event marked the end of the I2C data
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum EndMarker {
    /// A stop condition was encountered
    Stop,
    /// A start repeat condition was encountered
    StartRepeat,
}

#[derive(Debug, Clone, Copy)]
pub enum I2cResult<'a> {
    /// Contains address, direction, data slice reference, and the packet delimeter
    Data(u16, I2cDirection, &'a [u8], EndMarker),
    Addressed(u16, I2cDirection), // a slave is addressed by a master
}

#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum I2cDirection {
    MasterReadSlaveWrite = 0,
    MasterWriteSlaveRead = 1,
}

#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Event {
    AddressMatch,
    Rxne,
}

/// I2C error
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Error {
    Overrun,
    Nack,
    PECError,
    BusError,
    ArbitrationLost,
    IncorrectFrameSize(usize),
}

impl hal::i2c::Error for Error {
    fn kind(&self) -> ErrorKind {
        match self {
            Error::Overrun => ErrorKind::Overrun,
            Error::BusError => ErrorKind::Bus,
            Error::ArbitrationLost => ErrorKind::ArbitrationLoss,
            Error::Nack => ErrorKind::NoAcknowledge(NoAcknowledgeSource::Unknown),
            _ => ErrorKind::Other,
        }
    }
}

pub trait Instance:
    crate::Sealed
    + core::ops::Deref<Target = crate::stm32::i2c1::RegisterBlock>
    + rcc::Enable
    + rcc::Reset
{
}

impl Instance for crate::stm32::I2C1 {}
impl Instance for crate::stm32::I2C2 {}

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

pub trait I2cExt: Sized {
    fn i2c<SDA, SCL>(
        self,
        sda: SDA,
        scl: SCL,
        config: impl Into<Config>,
        rcc: &mut Rcc,
    ) -> I2c<Self, SDA, SCL>
    where
        SDA: SDAPin<Self>,
        SCL: SCLPin<Self>;
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
    current_direction: I2cDirection,
}

pub enum I2cPeripheralEvent {
    Read(u8),
    Write(u8),
}

pub trait I2cPeripheral {
    type Error;

    fn poll(&mut self) -> Result<Option<I2cPeripheralEvent>, Self::Error>;
    fn rx(&mut self, buf: &mut [u8]) -> Result<(), Self::Error>;
    fn tx(&mut self, buf: &[u8]) -> Result<(), Self::Error>;
    fn flush(&mut self) -> Result<(), Self::Error>;
}
