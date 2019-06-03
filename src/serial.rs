use core::fmt;
use core::marker::PhantomData;
use core::ops;
use core::pin::Pin;

use as_slice::AsMutSlice;

use crate::dma::{ReadDma, Transfer, WriteDma};
use crate::gpio::{gpioa::*, gpiob::*, gpioc::*, gpiod::*};
use crate::gpio::{AltFunction, DefaultMode};
use crate::prelude::*;
use crate::rcc::Rcc;
use crate::stm32::{LPUART, USART1, USART2, USART3, USART4};
use crate::time::Bps;
use hal;
use nb::block;

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

#[derive(PartialEq, PartialOrd, Clone, Copy)]
pub enum WordLength {
    DataBits7,
    DataBits8,
    DataBits9,
}

#[derive(PartialEq, PartialOrd, Clone, Copy)]
pub enum Parity {
    ParityNone,
    ParityEven,
    ParityOdd,
}

/// Interrupt event
pub enum Event {
    /// New data has been received
    Rxne,
    /// New data can be sent
    Txe,
    /// Idle line state detected
    Idle,
}

pub enum StopBits {
    #[doc = "1 stop bit"]
    STOP1,
    #[doc = "0.5 stop bits"]
    STOP0P5,
    #[doc = "2 stop bits"]
    STOP2,
    #[doc = "1.5 stop bits"]
    STOP1P5,
}

pub struct Config {
    baudrate: Bps,
    wordlength: WordLength,
    parity: Parity,
    stopbits: StopBits,
}

impl Config {
    pub fn baudrate(mut self, baudrate: Bps) -> Self {
        self.baudrate = baudrate;
        self
    }

    pub fn parity_none(mut self) -> Self {
        self.parity = Parity::ParityNone;
        self
    }

    pub fn parity_even(mut self) -> Self {
        self.parity = Parity::ParityEven;
        self
    }

    pub fn parity_odd(mut self) -> Self {
        self.parity = Parity::ParityOdd;
        self
    }

    pub fn wordlength_8(mut self) -> Self {
        self.wordlength = WordLength::DataBits8;
        self
    }

    pub fn wordlength_9(mut self) -> Self {
        self.wordlength = WordLength::DataBits9;
        self
    }

    pub fn stopbits(mut self, stopbits: StopBits) -> Self {
        self.stopbits = stopbits;
        self
    }
}

#[derive(Debug)]
pub struct InvalidConfig;

impl Default for Config {
    fn default() -> Config {
        let baudrate = 19_200.bps();
        Config {
            baudrate,
            wordlength: WordLength::DataBits8,
            parity: Parity::ParityNone,
            stopbits: StopBits::STOP1,
        }
    }
}

/// Serial receiver
pub struct Rx<USART> {
    _usart: PhantomData<USART>,
}

/// Serial transmitter
pub struct Tx<USART> {
    _usart: PhantomData<USART>,
}

/// Serial DMA receiver
pub struct DmaRx<USART, CHANNEL> {
    _usart: PhantomData<USART>,
    channel: CHANNEL,
}

/// Serial DMA transmitter
pub struct DmaTx<USART, CHANNEL> {
    _usart: PhantomData<USART>,
    channel: CHANNEL,
}

/// Serial abstraction
pub struct Serial<USART> {
    _tx: Tx<USART>,
    _rx: Rx<USART>,
}

pub trait SerialExt<USART> {
    fn usart<TX, RX>(
        self,
        tx: TX,
        rx: RX,
        config: Config,
        rcc: &mut Rcc,
    ) -> Result<Serial<USART>, InvalidConfig>
    where
        TX: TxPin<USART>,
        RX: RxPin<USART>;
}

// Serial TX pin
pub trait TxPin<USART> {
    fn setup(&self);
}

// Serial RX pin
pub trait RxPin<USART> {
    fn setup(&self);
}

impl<USART> fmt::Write for Serial<USART>
where
    Serial<USART>: hal::serial::Write<u8>,
{
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let _ = s
            .as_bytes()
            .into_iter()
            .map(|c| block!(self.write(*c)))
            .last();
        Ok(())
    }
}

