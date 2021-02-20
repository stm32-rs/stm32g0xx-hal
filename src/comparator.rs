//! Comparator

use core::marker::PhantomData;

#[cfg(feature = "dac")]
use crate::analog::dac;
use crate::gpio::*;
use crate::rcc::Rcc;
use crate::stm32::comp::{COMP1_CSR, COMP2_CSR};
use crate::stm32::COMP;

pub struct COMP1 {
    _rb: PhantomData<()>,
}

impl COMP1 {
    pub fn csr(&self) -> &COMP1_CSR {
        // SAFETY: The COMP1 type is only constructed with logical ownership of
        // these registers.
        &unsafe { &*COMP::ptr() }.comp1_csr
    }
}

pub struct COMP2 {
    _rb: PhantomData<()>,
}

impl COMP2 {
    pub fn csr(&self) -> &COMP2_CSR {
        // SAFETY: The COMP1 type is only constructed with logical ownership of
        // these registers.
        &unsafe { &*COMP::ptr() }.comp2_csr
    }
}

// TODO: Split COMP in PAC
// TODO: COMP3 for STM32G0Bxx etc.

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Config {
    power_mode: PowerMode,
    hysteresis: Hysteresis,
    inverted: bool,
    output_xor: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            hysteresis: Hysteresis::None,
            inverted: false,
            power_mode: PowerMode::HighSpeed,
            output_xor: false,
        }
    }
}

impl Config {
    pub fn hysteresis(mut self, hysteresis: Hysteresis) -> Self {
        self.hysteresis = hysteresis;
        self
    }

    pub fn output_inverted(mut self) -> Self {
        self.inverted = true;
        self
    }

    pub fn output_polarity(mut self, inverted: bool) -> Self {
        self.inverted = inverted;
        self
    }

    pub fn power_mode(mut self, power_mode: PowerMode) -> Self {
        self.power_mode = power_mode;
        self
    }

