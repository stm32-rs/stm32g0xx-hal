//! General Purpose Input / Output
use core::marker::PhantomData;

use crate::rcc::Rcc;
use core::convert::Infallible;
use hal::digital::{ErrorType, InputPin, OutputPin, PinState, StatefulOutputPin};

/// Default pin mode
pub type DefaultMode = Analog;

/// Extension trait to split a GPIO peripheral in independent pins and registers
pub trait GpioExt {
    /// The parts to split the GPIO into
    type Parts;

    /// Splits the GPIO block into independent pins and registers
    fn split(self, rcc: &mut Rcc) -> Self::Parts;
}

trait GpioRegExt {
    fn is_low(&self, pos: u8) -> bool;
    fn is_set_low(&self, pos: u8) -> bool;
    fn set_high(&self, pos: u8);
    fn set_low(&self, pos: u8);
}

/// Input mode (type state)
pub struct Input<MODE> {
    _mode: PhantomData<MODE>,
}

/// Floating input (type state)
pub struct Floating;

/// Pulled down input (type state)
pub struct PullDown;

/// Pulled up input (type state)
pub struct PullUp;

/// Open drain input or output (type state)
pub struct OpenDrain;

/// Analog mode (type state)
pub struct Analog;

/// Output mode (type state)
pub struct Output<MODE> {
    _mode: PhantomData<MODE>,
}

/// Push pull output (type state)
pub struct PushPull;

/// Fully erased pin
pub struct Pin<MODE> {
    i: u8,
    port: *const dyn GpioRegExt,
    _mode: PhantomData<MODE>,
}

macro_rules! gpio_trait {
    ($gpiox:ident) => {
        impl GpioRegExt for crate::stm32::$gpiox::RegisterBlock {
            fn is_low(&self, pos: u8) -> bool {
                // NOTE(unsafe) atomic read with no side effects
                self.idr().read().bits() & (1 << pos) == 0
            }

            fn is_set_low(&self, pos: u8) -> bool {
                // NOTE(unsafe) atomic read with no side effects
                self.odr().read().bits() & (1 << pos) == 0
            }

            fn set_high(&self, pos: u8) {
                // NOTE(unsafe) atomic write to a stateless register
                unsafe {
                    self.bsrr().write(|w| w.bits(1 << pos));
                }
            }

            fn set_low(&self, pos: u8) {
                // NOTE(unsafe) atomic write to a stateless register
                unsafe {
                    self.bsrr().write(|w| w.bits(1 << (pos + 16)));
                }
            }
        }
    };
}

gpio_trait!(gpioa);
gpio_trait!(gpiob);

// NOTE(unsafe) The only write acess is to BSRR, which is thread safe
unsafe impl<MODE> Sync for Pin<MODE> {}
// NOTE(unsafe) this only enables read access to the same pin from multiple
// threads
unsafe impl<MODE> Send for Pin<MODE> {}

impl<MODE> ErrorType for Pin<Output<MODE>> {
    type Error = Infallible;
}

impl<MODE> StatefulOutputPin for Pin<Output<MODE>> {
    #[inline(always)]
    fn is_set_high(&mut self) -> Result<bool, Self::Error> {
        self.is_set_low().map(|v| !v)
    }

    #[inline(always)]
    fn is_set_low(&mut self) -> Result<bool, Self::Error> {
        Ok(unsafe { (*self.port).is_set_low(self.i) })
    }
}

impl<MODE> OutputPin for Pin<Output<MODE>> {
    #[inline(always)]
    fn set_high(&mut self) -> Result<(), Self::Error> {
        unsafe { (*self.port).set_high(self.i) };
        Ok(())
    }

    #[inline(always)]
    fn set_low(&mut self) -> Result<(), Self::Error> {
        unsafe { (*self.port).set_low(self.i) }
        Ok(())
    }
}

impl InputPin for Pin<Output<OpenDrain>> {
    #[inline(always)]
    fn is_high(&mut self) -> Result<bool, Self::Error> {
        self.is_low().map(|v| !v)
    }

    #[inline(always)]
    fn is_low(&mut self) -> Result<bool, Self::Error> {
        Ok(unsafe { (*self.port).is_low(self.i) })
    }
}

impl<MODE> ErrorType for Pin<Input<MODE>> {
    type Error = Infallible;
}

