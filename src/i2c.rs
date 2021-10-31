//! I2C
use hal::blocking::i2c::{Read, Write, WriteRead};

use crate::gpio::{gpioa::*, gpiob::*};
use crate::gpio::{AltFunction, OpenDrain, Output};
use crate::rcc::*;
use crate::stm32::{I2C1, I2C2};
use crate::time::Hertz;
use core::cmp;

pub struct Config {
    speed: Option<Hertz>,
    timing: Option<u32>,
    analog_filter: bool,
    digital_filter: u8,
}

impl Config {
    pub fn new<T>(speed: T) -> Self
    where
        T: Into<Hertz>,
    {
        Config {
            speed: Some(speed.into()),
            timing: None,
            analog_filter: true,
            digital_filter: 0,
        }
    }

    pub fn with_timing(timing: u32) -> Self {
        Config {
            timing: Some(timing),
            speed: None,
            analog_filter: true,
            digital_filter: 0,
        }
    }

    pub fn disable_analog_filter(mut self) -> Self {
        self.analog_filter = false;
        self
    }

    pub fn enable_digital_filter(mut self, cycles: u8) -> Self {
        assert!(cycles <= 16);
        self.digital_filter = cycles;
        self
    }

    fn timing_bits(&self, i2c_clk: Hertz) -> u32 {
        if let Some(bits) = self.timing {
            return bits;
        }
        let speed = self.speed.unwrap();
        let (psc, scll, sclh, sdadel, scldel) = if speed.0 <= 100_000 {
            let psc = 3;
            let scll = cmp::min((((i2c_clk.0 >> 1) / (psc + 1)) / speed.0) - 1, 255);
            let sclh = scll - 4;
            let sdadel = 2;
            let scldel = 4;
            (psc, scll, sclh, sdadel, scldel)
        } else {
            let psc = 1;
            let scll = cmp::min((((i2c_clk.0 >> 1) / (psc + 1)) / speed.0) - 1, 255);
            let sclh = scll - 6;
            let sdadel = 1;
            let scldel = 3;
            (psc, scll, sclh, sdadel, scldel)
        };
        psc << 28 | scldel << 20 | sdadel << 16 | sclh << 8 | scll
    }
}

impl<F> From<F> for Config
where
    F: Into<Hertz>,
{
    fn from(speed: F) -> Self {
        Config::new(speed)
    }
}

/// I2C abstraction
pub struct I2c<I2C, SDA, SCL> {
    i2c: I2C,
    sda: SDA,
    scl: SCL,
}

// I2C SDA pin
pub trait SDAPin<I2C> {
    fn setup(&self);
    fn release(self) -> Self;
}

// I2C SCL pin
pub trait SCLPin<I2C> {
    fn setup(&self);
    fn release(self) -> Self;
}

