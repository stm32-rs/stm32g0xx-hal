//! Direct Memory Access Engine
use crate::rcc::Rcc;
use crate::stm32::DMA;
use as_slice::{AsMutSlice, AsSlice};
use core::ops;
use core::pin::Pin;
use core::sync::atomic::{self, Ordering};

#[derive(Debug)]
pub enum Error {
    Overrun,
    BufferError,
}

#[derive(Debug)]
pub enum Event {
    HalfTransfer,
    TransferComplete,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Half {
    First,
    Second,
}

#[derive(Clone, Copy, PartialEq)]
pub enum TransferDirection {
    MemoryToMemory,
    MemoryToPeriph,
    PeriphToMemory,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Priority {
    Low = 0b00,
    Medium = 0b01,
    High = 0b10,
    VeryHigh = 0b11,
}

pub struct Transfer<CHANNEL, BUFFER> {
    pub(crate) channel: CHANNEL,
    pub(crate) buffer: BUFFER,
}

pub trait ReadDma<B>
where
    B: ops::DerefMut + 'static,
    B::Target: AsMutSlice<Element = u8> + Unpin,
    Self: core::marker::Sized,
{
    /// Receives data into the given `buffer` until it's filled
    ///
    /// Returns a value that represents the in-progress DMA transfer
    fn read(self, buffer: Pin<B>) -> Transfer<Self, Pin<B>>;
}

pub trait WriteDma<B>
where
    B: ops::Deref + 'static,
    B::Target: AsSlice<Element = u8>,
    Self: core::marker::Sized,
{
    /// Sends out the given `buffer`
    ///
    /// Returns a value that represents the in-progress DMA transfer
    fn write(self, buffer: Pin<B>) -> Transfer<Self, Pin<B>>;
}

pub trait CopyDma<F, T>
where
    F: ops::Deref + 'static,
    F::Target: AsSlice<Element = u8>,
    T: ops::Deref + 'static,
    T::Target: AsMutSlice<Element = u8> + Unpin,
    Self: core::marker::Sized,
{
    /// Copy data between bufferss
    ///
    /// Returns a value that represents the in-progress DMA transfer
    fn copy(self, from: Pin<F>, to: Pin<T>) -> Transfer<Self, (Pin<F>, Pin<T>)>;
}

pub trait DmaExt {
    type Channels;

