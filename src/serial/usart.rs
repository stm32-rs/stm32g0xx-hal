use core::fmt;
use core::marker::PhantomData;

use crate::dma;
use crate::dmamux::DmaMuxIndex;
use crate::gpio::AltFunction;
use crate::gpio::{gpioa::*, gpiob::*, gpioc::*, gpiod::*};
use crate::prelude::*;
use crate::rcc::*;
use crate::stm32::*;

use cortex_m::interrupt;
use nb::block;

use crate::serial::config::*;
/// Serial error
#[derive(Debug)]
pub enum Error {
    /// Framing error
    Framing,
    /// Noise error
    Noise,
    /// RX buffer overrun
    Overrun,
    /// Parity check error
    Parity,
}

/// Interrupt event
pub enum Event {
    /// TXFIFO reaches the threshold
    TXFT = 1 << 27,
    /// This bit is set by hardware when the threshold programmed in RXFTCFG in USART_CR3 register is reached.
    RXFT = 1 << 26,

    /// RXFIFO full
    RXFF = 1 << 24,
    /// TXFIFO empty
    TXFE = 1 << 23,

    /// Active when a communication is ongoing on the RX line
    BUSY = 1 << 16,

    /// Receiver timeout.This bit is set by hardware when the timeout value,
    /// programmed in the RTOR register has lapsed, without any communication.
    RTOF = 1 << 11,
    /// Transmit data register empty. New data can be sent
    Txe = 1 << 7,

    /// Transmission Complete. The last data written in the USART_TDR has been transmitted out of the shift register.
    TC = 1 << 6,
    /// New data has been received
    Rxne = 1 << 5,
    /// Idle line state detected
    Idle = 1 << 4,

    /// Overrun error
    ORE = 1 << 3,

    /// Noise detection flag
    NE = 1 << 2,

    /// Framing error
    FE = 1 << 1,

    /// Parity error
    PE = 1 << 0,
}
impl Event {
    fn val(self) -> u32 {
        self as u32
    }
}

/// Serial receiver
pub struct Rx<USART, Config> {
    _usart: PhantomData<USART>,
    _config: PhantomData<Config>,
}

/// Serial transmitter
pub struct Tx<USART, Config> {
    _usart: PhantomData<USART>,
    _config: PhantomData<Config>,
}

/// Serial abstraction
pub struct Serial<USART, Config> {
    tx: Tx<USART, Config>,
    rx: Rx<USART, Config>,
    usart: USART,
    _config: PhantomData<Config>,
}

// Serial TX pin
pub trait TxPin<USART> {
    fn setup(&self);
}

// Serial RX pin
pub trait RxPin<USART> {
    fn setup(&self);
}

pub trait SerialExt<USART, Config> {
    fn usart<TX, RX>(
        self,
        tx: TX,
        rx: RX,
        config: Config,
        rcc: &mut Rcc,
    ) -> Result<Serial<USART, Config>, InvalidConfig>
    where
        TX: TxPin<USART>,
        RX: RxPin<USART>;
}

impl<USART, Config> fmt::Write for Serial<USART, Config>
where
    Serial<USART, Config>: hal::serial::Write<u8>,
{
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let _ = s.as_bytes().iter().map(|c| block!(self.write(*c))).last();
        Ok(())
    }
}

impl<USART, Config> fmt::Write for Tx<USART, Config>
where
    Tx<USART, Config>: hal::serial::Write<u8>,
{
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let _ = s.as_bytes().iter().map(|c| block!(self.write(*c))).last();
        Ok(())
    }
}