impl<MODE> InputPin for Pin<Input<MODE>> {
    #[inline(always)]
    fn is_high(&mut self) -> Result<bool, Self::Error> {
        self.is_low().map(|v| !v)
    }

    #[inline(always)]
    fn is_low(&mut self) -> Result<bool, Self::Error> {
        Ok(unsafe { (*self.port).is_low(self.i) })
    }
}

/// GPIO Pin speed selection
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Speed {
    Low = 0,
    Medium = 1,
    High = 2,
    VeryHigh = 3,
}

/// Trigger edge
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum SignalEdge {
    Rising,
    Falling,
    All,
}

#[allow(dead_code)]
pub(crate) enum AltFunction {
    AF0 = 0,
    AF1 = 1,
    AF2 = 2,
    AF3 = 3,
    AF4 = 4,
    AF5 = 5,
    AF6 = 6,
    AF7 = 7,
}

macro_rules! gpio {
    ($GPIOX:ident, $gpiox:ident, $PXx:ident, $Pxn:expr, [
        $($PXi:ident: ($pxi:ident, $i:expr),)+
    ]) => {
        /// GPIO
        pub mod $gpiox {
            use core::convert::Infallible;
            use core::marker::PhantomData;
            use crate::stm32::{EXTI, $GPIOX};
            use crate::exti::{ExtiExt, Event};
            use crate::rcc::{Enable, Rcc};
            use super::*;

            /// GPIO parts
            pub struct Parts {
                $(
                    pub $pxi: $PXi<DefaultMode>,
                )+
            }

            impl GpioExt for $GPIOX {
                type Parts = Parts;

                fn split(self, rcc: &mut Rcc) -> Parts {
                    <$GPIOX>::enable(rcc);

                    Parts {
                        $(
                            $pxi: $PXi { _mode: PhantomData },
                        )+
                    }
                }
            }

            /// Partially erased pin
            pub struct $PXx<MODE> {
                i: u8,
                _mode: PhantomData<MODE>,
            }

            impl<MODE> ErrorType for $PXx<Output<MODE>> {
                type Error = Infallible;
            }

            impl<MODE> OutputPin for $PXx<Output<MODE>> {
                fn set_high(&mut self) -> Result<(), Self::Error> {
                    // NOTE(unsafe) atomic write to a stateless register
                    unsafe { (*$GPIOX::ptr()).bsrr().write(|w| w.bits(1 << self.i)) };
                    Ok(())
                }

                fn set_low(&mut self) -> Result<(), Self::Error> {
                    // NOTE(unsafe) atomic write to a stateless register
                    unsafe { (*$GPIOX::ptr()).bsrr().write(|w| w.bits(1 << (self.i + 16))) };
                    Ok(())
                }
            }

            impl<MODE> StatefulOutputPin for $PXx<Output<MODE>> {
                fn is_set_high(&mut self) -> Result<bool, Self::Error> {
                    let is_set_high = !self.is_set_low()?;
                    Ok(is_set_high)
                }

                fn is_set_low(&mut self) -> Result<bool, Self::Error> {
                    // NOTE(unsafe) atomic read with no side effects
                    let is_set_low = unsafe { (*$GPIOX::ptr()).odr().read().bits() & (1 << self.i) == 0 };
                    Ok(is_set_low)
                }
            }

            impl<MODE> InputPin for $PXx<Output<MODE>> {
                fn is_high(&mut self) -> Result<bool, Self::Error> {
                    let is_high = !self.is_low()?;
                    Ok(is_high)
                }

                fn is_low(&mut self) -> Result<bool, Self::Error>  {
                    // NOTE(unsafe) atomic read with no side effects
                    let is_low = unsafe { (*$GPIOX::ptr()).idr().read().bits() & (1 << self.i) == 0 };
                    Ok(is_low)
                }
            }

            impl<MODE> ErrorType for $PXx<Input<MODE>> {
                type Error = Infallible;
            }

            impl<MODE> InputPin for $PXx<Input<MODE>> {
                fn is_high(&mut self) -> Result<bool, Self::Error> {
                    let is_high = !self.is_low()?;
                    Ok(is_high)
                }

                fn is_low(&mut self) -> Result<bool, Self::Error> {
                    // NOTE(unsafe) atomic read with no side effects
                    let is_low = unsafe { (*$GPIOX::ptr()).idr().read().bits() & (1 << self.i) == 0 };
                    Ok(is_low)
                }
            }

            $(
                pub struct $PXi<MODE> {
                    _mode: PhantomData<MODE>,
                }

                #[allow(clippy::from_over_into)]
                impl Into<$PXi<Input<PullDown>>> for $PXi<DefaultMode> {
                    fn into(self) -> $PXi<Input<PullDown>> {
                        self.into_pull_down_input()
                    }
                }

                #[allow(clippy::from_over_into)]
                impl Into<$PXi<Input<PullUp>>> for $PXi<DefaultMode> {
                    fn into(self) -> $PXi<Input<PullUp>> {
                        self.into_pull_up_input()
                    }
                }

                #[allow(clippy::from_over_into)]
                impl Into<$PXi<Input<Floating>>> for $PXi<DefaultMode> {
                    fn into(self) -> $PXi<Input<Floating>> {
                        self.into_floating_input()
                    }
                }

                #[allow(clippy::from_over_into)]
                impl Into<$PXi<Output<OpenDrain>>> for $PXi<DefaultMode> {
                    fn into(self) -> $PXi<Output<OpenDrain>> {
                        self.into_open_drain_output()
                    }
                }

                #[allow(clippy::from_over_into)]
                impl Into<$PXi<Output<PushPull>>> for $PXi<DefaultMode> {
                    fn into(self) -> $PXi<Output<PushPull>> {
                        self.into_push_pull_output()
                    }
                }

                impl<MODE> $PXi<MODE> {
                    /// Configures the pin to operate as a floating input pin
                    pub fn into_floating_input(self) -> $PXi<Input<Floating>> {
                        let offset = 2 * $i;
                        unsafe {
                            let gpio = &(*$GPIOX::ptr());
                            gpio.pupdr().modify(|r, w| {
                                w.bits(r.bits() & !(0b11 << offset))
                            });
                            gpio.moder().modify(|r, w| {
                                w.bits(r.bits() & !(0b11 << offset))
                            })
                        };
                        $PXi { _mode: PhantomData }
                    }

                    /// Configures the pin to operate as a pulled down input pin
                    pub fn into_pull_down_input(self) -> $PXi<Input<PullDown>> {
                        let offset = 2 * $i;
                        unsafe {
                            let gpio = &(*$GPIOX::ptr());
                            gpio.pupdr().modify(|r, w| {
                                w.bits((r.bits() & !(0b11 << offset)) | (0b10 << offset))
                            });
                            gpio.moder().modify(|r, w| {
                                w.bits(r.bits() & !(0b11 << offset))
                            })
                        };
                        $PXi { _mode: PhantomData }
                    }

                    /// Configures the pin to operate as a pulled up input pin
                    pub fn into_pull_up_input(self) -> $PXi<Input<PullUp>> {
                        let offset = 2 * $i;
                        unsafe {
                            let gpio = &(*$GPIOX::ptr());
                            gpio.pupdr().modify(|r, w| {
                                w.bits((r.bits() & !(0b11 << offset)) | (0b01 << offset))
                            });
                            gpio.moder().modify(|r, w| {
                                w.bits(r.bits() & !(0b11 << offset))
                            })
                        };
                        $PXi { _mode: PhantomData }
                    }

                    /// Configures the pin to operate as an analog pin
                    pub fn into_analog(self) -> $PXi<Analog> {
                        let offset = 2 * $i;
                        unsafe {
                            let gpio = &(*$GPIOX::ptr());
                            gpio.pupdr().modify(|r, w| {
                                w.bits(r.bits() & !(0b11 << offset))
                            });
                            gpio.moder().modify(|r, w| {
                                w.bits((r.bits() & !(0b11 << offset)) | (0b11 << offset))
                            });
                        }
                        $PXi { _mode: PhantomData }
                    }

                    /// Configures the pin to operate as an open drain output
                    /// pin with `initial_state` specifying whether the pin
                    /// should initially be high or low
                    pub fn into_open_drain_output_in_state(mut self, initial_state: PinState) -> $PXi<Output<OpenDrain>> {
                        self.internal_set_state(initial_state);
                        self.into_open_drain_output()
                    }

                    /// Configures the pin to operate as an open drain output pin
                    pub fn into_open_drain_output(self) -> $PXi<Output<OpenDrain>> {
                        let offset = 2 * $i;
                        unsafe {
                            let gpio = &(*$GPIOX::ptr());
                            gpio.pupdr().modify(|r, w| {
                                w.bits(r.bits() & !(0b11 << offset))
                            });
                            gpio.otyper().modify(|r, w| {
                                w.bits(r.bits() | (0b1 << $i))
                            });
                            gpio.moder().modify(|r, w| {
                                w.bits((r.bits() & !(0b11 << offset)) | (0b01 << offset))
                            })
                        };
                        $PXi { _mode: PhantomData }
                    }

                    /// Configures the pin to operate as a push pull output pin
                    /// with `initial_state` specifying whether the pin should
                    /// initially be high or low
                    pub fn into_push_pull_output_in_state(mut self, initial_state: PinState) -> $PXi<Output<PushPull>> {
                        self.internal_set_state(initial_state);
                        self.into_push_pull_output()
                    }

                    /// Configures the pin to operate as a push pull output pin
                    pub fn into_push_pull_output(self) -> $PXi<Output<PushPull>> {
                        let offset = 2 * $i;
                        unsafe {
                            let gpio = &(*$GPIOX::ptr());
                            gpio.pupdr().modify(|r, w| {
                                w.bits(r.bits() & !(0b11 << offset))
                            });
                            gpio.otyper().modify(|r, w| {
                                w.bits(r.bits() & !(0b1 << $i))
                            });
                            gpio.moder().modify(|r, w| {
                                w.bits((r.bits() & !(0b11 << offset)) | (0b01 << offset))
                            })
                        };
                        $PXi { _mode: PhantomData }
                    }

                    /// Configures the pin as external trigger
                    pub fn listen(self, edge: SignalEdge, exti: &mut EXTI) -> $PXi<Input<Floating>> {
                        let offset = 2 * $i;
                        unsafe {
                            let _ = &(*$GPIOX::ptr()).pupdr().modify(|r, w| {
                                w.bits(r.bits() & !(0b11 << offset))
                            });
                            let _ = &(*$GPIOX::ptr()).moder().modify(|r, w| {
                                w.bits(r.bits() & !(0b11 << offset))
                            });
                        };
                        let offset = ($i % 4) * 8;
                        let mask = $Pxn << offset;
                        let reset = !(0xff << offset);
                        match $i as u8 {
                            0..=3   => exti.exticr1().modify(|r, w| unsafe {
                                w.bits(r.bits() & reset | mask)
                            }),
                            4..=7  => exti.exticr2().modify(|r, w| unsafe {
                                w.bits(r.bits() & reset | mask)
                            }),
                            8..=11 => exti.exticr3().modify(|r, w| unsafe {
                                w.bits(r.bits() & reset | mask)
                            }),
                            12..=16 => exti.exticr4().modify(|r, w| unsafe {
                                w.bits(r.bits() & reset | mask)
                            }),
                            _ => unreachable!(),
                        };
                        exti.listen(Event::from_code($i), edge);
                        $PXi { _mode: PhantomData }
                    }

                    /// Set pin speed
                    pub fn set_speed(self, speed: Speed) -> Self {
                        let offset = 2 * $i;
                        unsafe {
                            let _ = &(*$GPIOX::ptr()).ospeedr().modify(|r, w| {
                                w.bits((r.bits() & !(0b11 << offset)) | ((speed as u32) << offset))
                            });
                        };
                        self
                    }

                    #[allow(dead_code)]
                    pub(crate) fn set_alt_mode(&self, mode: AltFunction) {
                        let mode = mode as u32;
                        let offset = 2 * $i;
                        let offset2 = 4 * $i;
                        unsafe {
                            let gpio = &(*$GPIOX::ptr());
                            if offset2 < 32 {
                                gpio.afrl().modify(|r, w| {
                                    w.bits((r.bits() & !(0b1111 << offset2)) | (mode << offset2))
                                });
                            } else {
                                let offset2 = offset2 - 32;
                                gpio.afrh().modify(|r, w| {
                                    w.bits((r.bits() & !(0b1111 << offset2)) | (mode << offset2))
                                });
                            }
                            gpio.moder().modify(|r, w| {
                                w.bits((r.bits() & !(0b11 << offset)) | (0b10 << offset))
                            });
                        }
                    }

                    fn internal_set_state(&mut self, state: PinState) {
                        match state {
                            PinState::High => {
                                // NOTE(unsafe) atomic write to a stateless register
                                unsafe { (*$GPIOX::ptr()).bsrr().write(|w| w.bits(1 << $i)) };
                            }
                            PinState::Low => {
                                // NOTE(unsafe) atomic write to a stateless register
                                unsafe { (*$GPIOX::ptr()).bsrr().write(|w| w.bits(1 << ($i + 16))) };
                            }
                        }
                    }
                }

                impl<MODE> $PXi<Output<MODE>> {
                    /// Erases the pin number from the type
                    ///
                    /// This is useful when you want to collect the pins into an array where you
                    /// need all the elements to have the same type
                    pub fn downgrade(self) -> $PXx<Output<MODE>> {
                        $PXx { i: $i, _mode: self._mode }
                    }
                }

                impl<MODE> ErrorType for $PXi<Output<MODE>> {
                    type Error = Infallible;
                }

                impl<MODE> OutputPin for $PXi<Output<MODE>> {
                    fn set_high(&mut self) -> Result<(), Self::Error> {
                        self.internal_set_state(PinState::High);
                        Ok(())
                    }

                    fn set_low(&mut self) -> Result<(), Self::Error>{
                        self.internal_set_state(PinState::Low);
                        Ok(())
                    }
                }

                impl<MODE> StatefulOutputPin for $PXi<Output<MODE>> {
                    fn is_set_high(&mut self) -> Result<bool, Self::Error> {
                        let is_set_high = !self.is_set_low()?;
                        Ok(is_set_high)
                    }

                    fn is_set_low(&mut self) -> Result<bool, Self::Error> {
                        // NOTE(unsafe) atomic read with no side effects
                        let is_set_low = unsafe { (*$GPIOX::ptr()).odr().read().bits() & (1 << $i) == 0 };
                        Ok(is_set_low)
                    }
                }

                impl<MODE> InputPin for $PXi<Output<MODE>> {
                    fn is_high(&mut self) -> Result<bool, Self::Error> {
                        let is_high = !self.is_low()?;
                        Ok(is_high)
                    }

                    fn is_low(&mut self) -> Result<bool, Self::Error>  {
                        // NOTE(unsafe) atomic read with no side effects
                        let is_low = unsafe { (*$GPIOX::ptr()).idr().read().bits() & (1 << $i) == 0 };
                        Ok(is_low)
                    }
                }

                impl<MODE> $PXi<Input<MODE>> {
                    /// Erases the pin number from the type
                    ///
                    /// This is useful when you want to collect the pins into an array where you
                    /// need all the elements to have the same type
                    pub fn downgrade(self) -> $PXx<Input<MODE>> {
                        $PXx { i: $i, _mode: self._mode }
                    }
                }

                impl<MODE> ErrorType for $PXi<Input<MODE>> {
                    type Error = Infallible;
                }

                impl<MODE> InputPin for $PXi<Input<MODE>> {
                    fn is_high(&mut self) -> Result<bool, Self::Error> {
                        let is_high = !self.is_low()?;
                        Ok(is_high)
                    }

                    fn is_low(&mut self) -> Result<bool, Self::Error> {
                        // NOTE(unsafe) atomic read with no side effects
                        let is_low = unsafe { (*$GPIOX::ptr()).idr().read().bits() & (1 << $i) == 0 };
                        Ok(is_low)
                    }
                }
            )+

            impl<TYPE> $PXx<TYPE> {
                pub fn get_id (&self) -> u8 {
                    self.i
                }
            }

            impl<MODE> $PXx<Output<MODE>> {
                /// Erases the port number from the type
                ///
                /// This is useful when you want to collect the pins into an array where you
                /// need all the elements to have the same type
                pub fn downgrade(self) -> Pin<Output<MODE>> {
                    Pin {
                        i: self.get_id(),
                        port: $GPIOX::ptr() as *const dyn GpioRegExt,
                        _mode: self._mode,
                    }
                }
            }

            impl<MODE> $PXx<Input<MODE>> {
                /// Erases the port number from the type
                ///
                /// This is useful when you want to collect the pins into an array where you
                /// need all the elements to have the same type
                pub fn downgrade(self) -> Pin<Input<MODE>> {
                    Pin {
                        i: self.get_id(),
                        port: $GPIOX::ptr() as *const dyn GpioRegExt,
                        _mode: self._mode,
                    }
                }
            }
        }

        pub use $gpiox::{ $($PXi,)+ };
    }
}

