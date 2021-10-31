//! Direct Memory Access Engine
use crate::dmamux::{self, DmaMuxIndex};
use crate::rcc::{Enable, Rcc, Reset};
use crate::stm32::{self, DMA, DMAMUX};

use crate::dmamux::DmaMuxExt;

/// Extension trait to split a DMA peripheral into independent channels
pub trait DmaExt {
    /// The type to split the DMA into
    type Channels;

    /// Reset the DMA peripheral
    fn reset(self, rcc: &mut Rcc) -> Self;

    /// Split the DMA into independent channels
    fn split(self, rcc: &mut Rcc, dmamux: DMAMUX) -> Self::Channels;
}

/// Channel priority level
pub enum Priority {
    /// Low
    Low = 0b00,
    /// Medium
    Medium = 0b01,
    /// High
    High = 0b10,
    /// Very high
    VeryHigh = 0b11,
}

impl From<Priority> for u8 {
    fn from(prio: Priority) -> Self {
        match prio {
            Priority::Low => 0b00,
            Priority::Medium => 0b01,
            Priority::High => 0b10,
            Priority::VeryHigh => 0b11,
        }
    }
}

/// DMA transfer direction
pub enum Direction {
    /// From memory to peripheral
    FromMemory,
    /// From peripheral to memory
    FromPeripheral,
}

impl From<Direction> for bool {
    fn from(dir: Direction) -> Self {
        match dir {
            Direction::FromMemory => true,
            Direction::FromPeripheral => false,
        }
    }
}

#[doc = "Peripheral size"]
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(u8)]
pub enum WordSize {
    #[doc = "0: 8-bit size"]
    BITS8 = 0,
    #[doc = "1: 16-bit size"]
    BITS16 = 1,
    #[doc = "2: 32-bit size"]
    BITS32 = 2,
}
impl From<WordSize> for u8 {
    #[inline(always)]
    fn from(variant: WordSize) -> Self {
        variant as _
    }
}

/// DMA events
pub enum Event {
    /// First half of a transfer is done
    HalfTransfer,
    /// Transfer is complete
    TransferComplete,
    /// A transfer error occurred
    TransferError,
    /// Any of the above events occurred
    Any,
}

mod private {
    use crate::stm32;

    /// Channel methods private to this module
    pub trait Channel {
        /// Return the register block for this channel
        fn ch(&self) -> &stm32::dma::CH;
    }
}

/// Trait implemented by all DMA channels
pub trait Channel: private::Channel {
    /// Connects the DMAMUX channel to the peripheral corresponding to index
    fn select_peripheral(&mut self, index: DmaMuxIndex);

    /// Is the interrupt flag for the given event set?
    fn event_occurred(&self, event: Event) -> bool;

    /// Clear the interrupt flag for the given event.
    ///
    /// Passing `Event::Any` clears all interrupt flags.
    ///
    /// Note that the the global interrupt flag is not automatically cleared
    /// even when all other flags are cleared. The only way to clear it is to
    /// call this method with `Event::Any`.
    fn clear_event(&mut self, event: Event);

    /// Reset the control registers of this channel.
    /// This stops any ongoing transfers.
    fn reset(&mut self) {
        self.ch().cr.reset();
        self.ch().ndtr.reset();
        self.ch().par.reset();
        self.ch().mar.reset();
        self.clear_event(Event::Any);
    }

    /// Set the base address of the peripheral data register from/to which the
    /// data will be read/written.
    ///
    /// Only call this method on disabled channels.
    ///
    /// # Panics
    ///
    /// Panics if this channel is enabled.
    fn set_peripheral_address(&mut self, address: u32, inc: bool) {
        assert!(!self.is_enabled());

        self.ch().par.write(|w| unsafe { w.pa().bits(address) });
        self.ch().cr.modify(|_, w| w.pinc().bit(inc));
    }

    /// Set the base address of the memory area from/to which
    /// the data will be read/written.
    ///
    /// Only call this method on disabled channels.
    ///
    /// # Panics
    ///
    /// Panics if this channel is enabled.
    fn set_memory_address(&mut self, address: u32, inc: bool) {
        assert!(!self.is_enabled());

        self.ch().mar.write(|w| unsafe { w.ma().bits(address) });
        self.ch().cr.modify(|_, w| w.minc().bit(inc));
    }