macro_rules! uart_shared {
    ($USARTX:ident, $dmamux_rx:ident, $dmamux_tx:ident,
        tx: [ $(($PTX:ident, $TAF:expr),)+ ],
        rx: [ $(($PRX:ident, $RAF:expr),)+ ]) => {

        $(
            impl<MODE> TxPin<$USARTX> for $PTX<MODE> {
                fn setup(&self) {
                    self.set_alt_mode($TAF)
                }
            }
        )+

        $(
            impl<MODE> RxPin<$USARTX> for $PRX<MODE> {
                fn setup(&self) {
                    self.set_alt_mode($RAF)
                }
            }
        )+

        impl<Config> Rx<$USARTX, Config> {
            pub fn listen(&mut self) {
                let usart = unsafe { &(*$USARTX::ptr()) };
                usart.cr1.modify(|_, w| w.rxneie().set_bit());
            }

            /// Stop listening for an interrupt event
            pub fn unlisten(&mut self) {
                let usart = unsafe { &(*$USARTX::ptr()) };
                usart.cr1.modify(|_, w| w.rxneie().clear_bit());
            }

            /// Return true if the rx register is not empty (and can be read)
            pub fn is_rxne(&self) -> bool {
                let usart = unsafe { &(*$USARTX::ptr()) };
                usart.isr.read().rxne().bit_is_set()
            }

        }

        impl<Config> hal::serial::Read<u8> for Rx<$USARTX, Config> {
            type Error = Error;

            fn read(&mut self) -> nb::Result<u8, Error> {
                let usart = unsafe { &(*$USARTX::ptr()) };
                let isr = usart.isr.read();
                Err(
                    if isr.pe().bit_is_set() {
                        usart.icr.write(|w| w.pecf().set_bit());
                        nb::Error::Other(Error::Parity)
                    } else if isr.fe().bit_is_set() {
                        usart.icr.write(|w| w.fecf().set_bit());
                        nb::Error::Other(Error::Framing)
                    } else if isr.nf().bit_is_set() {
                        usart.icr.write(|w| w.ncf().set_bit());
                        nb::Error::Other(Error::Noise)
                    } else if isr.ore().bit_is_set() {
                        usart.icr.write(|w| w.orecf().set_bit());
                        nb::Error::Other(Error::Overrun)
                    } else if isr.rxne().bit_is_set() {
                        return Ok(usart.rdr.read().bits() as u8)
                    } else {
                        nb::Error::WouldBlock
                    }
                )
            }
        }

        impl<Config> hal::serial::Read<u8> for Serial<$USARTX, Config> {
            type Error = Error;

            fn read(&mut self) -> nb::Result<u8, Error> {
                self.rx.read()
            }
        }

        impl<Config> Tx<$USARTX, Config> {

            /// Starts listening for an interrupt event
            pub fn listen(&mut self) {
                let usart = unsafe { &(*$USARTX::ptr()) };
                usart.cr1.modify(|_, w| w.txeie().set_bit());
            }

            /// Stop listening for an interrupt event
            pub fn unlisten(&mut self) {
                let usart = unsafe { &(*$USARTX::ptr()) };
                usart.cr1.modify(|_, w| w.txeie().clear_bit());
            }

            /// Return true if the tx register is empty (and can accept data)
            pub fn is_txe(&self) -> bool {
                let usart = unsafe { &(*$USARTX::ptr()) };
                usart.isr.read().txe().bit_is_set()
            }

        }

        impl<Config> hal::serial::Write<u8> for Tx<$USARTX, Config> {
            type Error = Error;

            fn flush(&mut self) -> nb::Result<(), Self::Error> {
                let usart = unsafe { &(*$USARTX::ptr()) };
                if usart.isr.read().tc().bit_is_set() {
                    Ok(())
                } else {
                    Err(nb::Error::WouldBlock)
                }
            }

            fn write(&mut self, byte: u8) -> nb::Result<(), Self::Error> {
                let usart = unsafe { &(*$USARTX::ptr()) };
                if usart.isr.read().txe().bit_is_set() {
                    usart.tdr.write(|w| unsafe { w.bits(byte as u32) });
                    Ok(())
                } else {
                    Err(nb::Error::WouldBlock)
                }
            }
        }

        impl<Config> hal::serial::Write<u8> for Serial<$USARTX, Config> {
            type Error = Error;

            fn flush(&mut self) -> nb::Result<(), Self::Error> {
                self.tx.flush()
            }

            fn write(&mut self, byte: u8) -> nb::Result<(), Self::Error> {
                self.tx.write(byte)
            }
        }


        impl<Config> Serial<$USARTX, Config> {

            /// Separates the serial struct into separate channel objects for sending (Tx) and
            /// receiving (Rx)
            pub fn split(self) -> (Tx<$USARTX, Config>, Rx<$USARTX, Config>) {
                (self.tx, self.rx)
            }

        }

        impl<Config> dma::Target for Rx<$USARTX, Config> {

            fn dmamux(&self) -> DmaMuxIndex {
                DmaMuxIndex::$dmamux_rx
            }

            fn enable_dma(&mut self) {
                // NOTE(unsafe) critical section prevents races
                interrupt::free(|_| unsafe {
                    let cr3 = &(*$USARTX::ptr()).cr3;
                    cr3.modify(|_, w| w.dmar().set_bit());
                });
            }

            fn disable_dma(&mut self) {
                // NOTE(unsafe) critical section prevents races
                interrupt::free(|_| unsafe {
                    let cr3 = &(*$USARTX::ptr()).cr3;
                    cr3.modify(|_, w| w.dmar().clear_bit());
                });
            }
        }

        impl<Config> dma::Target for Tx<$USARTX, Config> {

            fn dmamux(&self) -> DmaMuxIndex {
                DmaMuxIndex::$dmamux_tx
            }

            fn enable_dma(&mut self) {
                // NOTE(unsafe) critical section prevents races
                interrupt::free(|_| unsafe {
                    let cr3 = &(*$USARTX::ptr()).cr3;
                    cr3.modify(|_, w| w.dmat().set_bit());
                });
            }

            fn disable_dma(&mut self) {
                // NOTE(unsafe) critical section prevents races
                interrupt::free(|_| unsafe {
                    let cr3 = &(*$USARTX::ptr()).cr3;
                    cr3.modify(|_, w| w.dmat().clear_bit());
                });
            }
        }
    }
}