    fn split(self, rcc: &mut Rcc) -> Self::Channels;
}

macro_rules! dma {
    ($($DMAX:ident: ($dmaXen:ident, $dmaXrst:ident, {
        $($CX:ident: ($ccrX:ident, $cndtrX:ident, $cparX:ident, $cmarX:ident, $cgifX:ident),)+
    }),)+) => {
        $(
            impl DmaExt for $DMAX {
                type Channels = Channels;

                fn split(self, rcc: &mut Rcc) -> Channels {
                    rcc.rb.ahbenr.modify(|_, w| w.$dmaXen().set_bit());
                    $(
                        self.$ccrX.reset();
                    )+
                    Channels((), $($CX { }),+)
                }
            }

            pub struct Channels((), $(pub $CX),+);

            $(
                pub struct $CX;

                impl $CX {
                    /// Associated peripheral `address`
                    ///
                    /// `inc` indicates whether the address will be incremented after every byte transfer
                    pub fn set_peripheral_address(&mut self, address: u32, inc: bool) {
                        let dma = unsafe { &(*$DMAX::ptr()) };

                        dma.$cparX.write(|w| unsafe { w.pa().bits(address) });
                        dma.$ccrX.modify(|_, w| w.pinc().bit(inc) );
                    }

                    /// `address` where from/to data will be read/write
                    ///
                    /// `inc` indicates whether the address will be incremented after every byte transfer
                    pub fn set_memory_address(&mut self, address: u32, inc: bool) {
                        let dma = unsafe { &(*$DMAX::ptr()) };

                        dma.$cmarX.write(|w| unsafe { w.ma().bits(address) });
                        dma.$ccrX.modify(|_, w| w.minc().bit(inc) );
                    }

                    /// Number of bytes to transfer
                    pub fn set_transfer_length(&mut self, len: usize) {
                        let dma = unsafe { &(*$DMAX::ptr()) };

                        dma.$cndtrX.write(|w| unsafe { w.ndt().bits(len as u16) });
                    }

                    /// DMA Transfer direction
                    pub fn set_direction(&mut self, dir: TransferDirection) {
                        let dma = unsafe { &(*$DMAX::ptr()) };
                        match dir {
                            TransferDirection::MemoryToMemory => dma.$ccrX.modify(|_, w| {
                                w.mem2mem().set_bit().circ().clear_bit()
                            }),
                            TransferDirection::MemoryToPeriph => dma.$ccrX.modify(|_, w| {
                                w.mem2mem().clear_bit().circ().clear_bit().dir().set_bit()
                            }),
                            TransferDirection::PeriphToMemory => dma.$ccrX.modify(|_, w| {
                                w.mem2mem().clear_bit().circ().clear_bit().dir().clear_bit()
                            }),
                        }

                    }

                    /// Set channel priority
                    pub fn set_priority(&mut self, priority: Priority) {
                        let dma = unsafe { &(*$DMAX::ptr()) };
                        dma.$ccrX.modify(|_, w| unsafe { w.pl().bits(priority as u8) });
                    }

                    /// Starts the DMA transfer
                    pub fn start(&mut self) {
                        let dma = unsafe { &(*$DMAX::ptr()) };
                        dma.$ccrX.modify(|_, w| w.en().set_bit() );
                    }

                    /// Stops the DMA transfer
                    pub fn stop(&mut self) {
                        let dma = unsafe { &(*$DMAX::ptr()) };

                        // TODO: https://github.com/stm32-rs/stm32-rs/pull/228
                        // dma.ifcr.$cgifX().write(|w| w.set_bit());
                        dma.$ccrX.modify(|_, w| w.en().clear_bit() );
                    }

                    pub fn listen(&mut self, event: Event) {
                        let dma = unsafe { &(*$DMAX::ptr()) };
                        match event {
                            Event::HalfTransfer => dma.$ccrX.modify(|_, w| w.htie().set_bit()),
                            Event::TransferComplete => {
                                dma.$ccrX.modify(|_, w| w.tcie().set_bit())
                            }
                        }
                    }

                    pub fn unlisten(&mut self, event: Event) {
                        let dma = unsafe { &(*$DMAX::ptr()) };
                        match event {
                            Event::HalfTransfer => {
                                dma.$ccrX.modify(|_, w| w.htie().clear_bit())
                            },
                            Event::TransferComplete => {
                                dma.$ccrX.modify(|_, w| w.tcie().clear_bit())
                            }
                        }
                    }
                }

                impl<F, T> CopyDma<F, T> for $CX
                where
                    F: ops::Deref + 'static,
                    T: ops::Deref + 'static,
                    F::Target: AsSlice<Element = u8>,
                    T::Target: AsMutSlice<Element = u8> + Unpin,
                    Self: core::marker::Sized,
                {
                    fn copy(mut self, buf_from: Pin<F>, buf_to: Pin<T>) -> Transfer<Self, (Pin<F>, Pin<T>)> {
                        {
                            let slice_from = buf_from.as_slice();
                            let slice_to = buf_to.as_slice();

                            let (ptr_from, len_from) = (slice_from.as_ptr(), slice_from.len());
                            let (ptr_to, len_to) = (slice_to.as_ptr(), slice_to.len());

                            assert!(len_from == len_to);

                            self.set_direction(TransferDirection::MemoryToMemory);
                            self.set_memory_address(ptr_from as u32, true);
                            self.set_peripheral_address(ptr_to as u32, false);
                            self.set_transfer_length(len_from);
                        }
                        atomic::compiler_fence(Ordering::Release);

                        self.start();

                        Transfer {
                            buffer: (buf_from, buf_to),
                            channel: self,
                        }
                    }
                }
            )+
        )+
    }
}

dma! {
    DMA: (dmaen, dma1rst, {
        C1: ( ccr1, cndtr1, cpar1, cmar1, cgif1 ),
        C2: ( ccr2, cndtr2, cpar2, cmar2, cgif2 ),
        C3: ( ccr3, cndtr3, cpar3, cmar3, cgif3 ),
        C4: ( ccr4, cndtr4, cpar4, cmar4, cgif4 ),
        C5: ( ccr5, cndtr5, cpar5, cmar5, cgif5 ),
        C6: ( ccr6, cndtr6, cpar6, cmar6, cgif6 ),
        C7: ( ccr7, cndtr7, cpar7, cmar7, cgif7 ),
    }),
}