    /// Set the number of words to transfer.
    ///
    /// Only call this method on disabled channels.
    ///
    /// # Panics
    ///
    /// Panics if this channel is enabled.
    fn set_transfer_length(&mut self, len: u16) {
        assert!(!self.is_enabled());

        self.ch().ndtr.write(|w| unsafe { w.ndt().bits(len) });
    }

    /// Set the word size.
    fn set_word_size(&mut self, wsize: WordSize) {
        self.ch().cr.modify(|_, w| unsafe {
            w.psize().bits(wsize as u8);
            w.msize().bits(wsize as u8)
        });
    }

    /// Set the priority level of this channel
    fn set_priority_level(&mut self, priority: Priority) {
        let pl = priority.into();
        self.ch().cr.modify(|_, w| unsafe { w.pl().bits(pl) });
    }

    /// Set the transfer direction
    fn set_direction(&mut self, direction: Direction) {
        let dir = direction.into();
        self.ch().cr.modify(|_, w| w.dir().bit(dir));
    }

    /// Set the circular mode of this channel
    fn set_circular_mode(&mut self, circular: bool) {
        self.ch().cr.modify(|_, w| w.circ().bit(circular));
    }

    /// Enable the interrupt for the given event
    fn listen(&mut self, event: Event) {
        use Event::*;
        match event {
            HalfTransfer => self.ch().cr.modify(|_, w| w.htie().set_bit()),
            TransferComplete => self.ch().cr.modify(|_, w| w.tcie().set_bit()),
            TransferError => self.ch().cr.modify(|_, w| w.teie().set_bit()),
            Any => self.ch().cr.modify(|_, w| {
                w.htie().set_bit();
                w.tcie().set_bit();
                w.teie().set_bit()
            }),
        }
    }

    /// Disable the interrupt for the given event
    fn unlisten(&mut self, event: Event) {
        use Event::*;
        match event {
            HalfTransfer => self.ch().cr.modify(|_, w| w.htie().clear_bit()),
            TransferComplete => self.ch().cr.modify(|_, w| w.tcie().clear_bit()),
            TransferError => self.ch().cr.modify(|_, w| w.teie().clear_bit()),
            Any => self.ch().cr.modify(|_, w| {
                w.htie().clear_bit();
                w.tcie().clear_bit();
                w.teie().clear_bit()
            }),
        }
    }

    /// Start a transfer
    fn enable(&mut self) {
        self.clear_event(Event::Any);
        self.ch().cr.modify(|_, w| w.en().set_bit());
    }

    /// Stop the current transfer
    fn disable(&mut self) {
        self.ch().cr.modify(|_, w| w.en().clear_bit());
    }

    /// Is there a transfer in progress on this channel?
    fn is_enabled(&self) -> bool {
        self.ch().cr.read().en().bit_is_set()
    }
}

macro_rules! dma {
    (
        channels: {
            $( $Ci:ident: (
                $chi:ident,
                $htifi:ident, $tcifi:ident, $teifi:ident, $gifi:ident,
                $chtifi:ident, $ctcifi:ident, $cteifi:ident, $cgifi:ident,
                $MuxCi: ident
            ), )+
        },
    ) => {

        /// DMA channels
        pub struct Channels {
            $( pub $chi: $Ci, )+
        }

        impl Channels {
            /// Reset the control registers of all channels.
            /// This stops any ongoing transfers.
            fn reset(&mut self) {
                $( self.$chi.reset(); )+
            }
        }


        $(
            /// Singleton that represents a DMA channel
            pub struct $Ci {
                mux: dmamux::$MuxCi,
            }

            impl private::Channel for $Ci {
                fn ch(&self) -> &stm32::dma::CH {
                    // NOTE(unsafe) $Ci grants exclusive access to this register
                    unsafe { &(*DMA::ptr()).$chi }
                }
            }

            impl $Ci {
                pub fn mux(&mut self) -> &mut dyn dmamux::DmaMuxChannel {
                    &mut self.mux
                }
            }

            impl Channel for $Ci {

                fn select_peripheral(&mut self, index: DmaMuxIndex) {
                    self.mux().select_peripheral(index);
                }

                fn event_occurred(&self, event: Event) -> bool {
                    use Event::*;

                    // NOTE(unsafe) atomic read
                    let flags = unsafe { (*DMA::ptr()).isr.read() };
                    match event {
                        HalfTransfer => flags.$htifi().bit_is_set(),
                        TransferComplete => flags.$tcifi().bit_is_set(),
                        TransferError => flags.$teifi().bit_is_set(),
                        Any => flags.$gifi().bit_is_set(),
                    }
                }

                fn clear_event(&mut self, event: Event) {
                    use Event::*;

                    // NOTE(unsafe) atomic write to a stateless register
                    unsafe {
                        let _ = &(*DMA::ptr()).ifcr.write(|w| match event {
                            HalfTransfer => w.$chtifi().set_bit(),
                            TransferComplete => w.$ctcifi().set_bit(),
                            TransferError => w.$cteifi().set_bit(),
                            Any => w.$cgifi().set_bit(),
                        });
                    }
                }

            }
        )+
    }
}