macro_rules! uart_basic {
    ($USARTX:ident,
        $usartX:ident, $clk_mul:expr
    ) => {
        impl SerialExt<$USARTX, BasicConfig> for $USARTX {
            fn usart<TX, RX>(
                self,
                tx: TX,
                rx: RX,
                config: BasicConfig,
                rcc: &mut Rcc,
            ) -> Result<Serial<$USARTX, BasicConfig>, InvalidConfig>
            where
                TX: TxPin<$USARTX>,
                RX: RxPin<$USARTX>,
            {
                Serial::$usartX(self, tx, rx, config, rcc)
            }
        }

        impl Serial<$USARTX, BasicConfig> {
            pub fn $usartX<TX, RX>(
                usart: $USARTX,
                tx: TX,
                rx: RX,
                config: BasicConfig,
                rcc: &mut Rcc,
            ) -> Result<Self, InvalidConfig>
            where
                TX: TxPin<$USARTX>,
                RX: RxPin<$USARTX>,
            {
                // Enable clock for USART
                $USARTX::enable(rcc);

                let clk = rcc.clocks.apb_clk.0 as u64;
                let bdr = config.baudrate.0 as u64;
                let div = ($clk_mul * clk) / bdr;
                usart.brr.write(|w| unsafe { w.bits(div as u32) });
                // Reset other registers to disable advanced USART features
                usart.cr2.reset();
                usart.cr3.reset();

                // Disable USART, there are many bits where UE=0 is required
                usart.cr1.modify(|_, w| w.ue().clear_bit());

                // Enable transmission and receiving
                usart.cr1.write(|w| {
                    w.te()
                        .set_bit()
                        .re()
                        .set_bit()
                        .m0()
                        .bit(config.wordlength == WordLength::DataBits9)
                        .m1()
                        .bit(config.wordlength == WordLength::DataBits7)
                        .pce()
                        .bit(config.parity != Parity::ParityNone)
                        .ps()
                        .bit(config.parity == Parity::ParityOdd)
                });
                usart.cr2.write(|w| unsafe {
                    w.stop()
                        .bits(match config.stopbits {
                            StopBits::STOP1 => 0b00,
                            StopBits::STOP0P5 => 0b01,
                            StopBits::STOP2 => 0b10,
                            StopBits::STOP1P5 => 0b11,
                        })
                        .swap()
                        .bit(config.swap)
                });

                // Enable pins
                tx.setup();
                rx.setup();

                // Enable USART
                usart.cr1.modify(|_, w| w.ue().set_bit());

                Ok(Serial {
                    tx: Tx {
                        _usart: PhantomData,
                        _config: PhantomData,
                    },
                    rx: Rx {
                        _usart: PhantomData,
                        _config: PhantomData,
                    },
                    usart,
                    _config: PhantomData,
                })
            }

            /// Starts listening for an interrupt event
            pub fn listen(&mut self, event: Event) {
                match event {
                    Event::Rxne => self.usart.cr1.modify(|_, w| w.rxneie().set_bit()),
                    Event::Txe => self.usart.cr1.modify(|_, w| w.txeie().set_bit()),
                    Event::Idle => self.usart.cr1.modify(|_, w| w.idleie().set_bit()),
                    _ => {}
                }
            }

            /// Stop listening for an interrupt event
            pub fn unlisten(&mut self, event: Event) {
                match event {
                    Event::Rxne => self.usart.cr1.modify(|_, w| w.rxneie().clear_bit()),
                    Event::Txe => self.usart.cr1.modify(|_, w| w.txeie().clear_bit()),
                    Event::Idle => self.usart.cr1.modify(|_, w| w.idleie().clear_bit()),
                    _ => {}
                }
            }

            /// Check if interrupt event is pending
            pub fn is_pending(&mut self, event: Event) -> bool {
                (self.usart.isr.read().bits() & event.val()) != 0
            }

            /// Clear pending interrupt
            pub fn unpend(&mut self, event: Event) {
                // mask the allowed bits
                let mask: u32 = 0x123BFF;
                self.usart
                    .icr
                    .write(|w| unsafe { w.bits(event.val() & mask) });
            }
        }
    };
}