gpio!(GPIOA, gpioa, PA, 0, [
    PA0: (pa0, 0),
    PA1: (pa1, 1),
    PA2: (pa2, 2),
    PA3: (pa3, 3),
    PA4: (pa4, 4),
    PA5: (pa5, 5),
    PA6: (pa6, 6),
    PA7: (pa7, 7),
    PA8: (pa8, 8),
    PA9: (pa9, 9),
    PA10: (pa10, 10),
    PA11: (pa11, 11),
    PA12: (pa12, 12),
    PA13: (pa13, 13),
    PA14: (pa14, 14),
    PA15: (pa15, 15),
]);

gpio!(GPIOB, gpiob, PB, 1, [
    PB0: (pb0, 0),
    PB1: (pb1, 1),
    PB2: (pb2, 2),
    PB3: (pb3, 3),
    PB4: (pb4, 4),
    PB5: (pb5, 5),
    PB6: (pb6, 6),
    PB7: (pb7, 7),
    PB8: (pb8, 8),
    PB9: (pb9, 9),
    PB10: (pb10, 10),
    PB11: (pb11, 11),
    PB12: (pb12, 12),
    PB13: (pb13, 13),
    PB14: (pb14, 14),
    PB15: (pb15, 15),
]);

gpio!(GPIOC, gpioc, PC, 2, [
    PC0: (pc0, 0),
    PC1: (pc1, 1),
    PC2: (pc2, 2),
    PC3: (pc3, 3),
    PC4: (pc4, 4),
    PC5: (pc5, 5),
    PC6: (pc6, 6),
    PC7: (pc7, 7),
    PC8: (pc8, 8),
    PC9: (pc9, 9),
    PC10: (pc10, 10),
    PC11: (pc11, 11),
    PC12: (pc12, 12),
    PC13: (pc13, 13),
    PC14: (pc14, 14),
    PC15: (pc15, 15),
]);

