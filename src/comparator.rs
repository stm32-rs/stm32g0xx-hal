//! Comparator

use core::marker::PhantomData;

use crate::analog::dac;
use crate::exti::{Event as ExtiEvent, ExtiExt};
use crate::gpio::*;
use crate::rcc::Rcc;
use crate::stm32::comp::{COMP1_CSR, COMP2_CSR};
use crate::stm32::{COMP, EXTI};

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

#[cfg(any(feature = "stm32g071", feature = "stm32g081"))]
dac_input!(COMP1, dac::Channel1<dac::Enabled>, 0b0100);
#[cfg(any(feature = "stm32g071", feature = "stm32g081"))]
dac_input!(COMP1, dac::Channel2<dac::Enabled>, 0b0101);

#[cfg(any(feature = "stm32g071", feature = "stm32g081"))]
dac_input!(COMP2, dac::Channel1<dac::Enabled>, 0b0100);
#[cfg(any(feature = "stm32g071", feature = "stm32g081"))]
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
    fn output_pin<P: OutputPin<COMP>>(&self, pin: P);
    fn listen(&self, edge: SignalEdge, exti: &EXTI);
    fn unlisten(&self, exti: &EXTI);
    fn is_pending(&self, edge: SignalEdge, exti: &EXTI) -> bool;
    fn unpend(&self, exti: &EXTI);
}

macro_rules! comparator_ext {
    ($COMP:ty, $Comparator:ty, $Event:expr) => {
        impl ComparatorExt<$COMP> for $Comparator {
            // TODO: Require init before any other functions?
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
                // TODO: Delay for comp scaler bridge voltage stabilization?
            }

            fn output(&self) -> bool {
                self.regs.csr().read().value().bit_is_set()
            }

            fn enable(&self) {
                self.regs.csr().modify(|_, w| w.en().set_bit());
                // TODO: Startup delay?
            }

            fn disable(&self) {
                self.regs.csr().modify(|_, w| w.en().clear_bit());
            }

            fn output_pin<P: OutputPin<$COMP>>(&self, pin: P) {
                pin.setup();
            }

            // TODO: Does the COMP need to be disabled when this is enabled?
            /// Enables raising the `ADC_COMP` interrupt at the specified signal edge
            fn listen(&self, edge: SignalEdge, exti: &EXTI) {
                exti.listen($Event, edge);
            }

            fn unlisten(&self, exti: &EXTI) {
                exti.unlisten($Event);
            }

            fn is_pending(&self, edge: SignalEdge, exti: &EXTI) -> bool {
                exti.is_pending($Event, edge)
            }

            fn unpend(&self, exti: &EXTI) {
                exti.unpend($Event);
            }
        }
    };
}

comparator_ext!(COMP1, Comparator<COMP1>, ExtiEvent::COMP1);
comparator_ext!(COMP2, Comparator<COMP2>, ExtiEvent::COMP2);

/// Uses two comparators to implement a window comparator.
/// See Figure 69 in RM0444 Rev 5.
pub struct WindowComparator<U, L> {
    pub upper: Comparator<U>,
    pub lower: Comparator<L>,
}

pub trait WindowComparatorExt<UC, LC> {
    /// Uses two comparators to implement a window comparator.
    /// See Figure 69 in RM0444 Rev 5.
    fn init<I: PositiveInput<UC>, L: NegativeInput<LC>, U: NegativeInput<UC>>(
        &mut self,
        input: I,
        lower_threshold: L,
        upper_threshold: U,
        config: Config,
    );

    /// Returns `true` if the input is between the lower and upper thresholds
    fn output(&self) -> bool;
    fn output_pin<P: OutputPin<UC>>(&self, pin: P);
    /// Returns `true` if the input is above the lower threshold
    fn above_lower(&self) -> bool;
    fn enable(&self);
    fn disable(&self);
    fn listen(&self, edge: SignalEdge, exti: &mut EXTI);
    fn unlisten(&self, exti: &mut EXTI);
    fn is_pending(&self, edge: SignalEdge, exti: &EXTI) -> bool;
    fn unpend(&self, exti: &EXTI);
}