macro_rules! uart_full {
    ($USARTX:ident,
        $usartX:ident, $clk_mul:expr
    ) => {
        impl SerialExt<$USARTX, FullConfig> for $USARTX {
            fn usart<TX, RX>(
                self,
                tx: TX,
                rx: RX,
                config: FullConfig,
                rcc: &mut Rcc,
            ) -> Result<Serial<$USARTX, FullConfig>, InvalidConfig>
            where
                TX: TxPin<$USARTX>,
                RX: RxPin<$USARTX>,
            {
                Serial::$usartX(self, tx, rx, config, rcc)
            }
        }

        impl Serial<$USARTX, FullConfig> {
            pub fn $usartX<TX, RX>(
                usart: $USARTX,
                tx: TX,
                rx: RX,
                config: FullConfig,
                rcc: &mut Rcc,
            ) -> Result<Self, InvalidConfig>
            where
                TX: TxPin<$USARTX>,
                RX: RxPin<$USARTX>,
            {
                // Enable clock for USART
                $USARTX::enable(rcc);

                let clk = rcc.clocks.apb_clk.0 as u64;
                let bdr = config.baudrate.0 as u64;
                let clk_mul = 1;
                let div = (clk_mul * clk) / bdr;
                usart.brr.write(|w| unsafe { w.bits(div as u32) });

                usart.cr1.reset();
                usart.cr2.reset();
                usart.cr3.reset();

                usart.cr2.write(|w| unsafe {
                    w.stop()
                        .bits(config.stopbits.bits())
                        .swap()
                        .bit(config.swap)
                });

                if let Some(timeout) = config.receiver_timeout {
                    usart.cr1.write(|w| w.rtoie().set_bit());
                    usart.cr2.modify(|_, w| w.rtoen().set_bit());
                    usart.rtor.write(|w| unsafe { w.rto().bits(timeout) });
                }

                usart.cr3.write(|w| unsafe {
                    w.txftcfg()
                        .bits(config.tx_fifo_threshold.bits())
                        .rxftcfg()
                        .bits(config.rx_fifo_threshold.bits())
                        .txftie()
                        .bit(config.tx_fifo_interrupt)
                        .rxftie()
                        .bit(config.rx_fifo_interrupt)
                });

                usart.cr1.modify(|_, w| {
                    w.ue()
                        .set_bit()
                        .te()
                        .set_bit()
                        .re()
                        .set_bit()
                        .m0()
                        .bit(config.wordlength == WordLength::DataBits7)
                        .m1()
                        .bit(config.wordlength == WordLength::DataBits9)
                        .pce()
                        .bit(config.parity != Parity::ParityNone)
                        .ps()
                        .bit(config.parity == Parity::ParityOdd)
                        .fifoen()
                        .bit(config.fifo_enable)
                });

                tx.setup();
                rx.setup();

                Ok(Serial {
                    tx: Tx {
                        _usart: PhantomData,
                        _config: PhantomData,
                    },
                    rx: Rx {
                        _usart: PhantomData,
                        _config: PhantomData,
                    },
                    usart,
                    _config: PhantomData,
                })
            }

            /// Starts listening for an interrupt event
            pub fn listen(&mut self, event: Event) {
                match event {
                    Event::Rxne => self.usart.cr1.modify(|_, w| w.rxneie().set_bit()),
                    Event::Txe => self.usart.cr1.modify(|_, w| w.txeie().set_bit()),
                    Event::Idle => self.usart.cr1.modify(|_, w| w.idleie().set_bit()),
                    _ => {}
                }
            }

            /// Stop listening for an interrupt event
            pub fn unlisten(&mut self, event: Event) {
                match event {
                    Event::Rxne => self.usart.cr1.modify(|_, w| w.rxneie().clear_bit()),
                    Event::Txe => self.usart.cr1.modify(|_, w| w.txeie().clear_bit()),
                    Event::Idle => self.usart.cr1.modify(|_, w| w.idleie().clear_bit()),
                    _ => {}
                }
            }

            /// Check if interrupt event is pending
            pub fn is_pending(&mut self, event: Event) -> bool {
                (self.usart.isr.read().bits() & event.val()) != 0
            }

            /// Clear pending interrupt
            pub fn unpend(&mut self, event: Event) {
                // mask the allowed bits
                let mask: u32 = 0x123BFF;
                self.usart
                    .icr
                    .write(|w| unsafe { w.bits(event.val() & mask) });
            }
        }
        impl Tx<$USARTX, FullConfig> {
            /// Returns true if the tx fifo threshold has been reached.
            pub fn fifo_threshold_reached(&self) -> bool {
                let usart = unsafe { &(*$USARTX::ptr()) };
                usart.isr.read().txft().bit_is_set()
            }
        }

        impl Rx<$USARTX, FullConfig> {
            /// Check if receiver timeout has lapsed
            /// Returns the current state of the ISR RTOF bit
            pub fn timeout_lapsed(&self) -> bool {
                let usart = unsafe { &(*$USARTX::ptr()) };
                usart.isr.read().rtof().bit_is_set()
            }

            /// Clear pending receiver timeout interrupt
            pub fn clear_timeout(&mut self) {
                let usart = unsafe { &(*$USARTX::ptr()) };
                usart.icr.write(|w| w.rtocf().set_bit());
            }

            /// Returns true if the rx fifo threshold has been reached.
            pub fn fifo_threshold_reached(&self) -> bool {
                let usart = unsafe { &(*$USARTX::ptr()) };
                usart.isr.read().rxft().bit_is_set()
            }
        }
    };
}

