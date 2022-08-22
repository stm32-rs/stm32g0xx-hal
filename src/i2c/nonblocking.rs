//! I2C
use crate::gpio::*;
use crate::gpio::{AltFunction, OpenDrain, Output};
use crate::i2c::config::Config;
use crate::i2c::{Error, I2c, I2cDirection, I2cExt, I2cResult, SCLPin, SDAPin};
use crate::rcc::*;
use crate::stm32::{I2C1, I2C2};
use nb::Error::{Other, WouldBlock};

pub trait I2cControl {
    /// Start listening for an interrupt event, will also enable non_blocking mode
    fn listen(&mut self);

    /// Stop listening for an interrupt event
    fn unlisten(&mut self);

    /// Check the isr flags.
    /// This funcion can be called inside the block! macro for blocking mode,
    /// or inside an I2C interrupt, in case the isr is enabled.
    /// Ignore the WouldBlock error in the i2c interrupt, as there will come
    /// yet another i2c interrrupt to handle the case.
    fn check_isr_flags(&mut self) -> nb::Result<I2cResult, Error>;

    /// get the global error counter. Reset to 0 after read
    fn get_errors_reset(&mut self) -> usize;

    /// optional function
    /// If used call this function once per 10th second. After 10 calls (after a second)
    /// i2c will be forcefully reset, if the watchdog counter is still greater than zero
    fn execute_watchdog(&mut self);
}

/// The trait I2c master and I2cSlave can operate in 3 modes:
///
/// Each function will first check the status of the bus. If busy it will return BusyWait
/// Wrap the function in the block! macro to make it blocking
///  
/// The actual work is done in a separate function: check_isr_flags, see the I2cControl trait
/// Wrap this function in the block! macro to make it blocking
///
/// If interrupts are enabled with listen() the application should enable an i2c interrupt and call
/// function check_isr_flags in the interrupt context
///
pub trait I2cMaster {
    /// Send the bytes in the given data buffer to the bus. The data is copied to the internal buffer.
    fn master_write(&mut self, addr: u16, data: &[u8]) -> nb::Result<(), Error>;

    /// Send the bytes in the given data buffer to the bus. The data is copied to the internal buffer.
    /// After the first write did end succesfully, in the irq function the read is started
    fn master_write_read(&mut self, addr: u16, data: &[u8], read_len: u8) -> nb::Result<(), Error>;

    /// Receive bytes from the addressed slave. The data is copied into the internal buffer.
    /// If the bus is not idle the function will return with wouldblock,
    /// so call the function wrapped in the block! macro, to make it blocking.
    ///
    fn master_read(&mut self, addr: u16, length: u8) -> nb::Result<(), Error>;

    /// return the address of the addressed slave
    fn get_address(&self) -> u16;

    /// return a non mutable slice to the internal data, with the size of the last transaction
    fn get_data(&self) -> &[u8];
}

/// The MasterWriteSlaveRead  is fully under control of the master. The slave simply has to accept
/// the amount of bytes send by the master
/// The MasterReadSlaveWrite is onder control of the slave. The slave decides how many bytes to send
pub trait I2cSlave {
    /// Enable/ disable sbc. Default sbc is switched on.
    /// For master write/read the transaction should start with sbc disabled.
    /// So ACK will be send on the last received byte. Then before the send phase sbc should enabled again
    fn slave_sbc(&mut self, sbc_enabled: bool);

    /// Start writing the bytes, the master want to receive. If OK returned, all bytes are transferred
    /// If the master wants more data than bytes.len()  the master will run into a timeout, This function will return Ok(())
    /// If the master wants less data than bytes.len(), this function will return OK, but with the incorrect nr
    /// of bytes  in the I2cResult
    /// Note that this function must be called after a I2cResult::Addressed when MasterReadSlaveWrite
    /// otherwise the bus gets blocked.
    fn slave_write(&mut self, bytes: &[u8]) -> Result<(), Error>;