macro_rules! window_comparator {
    ($UPPER:ident, $LOWER:ident, $LOTHR:expr) => {
        impl WindowComparatorExt<$UPPER, $LOWER> for WindowComparator<$UPPER, $LOWER> {
            fn init<
                I: PositiveInput<$UPPER>,
                L: NegativeInput<$LOWER>,
                U: NegativeInput<$UPPER>,
            >(
                &mut self,
                input: I,
                lower_threshold: L,
                upper_threshold: U,
                config: Config,
            ) {
                let mut configu = config.clone();
                configu.output_xor = true;
                self.upper.init(input, upper_threshold, configu);

                let mut configl = config;
                configl.output_xor = false;
                self.lower.init($LOTHR, lower_threshold, configl);
            }

            fn output(&self) -> bool {
                self.upper.output()
            }

            fn output_pin<P: OutputPin<$UPPER>>(&self, pin: P) {
                self.upper.output_pin(pin)
            }

            fn above_lower(&self) -> bool {
                self.lower.output()
            }

            fn enable(&self) {
                self.upper.enable();
                self.lower.enable();
            }

            fn disable(&self) {
                self.upper.disable();
                self.lower.disable();
            }

            // TODO: Does the COMP need to be disabled when this is enabled?
            // TODO: Do EXTI functions need to act on both comparators (like in ST HAL)?
            /// Enables raising the `ADC_COMP` interrupt at the specified signal edge
            fn listen(&self, edge: SignalEdge, exti: &mut EXTI) {
                self.upper.listen(edge, exti)
            }

            fn unlisten(&self, exti: &mut EXTI) {
                self.upper.unlisten(exti)
            }

            fn is_pending(&self, edge: SignalEdge, exti: &EXTI) -> bool {
                self.upper.is_pending(edge, exti)
            }

            fn unpend(&self, exti: &EXTI) {
                self.upper.unpend(exti)
            }
        }
    };
}

window_comparator!(COMP1, COMP2, Comp1InP);
window_comparator!(COMP2, COMP1, Comp2InP);

pub fn split(_comp: COMP, rcc: &mut Rcc) -> (Comparator<COMP1>, Comparator<COMP2>) {
    // Enable COMP, SYSCFG, VREFBUF clocks
    rcc.rb.apbenr2.modify(|_, w| w.syscfgen().set_bit());

    // Reset COMP, SYSCFG, VREFBUF
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

pub trait OutputPin<COMP> {
    fn setup(&self);
}

macro_rules! output_pin {
    ($COMP:ident, $pin:ty) => {
        impl OutputPin<$COMP> for $pin {
            fn setup(&self) {
                self.set_alt_mode(AltFunction::AF7)
            }
        }
    };
}

output_pin!(COMP1, gpioa::PA0<Output<PushPull>>);
output_pin!(COMP1, gpioa::PA0<Output<OpenDrain>>);
output_pin!(COMP1, gpioa::PA6<Output<PushPull>>);
output_pin!(COMP1, gpioa::PA6<Output<OpenDrain>>);
output_pin!(COMP1, gpioa::PA11<Output<PushPull>>);
output_pin!(COMP1, gpioa::PA11<Output<OpenDrain>>);
output_pin!(COMP1, gpiob::PB0<Output<PushPull>>);
output_pin!(COMP1, gpiob::PB0<Output<OpenDrain>>);
output_pin!(COMP1, gpiob::PB10<Output<PushPull>>);
output_pin!(COMP1, gpiob::PB10<Output<OpenDrain>>);

output_pin!(COMP2, gpioa::PA2<Output<PushPull>>);
output_pin!(COMP2, gpioa::PA2<Output<OpenDrain>>);
output_pin!(COMP2, gpioa::PA7<Output<PushPull>>);
output_pin!(COMP2, gpioa::PA7<Output<OpenDrain>>);
output_pin!(COMP2, gpioa::PA12<Output<PushPull>>);
output_pin!(COMP2, gpioa::PA12<Output<OpenDrain>>);
output_pin!(COMP2, gpiob::PB5<Output<PushPull>>);
output_pin!(COMP2, gpiob::PB5<Output<OpenDrain>>);
output_pin!(COMP2, gpiob::PB11<Output<PushPull>>);
output_pin!(COMP2, gpiob::PB11<Output<OpenDrain>>);