gpio!(GPIOD, gpiod, PD, 3, [
    PD0: (pd0, 0),
    PD1: (pd1, 1),
    PD2: (pd2, 2),
    PD3: (pd3, 3),
    PD4: (pd4, 4),
    PD5: (pd5, 5),
    PD6: (pd6, 6),
    PD7: (pd7, 7),
    PD8: (pd8, 8),
    PD9: (pd9, 9),
    PD10: (pd10, 10),
    PD11: (pd11, 11),
    PD12: (pd12, 12),
    PD13: (pd13, 13),
    PD14: (pd14, 14),
    PD15: (pd15, 15),
]);

#[cfg(feature = "stm32g0b1")]
gpio!(GPIOE, gpioe, PE, 4, [
    PE0: (pe0, 0),
    PE1: (pe1, 1),
    PE2: (pe2, 2),
    PE3: (pe3, 3),
    PE4: (pe4, 4),
    PE5: (pe5, 5),
    PE6: (pe6, 6),
    PE7: (pe7, 7),
    PE8: (pe8, 8),
    PE9: (pe9, 9),
    PE10: (pe10, 10),
    PE11: (pe11, 11),
    PE12: (pe12, 12),
    PE13: (pe13, 13),
    PE14: (pe14, 14),
    PE15: (pe15, 15),
]);

gpio!(GPIOF, gpiof, PF, 5, [
    PF0: (pf0, 0),
    PF1: (pf1, 1),
    PF2: (pf2, 2),
    PF3: (pf3, 3),
    PF4: (pf4, 4),
    PF5: (pf5, 5),
    PF6: (pf6, 6),
    PF7: (pf7, 7),
    PF8: (pf8, 8),
    PF9: (pf9, 9),
    PF10: (pf10, 10),
    PF11: (pf11, 11),
    PF12: (pf12, 12),
    PF13: (pf13, 13),
    PF14: (pf14, 14),
    PF15: (pf15, 15),
]);