    /// Sets the output to be Comparator 1 XOR Comparator 2.
    /// Used to implement window comparator mode.
    pub fn output_xor(mut self) -> Self {
        self.output_xor = true;
        self
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum Hysteresis {
    None = 0b00,
    Low = 0b01,
    Medium = 0b10,
    High = 0b11,
}

// TODO
// pub enum Blanking {
//     None,
//     Tim1Oc4(),
//     Tim1Oc5(),
//     Tim2Oc3(),
//     Tim3Oc3(),
//     Tim15Oc2()<
// }

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum PowerMode {
    HighSpeed = 0b00,
    MediumSpeed = 0b01,
}

/// Comparator positive input
pub trait PositiveInput<C> {
    fn setup(&self, comp: &C);
}

/// Comparator negative input
pub trait NegativeInput<C> {
    fn setup(&self, comp: &C);
}

/// Comparator negative input open (not connected)
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Open;

/// Comparator 1 positive input used as positive input for Comparator 2.
/// Used to implement window comparator mode.
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Comp1InP;

/// Comparator 2 positive input used as positive input for Comparator 1.
/// Used to implement window comparator mode.
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Comp2InP;

macro_rules! window_input_pin {
    ($COMP:ident, $pin:ty) => {
        impl PositiveInput<$COMP> for $pin {
            fn setup(&self, comp: &$COMP) {
                comp.csr().modify(|_, w| w.winmode().set_bit())
            }
        }
    };
}

window_input_pin!(COMP1, Comp2InP);
window_input_pin!(COMP2, Comp1InP);

macro_rules! positive_input_pin {
    ($COMP:ident, $pin:ty, $bits:expr) => {
        impl PositiveInput<$COMP> for $pin {
            fn setup(&self, comp: &$COMP) {
                comp.csr().modify(|_, w| unsafe { w.inpsel().bits($bits) })
            }
        }
    };
}

positive_input_pin!(COMP1, gpioc::PC5<Analog>, 0b00);
positive_input_pin!(COMP1, gpiob::PB2<Analog>, 0b01);
positive_input_pin!(COMP1, gpioa::PA1<Analog>, 0b10);
positive_input_pin!(COMP1, Open, 0b11);

positive_input_pin!(COMP2, gpiob::PB4<Analog>, 0b00);
positive_input_pin!(COMP2, gpiob::PB6<Analog>, 0b01);
positive_input_pin!(COMP2, gpioa::PA3<Analog>, 0b10);
positive_input_pin!(COMP2, Open, 0b11);

macro_rules! negative_input_pin {
    ($COMP:ident, $pin:ty, $bits:expr) => {
        impl NegativeInput<$COMP> for $pin {
            fn setup(&self, comp: &$COMP) {
                comp.csr().modify(|_, w| unsafe { w.inmsel().bits($bits) })
            }
        }
    };
}

negative_input_pin!(COMP1, gpiob::PB1<Analog>, 0b0110);
negative_input_pin!(COMP1, gpioc::PC4<Analog>, 0b0111);
negative_input_pin!(COMP1, gpioa::PA0<Analog>, 0b1000);

negative_input_pin!(COMP2, gpiob::PB3<Analog>, 0b0110);
negative_input_pin!(COMP2, gpiob::PB7<Analog>, 0b0111);
negative_input_pin!(COMP2, gpioa::PA2<Analog>, 0b1000);

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum RefintInput {
    /// VRefint * 1/4
    VRefintM14 = 0b0000,
    /// VRefint * 1/2
    VRefintM12 = 0b0001,
    /// VRefint * 3/4
    VRefintM34 = 0b0010,
    /// VRefint
    VRefint = 0b0011,
}

macro_rules! refint_input {
    ($COMP:ident) => {
        impl NegativeInput<$COMP> for RefintInput {
            fn setup(&self, comp: &$COMP) {
                comp.csr()
                    .modify(|_, w| unsafe { w.inmsel().bits(*self as u8) })
            }
        }
    };
}

refint_input!(COMP1);
refint_input!(COMP2);

macro_rules! dac_input {
    ($COMP:ident, $channel:ty, $bits:expr) => {
        impl NegativeInput<$COMP> for $channel {
            fn setup(&self, comp: &$COMP) {
                comp.csr().modify(|_, w| unsafe { w.inmsel().bits($bits) })
            }
        }
    };
}

#[cfg(feature = "dac")]
dac_input!(COMP1, dac::Channel1<dac::Enabled>, 0b0100);
#[cfg(feature = "dac")]
dac_input!(COMP1, dac::Channel2<dac::Enabled>, 0b0101);

#[cfg(feature = "dac")]
dac_input!(COMP2, dac::Channel1<dac::Enabled>, 0b0100);
#[cfg(feature = "dac")]
dac_input!(COMP2, dac::Channel2<dac::Enabled>, 0b0101);

pub struct Comparator<C> {
    regs: C,
}

pub trait ComparatorExt<COMP> {
    fn init<P: PositiveInput<COMP>, N: NegativeInput<COMP>>(
        &mut self,
        positive_input: P,
        negative_input: N,
        config: Config,
    );
    fn output(&self) -> bool;
    fn enable(&self);
    fn disable(&self);
    //fn listen(&self, exti: &mut ) TODO
    //fn unlisten(&self, exti: &mut)
}

macro_rules! comparator_ext {
    ($COMP:ty, $Comparator:ty) => {
        impl ComparatorExt<$COMP> for $Comparator {
            fn init<P: PositiveInput<$COMP>, N: NegativeInput<$COMP>>(
                &mut self,
                positive_input: P,
                negative_input: N,
                config: Config,
            ) {
                positive_input.setup(&self.regs);
                negative_input.setup(&self.regs);
                self.regs.csr().modify(|_, w| unsafe {
                    w.hyst()
                        .bits(config.hysteresis as u8)
                        .polarity()
                        .bit(config.inverted)
                        .pwrmode()
                        .bits(config.power_mode as u8)
                        .winout()
                        .bit(config.output_xor)
                });
            }

            fn output(&self) -> bool {
                self.regs.csr().read().value().bit_is_set()
            }

            fn enable(&self) {
                self.regs.csr().modify(|_, w| w.en().set_bit());
            }

            fn disable(&self) {
                self.regs.csr().modify(|_, w| w.en().clear_bit());
            }
        }
    };
}

comparator_ext!(COMP1, Comparator<COMP1>);
comparator_ext!(COMP2, Comparator<COMP2>);

pub struct WindowComparator {
    comp1: Comparator<COMP1>,
    comp2: Comparator<COMP2>,
}

// TODO: impl for (COMP2, COMP1), (COMP2, COMP3), (COMP3, COMP2)
pub trait WindowComparatorExt {
    /// Returns `true` if the input is between the lower and upper thresholds
    fn output(&self) -> bool;
    /// Returns `true` if the input is above the lower threshold
    fn above_lower(&self) -> bool;
    fn enable(&self);
    fn disable(&self);
}

/// Uses both Comparator 1 and Comparator 2 to implement a window comparator.
/// See Figure 69 in RM0444 Rev 5.
pub fn window_comparator<
    P: PositiveInput<COMP1>,
    L: NegativeInput<COMP2>,
    U: NegativeInput<COMP1>,
>(
    comp: COMP,
    input: P,
    lower_threshold: L,
    upper_threshold: U,
    config: Config,
    rcc: &mut Rcc,
) -> WindowComparator {
    let (mut comp1, mut comp2) = split(comp, rcc);

    let mut config1 = config.clone();
    config1.output_xor = true;
    comp1.init(input, upper_threshold, config1);

    let mut config2 = config;
    config2.output_xor = false;
    comp2.init(Comp1InP, lower_threshold, config2);

    WindowComparator { comp1, comp2 }
}

impl WindowComparator {
    pub fn output(&self) -> bool {
        self.comp1.output()
    }

    pub fn above_lower(&self) -> bool {
        self.comp2.output()
    }

    pub fn enable(&self) {
        self.comp1.enable();
        self.comp2.enable();
    }

    pub fn disable(&self) {
        self.comp1.disable();
        self.comp2.disable();
    }
}

pub fn split(_comp: COMP, rcc: &mut Rcc) -> (Comparator<COMP1>, Comparator<COMP2>) {
    // Enable COMP clocks
    rcc.rb.apbenr2.modify(|_, w| w.syscfgen().set_bit());

    // Reset COMP
    rcc.rb.apbrstr2.modify(|_, w| w.syscfgrst().set_bit());
    rcc.rb.apbrstr2.modify(|_, w| w.syscfgrst().clear_bit());

    (
        Comparator {
            regs: COMP1 { _rb: PhantomData },
        },
        Comparator {
            regs: COMP2 { _rb: PhantomData },
        },
    )
}

pub trait ComparatorSplit {
    fn split(self, rcc: &mut Rcc) -> (Comparator<COMP1>, Comparator<COMP2>);
}

impl ComparatorSplit for COMP {
    fn split(self, rcc: &mut Rcc) -> (Comparator<COMP1>, Comparator<COMP2>) {
        split(self, rcc)
    }
}