macro_rules! uart {
    ($USARTX:ident,
        $usartX:ident, $apbXenr:ident, $usartXen:ident, $clk_mul:expr,
        tx: [ $(($PTX:ty, $TAF:expr),)+ ],
        rx: [ $(($PRX:ty, $RAF:expr),)+ ],
    ) => {
        $(
            impl TxPin<$USARTX> for $PTX {
                fn setup(&self) {
                    self.set_alt_mode($TAF)
                }
            }
        )+

        $(
            impl RxPin<$USARTX> for $PRX {
                fn setup(&self) {
                    self.set_alt_mode($RAF)
                }
            }
        )+

        impl SerialExt<$USARTX> for $USARTX {
            fn usart<TX, RX>(
                self,
                tx: TX,
                rx: RX,
                config: Config,
                rcc: &mut Rcc) -> Result<Serial<$USARTX>, InvalidConfig>
            where
                TX: TxPin<$USARTX>,
                RX: RxPin<$USARTX>,
            {
                Serial::$usartX(self, tx, rx, config, rcc)
            }
        }

        impl Serial<$USARTX> {
            pub fn $usartX<TX, RX>(
                usart: $USARTX,
                tx: TX,
                rx: RX,
                config: Config,
                rcc: &mut Rcc,
            ) -> Result<Self, InvalidConfig>
            where
                TX: TxPin<$USARTX>,
                RX: RxPin<$USARTX>,
            {
                tx.setup();
                rx.setup();

                // Enable clock for USART
                rcc.rb.$apbXenr.modify(|_, w| w.$usartXen().set_bit());
                let clk = rcc.clocks.apb_clk.0 as u64;
                let bdr = config.baudrate.0 as u64;
                let div = ($clk_mul * clk) / bdr;
                usart
                    .brr
                    .write(|w| unsafe { w.bits(div as u32) });
                // Reset other registers to disable advanced USART features
                usart.cr2.reset();
                usart.cr3.reset();
                // Enable transmission and receiving
                usart.cr1.write(|w| {
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
                });
                usart.cr2.write(|w| unsafe {
                    w.stop().bits(match config.stopbits {
                        StopBits::STOP1 => 0b00,
                        StopBits::STOP0P5 => 0b01,
                        StopBits::STOP2 => 0b10,
                        StopBits::STOP1P5 => 0b11,
                    })
                });
                Ok(Serial {
                    _tx: Tx { _usart: PhantomData },
                    _rx: Rx { _usart: PhantomData },
                })
            }

            /// Starts listening for an interrupt event
            pub fn listen(&mut self, event: Event) {
                let usart = unsafe { &(*$USARTX::ptr()) };

                match event {
                    Event::Rxne => usart.cr1.modify(|_, w| w.rxneie().set_bit()),
                    Event::Txe => usart.cr1.modify(|_, w| w.txeie().set_bit()),
                    Event::Idle => usart.cr1.modify(|_, w| w.idleie().set_bit()),
                }
            }

            /// Stop listening for an interrupt event
            pub fn unlisten(&mut self, event: Event) {
                let usart = unsafe { &(*$USARTX::ptr()) };

                match event {
                    Event::Rxne => usart.cr1.modify(|_, w| w.rxneie().clear_bit()),
                    Event::Txe => usart.cr1.modify(|_, w| w.txeie().clear_bit()),
                    Event::Idle => usart.cr1.modify(|_, w| w.idleie().clear_bit()),
                }
            }

            /// Separates the serial struct into separate channel objects for sending (Tx) and
            /// receiving (Rx)
            pub fn split(self) -> (Tx<$USARTX>, Rx<$USARTX>) {
                (self._tx, self._rx)
            }
        }

        impl Tx<$USARTX> {
            pub fn with_dma<CHANNEL>(self, channel: CHANNEL) -> DmaTx<$USARTX, CHANNEL> {
                DmaTx {
                    _usart: PhantomData,
                    channel,
                }
            }
        }

        impl Rx<$USARTX> {
            pub fn with_dma<CHANNEL>(self, channel: CHANNEL) -> DmaRx<$USARTX, CHANNEL> {
                DmaRx {
                    _usart: PhantomData,
                    channel,
                }
            }
        }

        impl<CHANNEL, B> ReadDma<B> for DmaRx<$USARTX, CHANNEL>
        where
            B: ops::DerefMut + 'static,
            B::Target: AsMutSlice<Element = u8> + Unpin,
            Self: core::marker::Sized,
        {
            fn read(self, buffer: Pin<B>) -> Transfer<Self, Pin<B>> {

                Transfer {
                    buffer,
                    channel: self,
                }
            }
        }

        impl<CHANNEL, B> WriteDma<B> for DmaRx<$USARTX, CHANNEL>
        where
            B: ops::DerefMut + 'static,
            B::Target: AsMutSlice<Element = u8> + Unpin,
            Self: core::marker::Sized,
        {
            fn write(self, buffer: Pin<B>) -> Transfer<Self, Pin<B>> {

                Transfer {
                    buffer,
                    channel: self,
                }
            }
        }

        impl hal::serial::Read<u8> for Tx<$USARTX> {
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

        impl hal::serial::Read<u8> for Serial<$USARTX> {
            type Error = Error;

            fn read(&mut self) -> nb::Result<u8, Error> {
                self._tx.read()
            }
        }

        impl hal::serial::Write<u8> for Rx<$USARTX> {
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

        impl hal::serial::Write<u8> for Serial<$USARTX> {
            type Error = Error;

            fn flush(&mut self) -> nb::Result<(), Self::Error> {
                self._rx.flush()
            }

            fn write(&mut self, byte: u8) -> nb::Result<(), Self::Error> {
                self._rx.write(byte)
            }
        }
    }
}

uart!(
    LPUART, lpuart, apbenr1, lpuart1en, 256,
    tx: [
        (PA2<DefaultMode>, AltFunction::AF6),
        (PB11<DefaultMode>, AltFunction::AF1),
        (PC1<DefaultMode>, AltFunction::AF1),
    ],
    rx: [
        (PA3<DefaultMode>, AltFunction::AF6),
        (PB10<DefaultMode>, AltFunction::AF1),
        (PC0<DefaultMode>, AltFunction::AF1),
    ],
);

uart!(
    USART1, usart1, apbenr2, usart1en, 1,
    tx: [
        (PA9<DefaultMode>, AltFunction::AF1),
        (PB6<DefaultMode>, AltFunction::AF0),
        (PC4<DefaultMode>, AltFunction::AF1),
    ],
    rx: [
        (PA10<DefaultMode>, AltFunction::AF1),
        (PB7<DefaultMode>, AltFunction::AF0),
        (PC5<DefaultMode>, AltFunction::AF1),
    ],
);

uart!(
    USART2, usart2, apbenr1, usart2en, 1,
    tx: [
        (PA2<DefaultMode>, AltFunction::AF1),
        (PA14<DefaultMode>, AltFunction::AF1),
        (PD5<DefaultMode>, AltFunction::AF0),
    ],
    rx: [
        (PA3<DefaultMode>, AltFunction::AF1),
        (PA15<DefaultMode>, AltFunction::AF1),
        (PD6<DefaultMode>, AltFunction::AF0),
    ],
);

uart!(
    USART3, usart3, apbenr1, usart3en, 1,
    tx: [
        (PA5<DefaultMode>, AltFunction::AF4),
        (PB2<DefaultMode>, AltFunction::AF4),
        (PB8<DefaultMode>, AltFunction::AF4),
        (PB10<DefaultMode>, AltFunction::AF4),
        (PC4<DefaultMode>, AltFunction::AF1),
        (PC10<DefaultMode>, AltFunction::AF1),
        (PD8<DefaultMode>, AltFunction::AF1),
    ],
    rx: [
        (PB0<DefaultMode>, AltFunction::AF4),
        (PB9<DefaultMode>, AltFunction::AF4),
        (PB11<DefaultMode>, AltFunction::AF4),
        (PC5<DefaultMode>, AltFunction::AF1),
        (PC11<DefaultMode>, AltFunction::AF1),
        (PD9<DefaultMode>, AltFunction::AF1),
    ],
);

uart!(
    USART4, usart4, apbenr1, usart4en, 1,
    tx: [
        (PA0<DefaultMode>, AltFunction::AF4),
        (PC10<DefaultMode>, AltFunction::AF1),
    ],
    rx: [
        (PC11<DefaultMode>, AltFunction::AF1),
        (PA1<DefaultMode>, AltFunction::AF4),
    ],
);