// I2C error
#[derive(Debug)]
pub enum Error {
    Overrun,
    Nack,
    PECError,
    BusError,
    ArbitrationLost,
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

// Sequence to flush the TXDR register. This resets the TXIS and TXE flags
macro_rules! flush_txdr {
    ($i2c:expr) => {
        // If a pending TXIS flag is set, write dummy data to TXDR
        if $i2c.isr.read().txis().bit_is_set() {
            $i2c.txdr.write(|w| unsafe { w.txdata().bits(0) });
        }

        // If TXDR is not flagged as empty, write 1 to flush it
        if $i2c.isr.read().txe().bit_is_set() {
            $i2c.isr.write(|w| w.txe().set_bit());
        }
    };
}

macro_rules! busy_wait {
    ($i2c:expr, $flag:ident, $variant:ident) => {
        loop {
            let isr = $i2c.isr.read();

            if isr.$flag().$variant() {
                break;
            } else if isr.berr().bit_is_set() {
                $i2c.icr.write(|w| w.berrcf().set_bit());
                return Err(Error::BusError);
            } else if isr.arlo().bit_is_set() {
                $i2c.icr.write(|w| w.arlocf().set_bit());
                return Err(Error::ArbitrationLost);
            } else if isr.nackf().bit_is_set() {
                $i2c.icr.write(|w| w.stopcf().set_bit().nackcf().set_bit());
                flush_txdr!($i2c);
                return Err(Error::Nack);
            } else {
                // try again
            }
        }
    };
}

macro_rules! i2c {
    ($I2CX:ident, $i2cx:ident,
        sda: [ $($PSDA:ty,)+ ],
        scl: [ $($PSCL:ty,)+ ],
    ) => {
        $(
            impl SDAPin<$I2CX> for $PSDA {
                fn setup(&self) {
                    self.set_alt_mode(AltFunction::AF6)
                }

                fn release(self) -> Self {
                    self.into_open_drain_output()
                }
            }
        )+

        $(
            impl SCLPin<$I2CX> for $PSCL {
                fn setup(&self) {
                    self.set_alt_mode(AltFunction::AF6)
                }

                fn release(self) -> Self {
                    self.into_open_drain_output()
                }
            }
        )+

        impl I2cExt<$I2CX> for $I2CX {
            fn i2c<SDA, SCL>(
                self,
                sda: SDA,
                scl: SCL,
                config: impl Into<Config>,
                rcc: &mut Rcc,
            ) -> I2c<$I2CX, SDA, SCL>
            where
                SDA: SDAPin<$I2CX>,
                SCL: SCLPin<$I2CX>,
            {
                I2c::$i2cx(self, sda, scl, config, rcc)
            }
        }

        impl<SDA, SCL> I2c<$I2CX, SDA, SCL> where
            SDA: SDAPin<$I2CX>,
            SCL: SCLPin<$I2CX>
        {
            pub fn $i2cx(i2c: $I2CX, sda: SDA, scl: SCL, config: impl Into<Config>, rcc: &mut Rcc) -> Self
            where
                SDA: SDAPin<$I2CX>,
                SCL: SCLPin<$I2CX>,
            {
                let config = config.into();
                $I2CX::enable(rcc);
                $I2CX::reset(rcc);

                // Make sure the I2C unit is disabled so we can configure it
                i2c.cr1.modify(|_, w| w.pe().clear_bit());

                // Setup protocol timings
                let timing_bits = config.timing_bits(rcc.clocks.apb_clk);
                i2c.timingr.write(|w| unsafe { w.bits(timing_bits) });

                // Enable the I2C processing
                i2c.cr1.modify(|_, w| unsafe {
                    w.pe()
                        .set_bit()
                        .dnf()
                        .bits(config.digital_filter)
                        .anfoff()
                        .bit(!config.analog_filter)
                });

                // Enable pins
                sda.setup();
                scl.setup();

                I2c { i2c, sda, scl }
            }

            pub fn release(self) -> ($I2CX, SDA, SCL) {
                (self.i2c, self.sda.release(), self.scl.release())
            }
        }

        impl<SDA, SCL> WriteRead for I2c<$I2CX, SDA, SCL> {
            type Error = Error;

            fn write_read(
                &mut self,
                addr: u8,
                bytes: &[u8],
                buffer: &mut [u8],
            ) -> Result<(), Self::Error> {
                // TODO support transfers of more than 255 bytes
                assert!(bytes.len() < 256 && bytes.len() > 0);
                assert!(buffer.len() < 256 && buffer.len() > 0);

                // Wait for any previous address sequence to end automatically.
                // This could be up to 50% of a bus cycle (ie. up to 0.5/freq)
                while self.i2c.cr2.read().start().bit_is_set() {};

                // Set START and prepare to send `bytes`.
                // The START bit can be set even if the bus is BUSY or
                // I2C is in slave mode.
                self.i2c.cr2.write(|w| unsafe {
                    w
                        // Start transfer
                        .start().set_bit()
                        // Set number of bytes to transfer
                        .nbytes().bits(bytes.len() as u8)
                        // Set address to transfer to/from
                        .sadd().bits((addr << 1) as u16)
                        // 7-bit addressing mode
                        .add10().clear_bit()
                        // Set transfer direction to write
                        .rd_wrn().clear_bit()
                        // Software end mode
                        .autoend().clear_bit()
                });

                for byte in bytes {
                    // Wait until we are allowed to send data
                    // (START has been ACKed or last byte went through)
                    busy_wait!(self.i2c, txis, bit_is_set);

                    // Put byte on the wire
                    self.i2c.txdr.write(|w| unsafe { w.txdata().bits(*byte) });
                }

                // Wait until the write finishes before beginning to read.
                busy_wait!(self.i2c, tc, bit_is_set);

                // reSTART and prepare to receive bytes into `buffer`
                self.i2c.cr2.write(|w| unsafe {
                    w
                        // Start transfer
                        .start().set_bit()
                        // Set number of bytes to transfer
                        .nbytes().bits(buffer.len() as u8)
                        // Set address to transfer to/from
                        .sadd().bits((addr << 1) as u16)
                        // 7-bit addressing mode
                        .add10().clear_bit()
                        // Set transfer direction to read
                        .rd_wrn().set_bit()
                        // Automatic end mode
                        .autoend().set_bit()
                });

                for byte in buffer {
                    // Wait until we have received something
                    busy_wait!(self.i2c, rxne, bit_is_set);

                    *byte = self.i2c.rxdr.read().rxdata().bits();
                }

                // Wait for STOP
                busy_wait!(self.i2c, stopf, bit_is_set);
                self.i2c.icr.write(|w| w.stopcf().set_bit());

                Ok(())
            }
        }

        impl<SDA, SCL> Write for I2c<$I2CX, SDA, SCL> {
            type Error = Error;

            fn write(&mut self, addr: u8, bytes: &[u8]) -> Result<(), Self::Error> {
                assert!(bytes.len() < 256 && bytes.len() > 0);

                self.i2c.cr2.modify(|_, w| unsafe {
                    w
                        // Start transfer
                        .start().set_bit()
                        // Set number of bytes to transfer
                        .nbytes().bits(bytes.len() as u8)
                        // Set address to transfer to/from
                        .sadd().bits((addr << 1) as u16)
                        // Set transfer direction to write
                        .rd_wrn().clear_bit()
                        // Automatic end mode
                        .autoend().set_bit()
                });

                for byte in bytes {
                    // Wait until we are allowed to send data
                    // (START has been ACKed or last byte when through)
                    busy_wait!(self.i2c, txis, bit_is_set);

                    // Put byte on the wire
                    self.i2c.txdr.write(|w| unsafe { w.txdata().bits(*byte) });
                }

                // Wait for STOP
                busy_wait!(self.i2c, stopf, bit_is_set);
                self.i2c.icr.write(|w| w.stopcf().set_bit());

                Ok(())
            }
        }

        impl<SDA, SCL> Read for I2c<$I2CX, SDA, SCL> {
            type Error = Error;

            fn read(&mut self, addr: u8, bytes: &mut [u8]) -> Result<(), Self::Error> {
                // TODO support transfers of more than 255 bytes
                assert!(bytes.len() < 256 && bytes.len() > 0);

                // Wait for any previous address sequence to end automatically.
                // This could be up to 50% of a bus cycle (ie. up to 0.5/freq)
                while self.i2c.cr2.read().start().bit_is_set() {};

                // Set START and prepare to receive bytes into `buffer`.
                // The START bit can be set even if the bus
                // is BUSY or I2C is in slave mode.
                self.i2c.cr2.modify(|_, w| unsafe {
                    w
                        // Start transfer
                        .start().set_bit()
                        // Set number of bytes to transfer
                        .nbytes().bits(bytes.len() as u8)
                        // Set address to transfer to/from
                        .sadd().bits((addr << 1) as u16)
                        // Set transfer direction to read
                        .rd_wrn().set_bit()
                        // automatic end mode
                        .autoend().set_bit()
                });

                for byte in bytes {
                    // Wait until we have received something
                    busy_wait!(self.i2c, rxne, bit_is_set);

                    *byte = self.i2c.rxdr.read().rxdata().bits();
                }

                // Wait for STOP
                busy_wait!(self.i2c, stopf, bit_is_set);
                self.i2c.icr.write(|w| w.stopcf().set_bit());

                Ok(())
            }
        }
    };
}

i2c!(
    I2C1,
    i2c1,
    sda: [
        PA10<Output<OpenDrain>>,
        PB7<Output<OpenDrain>>,
        PB9<Output<OpenDrain>>,
    ],
    scl: [
        PA9<Output<OpenDrain>>,
        PB6<Output<OpenDrain>>,
        PB8<Output<OpenDrain>>,
    ],
);

i2c!(
    I2C2,
    i2c2,
    sda: [
        PA12<Output<OpenDrain>>,
        PB11<Output<OpenDrain>>,
        PB14<Output<OpenDrain>>,
    ],
    scl: [
        PA11<Output<OpenDrain>>,
        PB10<Output<OpenDrain>>,
        PB13<Output<OpenDrain>>,
    ],
);