uart_shared!(USART1, USART1_RX, USART1_TX,
tx: [
    (PA9, AltFunction::AF1),
    (PB6, AltFunction::AF0),
    (PC4, AltFunction::AF1),
],
rx: [
    (PA10, AltFunction::AF1),
    (PB7, AltFunction::AF0),
    (PC5, AltFunction::AF1),
]);

uart_shared!(USART2, USART2_RX, USART2_TX,
    tx: [
        (PA2, AltFunction::AF1),
        (PA14, AltFunction::AF1),
        (PD5, AltFunction::AF0),
    ],
    rx: [
        (PA3, AltFunction::AF1),
        (PA15, AltFunction::AF1),
        (PD6, AltFunction::AF0),
    ]
);

#[cfg(any(feature = "stm32g070", feature = "stm32g071", feature = "stm32g081"))]
uart_shared!(USART3, USART3_RX, USART3_TX,
    tx: [
        (PA5, AltFunction::AF4),
        (PB2, AltFunction::AF4),
        (PB8, AltFunction::AF4),
        (PB10, AltFunction::AF4),
        (PC4, AltFunction::AF1),
        (PC10, AltFunction::AF1),
        (PD8, AltFunction::AF1),
    ],
    rx: [
        (PB0, AltFunction::AF4),
        (PB9, AltFunction::AF4),
        (PB11, AltFunction::AF4),
        (PC5, AltFunction::AF1),
        (PC11, AltFunction::AF1),
        (PD9, AltFunction::AF1),
    ]
);

#[cfg(any(feature = "stm32g070", feature = "stm32g071", feature = "stm32g081"))]
uart_shared!(USART4, USART4_RX, USART4_TX,
    tx: [
        (PA0, AltFunction::AF4),
        (PC10, AltFunction::AF1),
    ],
    rx: [
        (PC11, AltFunction::AF1),
        (PA1, AltFunction::AF4),
    ]
);

#[cfg(feature = "stm32g0x1")]
uart_shared!(LPUART, LPUART_RX, LPUART_TX,
    tx: [
        (PA2, AltFunction::AF6),
        (PB11, AltFunction::AF1),
        (PC1, AltFunction::AF1),
    ],
    rx: [
        (PA3, AltFunction::AF6),
        (PB10, AltFunction::AF1),
        (PC0, AltFunction::AF1),
    ]
);

uart_full!(USART1, usart1, 1);

#[cfg(any(feature = "stm32g070", feature = "stm32g071", feature = "stm32g081"))]
uart_full!(USART2, usart2, 1);

#[cfg(any(feature = "stm32g030", feature = "stm32g031", feature = "stm32g041"))]
uart_basic!(USART2, usart2, 1);

#[cfg(any(feature = "stm32g070", feature = "stm32g071", feature = "stm32g081"))]
uart_basic!(USART3, usart3, 1);

#[cfg(any(feature = "stm32g070", feature = "stm32g071", feature = "stm32g081"))]
uart_basic!(USART4, usart4, 1);

// LPUART Should be given its own implementation when it needs to be used with features not present on
// the basic feature set such as: Dual clock domain, FIFO or prescaler.
// Or when Synchronous mode is implemented for the basic feature set, since the LP feature set does not have support.
#[cfg(feature = "stm32g0x1")]
uart_basic!(LPUART, lpuart, 256);