#[cfg(any(feature = "stm32g070", feature = "stm32g071", feature = "stm32g081"))]
dma!(
    channels: {
        C1: (ch1, htif1, tcif1, teif1, gif1, chtif1, ctcif1, cteif1, cgif1, C0),
        C2: (ch2, htif2, tcif2, teif2, gif2, chtif2, ctcif2, cteif2, cgif2, C1),
        C3: (ch3, htif3, tcif3, teif3, gif3, chtif3, ctcif3, cteif3, cgif3, C2),
        C4: (ch4, htif4, tcif4, teif4, gif4, chtif4, ctcif4, cteif4, cgif4, C3),
        C5: (ch5, htif5, tcif5, teif5, gif5, chtif5, ctcif5, cteif5, cgif5, C4),
        C6: (ch6, htif6, tcif6, teif6, gif6, chtif6, ctcif6, cteif6, cgif6, C5),
        C7: (ch7, htif7, tcif7, teif7, gif7, chtif7, ctcif7, cteif7, cgif7, C6),
    },
);

#[cfg(any(feature = "stm32g030", feature = "stm32g031", feature = "stm32g041"))]
dma!(
    channels: {
        C1: (ch1, htif1, tcif1, teif1, gif1, chtif1, ctcif1, cteif1, cgif1, C0),
        C2: (ch2, htif2, tcif2, teif2, gif2, chtif2, ctcif2, cteif2, cgif2, C1),
        C3: (ch3, htif3, tcif3, teif3, gif3, chtif3, ctcif3, cteif3, cgif3, C2),
        C4: (ch4, htif4, tcif4, teif4, gif4, chtif4, ctcif4, cteif4, cgif4, C3),
        C5: (ch5, htif5, tcif5, teif5, gif5, chtif5, ctcif5, cteif5, cgif5, C4),
    },
);

impl DmaExt for DMA {
    type Channels = Channels;

    fn reset(self, rcc: &mut Rcc) -> Self {
        // reset DMA
        <DMA as Reset>::reset(rcc);
        self
    }

    fn split(self, rcc: &mut Rcc, dmamux: DMAMUX) -> Self::Channels {
        let muxchannels = dmamux.split();
        // enable DMA clock
        DMA::enable(rcc);

        let mut channels = Channels {
            ch1: C1 {
                mux: muxchannels.ch0,
            },
            ch2: C2 {
                mux: muxchannels.ch1,
            },
            ch3: C3 {
                mux: muxchannels.ch2,
            },
            ch4: C4 {
                mux: muxchannels.ch3,
            },
            ch5: C5 {
                mux: muxchannels.ch4,
            },
            #[cfg(any(feature = "stm32g070", feature = "stm32g071", feature = "stm32g081"))]
            ch6: C6 {
                mux: muxchannels.ch5,
            },
            #[cfg(any(feature = "stm32g070", feature = "stm32g071", feature = "stm32g081"))]
            ch7: C7 {
                mux: muxchannels.ch6,
            },
        };
        channels.reset();
        channels
    }
}

/// Trait implemented by DMA targets.
pub trait Target {
    /// Returns the correct DMAMUX index to configure DMA channel for this peripheral
    fn dmamux(&self) -> crate::dmamux::DmaMuxIndex;

    /// Enable DMA on the target
    fn enable_dma(&mut self) {}
    /// Disable DMA on the target
    fn disable_dma(&mut self) {}
}