    /// return the address of the addressed slave
    fn get_address(&self) -> u16;

    /// return a non mutable slice to the internal data, with the size of the last transaction
    fn get_data(&self) -> &[u8];

    /// Set and enable the (7 bit) adress. To keep the interface generic, only slave address 1 can be set
    fn set_address(&mut self, address: u16);
}

/// Sequence to flush the RXDR register. This resets the TXIS and TXE flags
macro_rules! flush_rxdr {
    ($i2c:expr) => {
        if $i2c.isr.read().rxne().bit_is_set() {
            // flush
            let _ = $i2c.rxdr.read().rxdata().bits();
        };
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

                if config.slave_address_1 > 0 {
                    if config.address_11bits {
                        i2c.oar1.write(|w| unsafe {
                            let addr = config.slave_address_1;
                            w.oa1_0() .bit(addr&0x1  == 0x1)
                            .oa1_7_1().bits( ((addr >> 1)  & 0x7F )as u8)
                            .oa1_8_9().bits( ((addr >> 8)  & 0x3  )as u8)
                            .oa1mode().set_bit()
                            .oa1en().set_bit()
                        });
                    }else {
                        i2c.oar1.write(|w| unsafe {
                            w.oa1_7_1().bits(config.slave_address_1 as u8)
                            .oa1mode().clear_bit()
                            .oa1en().set_bit()
                        });
                    }
                    // Enable acknowlidge control
                    i2c.cr1.modify(|_, w|  w.sbc().set_bit() );
                }

                if config.slave_address_2 > 0 {
                    i2c.oar2.write( |w| unsafe {
                        w.oa2msk().bits(  config.slave_address_mask as u8)
                        .oa2().bits(config.slave_address_2)
                        .oa2en().set_bit()
                    });
                    // Enable acknowlidge control
                    i2c.cr1.modify(|_, w|  w.sbc().set_bit() );
                }

                // Enable pins
                sda.setup();
                scl.setup();
                I2c { i2c, sda, scl,
                    address:0,
                    watchdog:0,
                    index: 0,
                    length:0,
                    errors:0,
                    length_write_read:0,
                    data:[0_u8;255]
                }
            }
            pub fn release(self) -> ($I2CX, SDA, SCL) {
                (self.i2c, self.sda.release(), self.scl.release())
            }
        } // I2c

        impl<SDA, SCL> I2cControl for I2c<$I2CX, SDA, SCL> {
            /// Starts listening for an interrupt event
            fn listen(&mut self) {
                self.i2c.cr1.modify(|_, w|
                       w.txie().set_bit()
                        .addrie().set_bit()
                        .rxie().set_bit()
                        .nackie().set_bit()
                        .stopie().set_bit()
                        .errie().set_bit()
                        .tcie().set_bit()
                   );
            }

            /// Stop listening for an interrupt event
            fn unlisten(&mut self) {
                self.i2c.cr1.modify(|_, w|
                    w.txie().clear_bit()
                     .rxie().clear_bit()
                     .addrie().clear_bit()
                     .nackie().clear_bit()
                     .stopie().clear_bit()
                     .tcie().clear_bit()
                     .errie().clear_bit()
                );
            }

            /// get the global error counter. Reset to 0 after read
            fn get_errors_reset(&mut self) -> usize {
                let result = self.errors;
                self.errors = 0;
                result
            }


            /// optional function
            /// If used call this function once per 10th second. After 10 calls (after a second)
            /// i2c will be forcefully reset, if the watchdog counter is still greater than zero
            fn execute_watchdog(&mut self) {
                match self.watchdog {
                    0 => return,
                    1 => {
                        self.errors += 1;
                        self.watchdog = 0;
                        // Disable I2C processing, resetting all hardware state machines
                        self.i2c.cr1.modify(|_, w| unsafe {w.pe().clear_bit() } );
                        // force enough wait states for the pe clear
                        let _ = self.i2c.cr1.read();
                        // Enable the I2C processing again
                        self.i2c.cr1.modify(|_, w| unsafe {w.pe().set_bit() });
                    },
                    _ => {self.watchdog -= 1},
                }
            }

            /// Check the isr flags. If the transaction still is not finished
            /// This funcion can be called inside the block! macro for blocking mode,
            /// or inside an I2C interrupt, in case the isr is enalbed
            fn check_isr_flags(&mut self) -> nb::Result< I2cResult, Error>{
                let isr = self.i2c.isr.read();

                if isr.berr().bit_is_set() {
                    self.i2c.icr.write(|w| w.berrcf().set_bit());
                    self.errors += 1;
                    return Err( Other(Error::BusError))
                } else
                if isr.arlo().bit_is_set() {
                    self.i2c.icr.write(|w| w.arlocf().set_bit());
                    return Err( Other(Error::ArbitrationLost))
                }else
                if isr.nackf().bit_is_set() {
                    self.i2c.icr.write(|w| w.nackcf().set_bit());
                    // Make one extra loop to wait on the stop condition
                    return Err( WouldBlock)
                } else
                if isr.txis().bit_is_set() {
                    // Put byte on the wire
                    if self.index < self.length {
                        self.i2c.txdr.write(|w| unsafe { w.txdata().bits(self.data[self.index]) });
                        self.index += 1; // ok previous byte is send now
                    }
                    return Err( WouldBlock)
                } else
                if isr.rxne().bit_is_set() {
                    // read byte from the wire
                    if self.index < self.length {
                        self.data[self.index] = self.i2c.rxdr.read().rxdata().bits();
                        self.index += 1;
                    }else {
                        // anyway read the result to clear the rxne flag
                        flush_rxdr!(self.i2c);
                    }
                    return Err( WouldBlock)
                } else
                if isr.stopf().bit_is_set() {
                    // Clear the stop condition flag
                    self.i2c.icr.write(|w| w.stopcf().set_bit());
                    // Disable the watchdog
                    self.watchdog = 0;
                    if self.index == 0 {
                        self.errors += 1;
                        return Err( Other(Error::Nack))
                    } else
                    {
                        // figure out the direction
                        let direction = if isr.dir().bit_is_set()
                            {
                                I2cDirection::MasterReadSlaveWrite
                            }  else  {
                                I2cDirection::MasterWriteSlaveRead
                            };
                        // return the actual amount of data (self.index), not the requested (self.length)
                        // application must evaluate the size of the frame
                        return Ok( I2cResult::Data(self.address, direction,  &self.data[0..self.index]) )
                    }
                }else
                if isr.tc().bit_is_set() {
                    // This condition Will only happen when autoend is 0 in master mode (write with subb addressing)
                    // Flag is reset by a start or stop condition.
                    // no stop condition will be generated in this transaction so evaluate the result here
                    if self.index < self.length {
                        self.index += 1; // ok previous byte is send now
                    }
                    if self.index == self.length {
                        // ok start the second part of the transaction
                        // reSTART and prepare to receive bytes into `rcv_buffer`
                        self.length = self.length_write_read;
                        self.length_write_read = 0;
                        self.index = 0;
                        self.i2c.cr2.write(|w| unsafe {
                            w
                                // Set number of bytes to transfer
                                .nbytes().bits(self.length as u8)
                                // Set address to transfer to/from
                                .sadd().bits((self.address << 1) as u16)
                                // 7-bit addressing mode
                                .add10().clear_bit()
                                // Set transfer direction to read
                                .rd_wrn().set_bit()
                                // Automatic end mode
                                .autoend().set_bit()
                                .reload().clear_bit()
                                // Start transfer
                                .start().set_bit()
                        });
                        // not yet ready here
                        return Err( WouldBlock)
                    } else
                    if self.index == 0 {
                        self.i2c.cr2.modify(|_, w| unsafe {
                            w.stop().set_bit()
                        });
                        self.errors += 1;
                        return Err( Other(Error::Nack))
                    } else
                    {
                        self.i2c.cr2.modify(|_, w| unsafe {
                            w.stop().set_bit()
                        });
                        self.errors += 1;
                        return Err(Other(Error::IncorrectFrameSize(self.index)))
                    }
                } else
                if isr.tcr().bit_is_set() {
                    // This condition Will only happen when reload == 1 and sbr == 1 (slave) and nbytes was written.
                    // Send a NACK, set nbytes to clear tcr flag
                    self.i2c.cr2.modify(|_, w| unsafe {
                        w.nack().set_bit().nbytes().bits( 1 as u8)
                    });
                    // Make one extra loop here to wait on the stop condition
                    return Err( WouldBlock)

                } else
                if isr.addr().bit_is_set() {
                    // handle the slave device case, addressed by a master
                    let current_address = isr.addcode().bits() as u16;
                    self.address = current_address;
                    // guard against misbehavior
                    self.watchdog = 10;

                    // figure out the direction.
                    let direction = if isr.dir().bit_is_set()
                        {
                            I2cDirection::MasterReadSlaveWrite
                        }  else  {
                            // Start the master write slave read transaction fully automatically here
                            // Set the nbytes to the max size and prepare to receive bytes into `buffer`.
                            self.length = self.data.len();
                            self.index = 0;
                            self.i2c.cr2.modify(|_, w| unsafe {
                                // Set number of bytes to transfer: as many as internal buffer
                                w.nbytes().bits(self.length as u8)
                                // during sending nbytes automatically send a ACK, stretch clock after last byte
                                .reload().set_bit()
                            });
                            // end address phase, release clock stretching
                            self.i2c.icr.write(|w| w.addrcf().set_bit());
                            // return result
                            I2cDirection::MasterWriteSlaveRead
                        };

                    // do not yet release the clock stretching here
                    return Ok(I2cResult::Addressed(current_address, direction))
                }
                return Err( WouldBlock)
            } // check_isr_flags
        } // i2c

        impl<SDA, SCL> I2cMaster for I2c<$I2CX, SDA, SCL> {


            fn master_write(&mut self, addr: u16, data: &[u8]) -> nb::Result<(), Error>{
                // Check if the bus is free
                if self.i2c.cr2.read().start().bit_is_set() {
                    return Err(nb::Error::WouldBlock)
                };
                self.watchdog = 10;
                let buflen = data.len();
                assert!(buflen < 256 && buflen > 0);
                self.length = buflen;
                self.data[..buflen].copy_from_slice(data);
                self.index = 0;
                self.address = addr;
                self.length_write_read = 0;

                self.i2c.cr2.modify(|_, w| unsafe {
                    w
                        // Start transfer
                        .start().set_bit()
                        // Set number of bytes to transfer
                        .nbytes().bits(buflen as u8)
                        // Set address to transfer to/from
                        .sadd().bits((addr << 1) as u16)
                        // Set transfer direction to write
                        .rd_wrn().clear_bit()
                        // Automatic end mode
                        .autoend().bit(true)
                        .reload().clear_bit()
                });
                // in non-blocking mode the result is not yet available
                Ok (())
            }
            fn master_write_read(&mut self, addr: u16, data: &[u8], read_len:u8) -> nb::Result<(), Error>{
                // Check if the bus is free
                if self.i2c.cr2.read().start().bit_is_set() {
                    return Err(nb::Error::WouldBlock)
                };
                self.watchdog = 10;
                let buflen = data.len();
                assert!(buflen < 256 && buflen > 0);
                self.length = buflen;
                self.data[..buflen].copy_from_slice(data);
                self.index = 0;
                self.address = addr;
                self.length_write_read = read_len as usize;

                self.i2c.cr2.modify(|_, w| unsafe {
                    w
                        // Start transfer
                        .start().set_bit()
                        // Set number of bytes to transfer
                        .nbytes().bits(buflen as u8)
                        // Set address to transfer to/from
                        .sadd().bits((addr << 1) as u16)
                        // Set transfer direction to write
                        .rd_wrn().clear_bit()
                        // Automatic end mode
                        .autoend().bit(false)
                        .reload().clear_bit()
                });
                // in non-blocking mode the result is not yet available
                Ok (())
            }


            fn master_read(&mut self, addr: u16, length: u8) -> nb::Result<(), Error>{
                // Wait for any previous address sequence to end automatically.
                // This could be up to 50% of a bus cycle (ie. up to 0.5/freq)
                if self.i2c.cr2.read().start().bit_is_set() {
                    return Err(nb::Error::WouldBlock)
                };
                // Flush rxdr register
                self.watchdog = 10;
                self.i2c.rxdr.read().rxdata().bits();
                self.length = length as usize;
                self.index = 0;
                self.address = addr;

                for i  in 0.. length as usize {
                    self.data[i] = 0;
                }

                // Set START and prepare to receive bytes into `buffer`.
                // The START bit can be set even if the bus
                // is BUSY or I2C is in slave mode.
                self.i2c.cr2.modify(|_, w| unsafe {
                    w
                        // Start transfer
                        .start().set_bit()
                        // Set number of bytes to transfer
                        .nbytes().bits(length as u8)
                        // Set address to transfer to/from
                        .sadd().bits((addr << 1) as u16)
                        // Set transfer direction to read
                        .rd_wrn().set_bit()
                        // automatic end mode
                        .autoend().set_bit()
                        .reload().clear_bit()
                });
                // in non-blocking mode the result is not yet available
                Ok (())
            }

            fn get_address(&self) -> u16 {
                self.address
            }

            /// return a non mutable slice to the internal data, with the size of the last transaction
            fn get_data(&self) -> &[u8] {
                &self.data[0..self.length]
            }
        }

        impl<SDA, SCL> I2cSlave for I2c<$I2CX, SDA, SCL> {

            fn slave_sbc(&mut self, sbc_enabled: bool)  {
                // enable acknowlidge control
                self.i2c.cr1.modify(|_, w|  w.sbc().bit(sbc_enabled) );
            }

            fn set_address(&mut self, address:u16) {
                self.i2c.oar1.write(|w| unsafe {
                    w.oa1_7_1().bits(address as u8)
                    .oa1en().clear_bit()
                });
                // set the 7 bits address
                self.i2c.oar1.write(|w| unsafe {
                    w.oa1_7_1().bits(address as u8)
                    .oa1mode().clear_bit()
                    .oa1en().set_bit()
                });
            }

            fn slave_write(&mut self, bytes: &[u8]) -> Result<(), Error> {
                let buflen = bytes.len();
                // TODO support transfers of more than 255 bytes
                assert!(buflen < 256 && buflen > 0);

                self.length = buflen;
                self.data[..buflen].copy_from_slice(bytes);
                self.index = 0;

                // Set the nbytes and prepare to send bytes into `buffer`.
                self.i2c.cr2.modify(|_, w| unsafe {
                    w.nbytes().bits( buflen as u8)
                    .reload().clear_bit()
                });
                // flush i2c tx register
                self.i2c.isr.write(|w| w.txe().set_bit());
                // end address phase, release clock stretching
                self.i2c.icr.write(|w| w.addrcf().set_bit() );

                // in non-blocking mode the result is not yet available
                Ok (())
            }
            fn get_address(&self) -> u16 {
                self.address
            }
            /// return a non mutable slice to the internal data, with the size of the last transaction
            fn get_data(&self) -> &[u8] {
                &self.data[0..self.index]
            }
        }

    }
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
