//! Comparator

use core::marker::PhantomData;

use crate::analog::dac;
use crate::exti::{Event as ExtiEvent, ExtiExt};
use crate::gpio::*;
use crate::rcc::{Clocks, Rcc};
use crate::stm32::comp::{COMP1_CSR, COMP2_CSR};
use crate::stm32::{COMP, EXTI};

/// Enabled Comparator (type state)
pub struct Enabled;

/// Disabled Comparator (type state)
pub struct Disabled;

pub trait ED {}
impl ED for Enabled {}
impl ED for Disabled {}

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
        impl<ED> NegativeInput<$COMP> for &$channel {
            fn setup(&self, comp: &$COMP) {
                comp.csr().modify(|_, w| unsafe { w.inmsel().bits($bits) })
            }
        }
    };
}

#[cfg(any(feature = "stm32g071", feature = "stm32g081"))]
dac_input!(COMP1, dac::Channel1<ED>, 0b0100);
#[cfg(any(feature = "stm32g071", feature = "stm32g081"))]
dac_input!(COMP1, dac::Channel2<ED>, 0b0101);

#[cfg(any(feature = "stm32g071", feature = "stm32g081"))]
dac_input!(COMP2, dac::Channel1<ED>, 0b0100);
#[cfg(any(feature = "stm32g071", feature = "stm32g081"))]
dac_input!(COMP2, dac::Channel2<ED>, 0b0101);

pub struct Comparator<C, ED> {
    regs: C,
    _enabled: PhantomData<ED>,
}

pub trait ComparatorExt<COMP> {
    /// Initializes a comparator
    fn comparator<P: PositiveInput<COMP>, N: NegativeInput<COMP>>(
        self,
        positive_input: P,
        negative_input: N,
        config: Config,
        clocks: &Clocks,
    ) -> Comparator<COMP, Disabled>;
}

macro_rules! impl_comparator {
    ($COMP:ty, $comp:ident, $Event:expr) => {
        impl ComparatorExt<$COMP> for $COMP {
            fn comparator<P: PositiveInput<$COMP>, N: NegativeInput<$COMP>>(
                self,
                positive_input: P,
                negative_input: N,
                config: Config,
                clocks: &Clocks,
            ) -> Comparator<$COMP, Disabled> {
                positive_input.setup(&self);
                negative_input.setup(&self);
                // Delay for scaler voltage bridge initialization for certain negative inputs
                let voltage_scaler_delay = clocks.sys_clk.0 / (1_000_000 / 200); // 200us
                cortex_m::asm::delay(voltage_scaler_delay);
                self.csr().modify(|_, w| unsafe {
                    w.hyst()
                        .bits(config.hysteresis as u8)
                        .polarity()
                        .bit(config.inverted)
                        .pwrmode()
                        .bits(config.power_mode as u8)
                        .winout()
                        .bit(config.output_xor)
                });

                Comparator {
                    regs: self,
                    _enabled: PhantomData,
                }
            }
        }

        impl Comparator<$COMP, Disabled> {
            /// Initializes a comparator
            pub fn $comp<P: PositiveInput<$COMP>, N: NegativeInput<$COMP>>(
                comp: $COMP,
                positive_input: P,
                negative_input: N,
                config: Config,
                clocks: &Clocks,
            ) -> Self {
                comp.comparator(positive_input, negative_input, config, clocks)
            }

            /// Enables the comparator
            pub fn enable(self) -> Comparator<$COMP, Enabled> {
                self.regs.csr().modify(|_, w| w.en().set_bit());
                Comparator {
                    regs: self.regs,
                    _enabled: PhantomData,
                }
            }

            /// Enables raising the `ADC_COMP` interrupt at the specified output signal edge
            pub fn listen(&self, edge: SignalEdge, exti: &EXTI) {
                exti.listen($Event, edge);
            }
        }

        impl Comparator<$COMP, Enabled> {
            /// Returns the value of the output of the comparator
            pub fn output(&self) -> bool {
                self.regs.csr().read().value().bit_is_set()
            }

            /// Disables the comparator
            pub fn disable(self) -> Comparator<$COMP, Disabled> {
                self.regs.csr().modify(|_, w| w.en().clear_bit());
                Comparator {
                    regs: self.regs,
                    _enabled: PhantomData,
                }
            }
        }

        impl<ED> Comparator<$COMP, ED> {
            /// Disables raising interrupts for the output signal
            pub fn unlisten(&self, exti: &EXTI) {
                exti.unlisten($Event);
            }

            /// Returns `true` if the output signal interrupt is pending for the `edge`
            pub fn is_pending(&self, edge: SignalEdge, exti: &EXTI) -> bool {
                exti.is_pending($Event, edge)
            }

            /// Unpends the output signal interrupt
            pub fn unpend(&self, exti: &EXTI) {
                exti.unpend($Event);
            }

            /// Configures a GPIO pin to output the signal of the comparator
            ///
            /// Multiple GPIO pins may be configured as the output simultaneously.
            pub fn output_pin<P: OutputPin<$COMP>>(&self, pin: P) {
                pin.setup();
            }
        }
    };
}

impl_comparator!(COMP1, comp1, ExtiEvent::COMP1);
impl_comparator!(COMP2, comp2, ExtiEvent::COMP2);

/// Uses two comparators to implement a window comparator.
/// See Figure 69 in RM0444 Rev 5.
pub struct WindowComparator<U, L, ED> {
    pub upper: Comparator<U, ED>,
    pub lower: Comparator<L, ED>,
}

pub trait WindowComparatorExt<UC, LC> {
    /// Uses two comparators to implement a window comparator
    ///
    /// See Figure 69 in RM0444 Rev 5. Ignores and overrides the `output_xor` setting in `config`.
    fn window_comparator<I: PositiveInput<UC>, L: NegativeInput<LC>, U: NegativeInput<UC>>(
        self,
        input: I,
        lower_threshold: L,
        upper_threshold: U,
        config: Config,
        clocks: &Clocks,
    ) -> WindowComparator<UC, LC, Disabled>;
}

macro_rules! impl_window_comparator {
    ($UPPER:ident, $LOWER:ident, $LOTHR:expr) => {
        impl WindowComparatorExt<$UPPER, $LOWER> for ($UPPER, $LOWER) {
            fn window_comparator<
                I: PositiveInput<$UPPER>,
                L: NegativeInput<$LOWER>,
                U: NegativeInput<$UPPER>,
            >(
                self,
                input: I,
                lower_threshold: L,
                upper_threshold: U,
                config: Config,
                clocks: &Clocks,
            ) -> WindowComparator<$UPPER, $LOWER, Disabled> {
                let (upper, lower) = self;

                let mut configu = config.clone();
                configu.output_xor = true;
                let upper = upper.comparator(input, upper_threshold, configu, clocks);

                let mut configl = config;
                configl.output_xor = false;
                let lower = lower.comparator($LOTHR, lower_threshold, configl, clocks);

                WindowComparator { upper, lower }
            }
        }

        impl WindowComparator<$UPPER, $LOWER, Disabled> {
            /// Enables the comparator
            pub fn enable(self) -> WindowComparator<$UPPER, $LOWER, Enabled> {
                WindowComparator {
                    upper: self.upper.enable(),
                    lower: self.lower.enable(),
                }
            }

            /// Enables raising the `ADC_COMP` interrupt at the specified signal edge
            pub fn listen(&self, edge: SignalEdge, exti: &mut EXTI) {
                self.upper.listen(edge, exti)
            }
        }

        impl WindowComparator<$UPPER, $LOWER, Enabled> {
            /// Disables the comparator
            pub fn disable(self) -> WindowComparator<$UPPER, $LOWER, Disabled> {
                WindowComparator {
                    upper: self.upper.disable(),
                    lower: self.lower.disable(),
                }
            }

            /// Returns the value of the output of the comparator
            pub fn output(&self) -> bool {
                self.upper.output()
            }

            /// Returns `true` if the input signal is above the lower threshold
            pub fn above_lower(&self) -> bool {
                self.lower.output()
            }
        }

        impl<ED> WindowComparator<$UPPER, $LOWER, ED> {
            /// Configures a GPIO pin to output the signal of the comparator
            ///
            /// Multiple GPIO pins may be configured as the output simultaneously.
            pub fn output_pin<P: OutputPin<$UPPER>>(&self, pin: P) {
                self.upper.output_pin(pin)
            }

            /// Disables raising interrupts for the output signal
            pub fn unlisten(&self, exti: &mut EXTI) {
                self.upper.unlisten(exti)
            }

            /// Returns `true` if the output signal interrupt is pending for the `edge`
            pub fn is_pending(&self, edge: SignalEdge, exti: &EXTI) -> bool {
                self.upper.is_pending(edge, exti)
            }

            /// Unpends the output signal interrupt
            pub fn unpend(&self, exti: &EXTI) {
                self.upper.unpend(exti)
            }
        }
    };
}

impl_window_comparator!(COMP1, COMP2, Comp1InP);
impl_window_comparator!(COMP2, COMP1, Comp2InP);

pub fn window_comparator12<
    I: PositiveInput<COMP1>,
    L: NegativeInput<COMP2>,
    U: NegativeInput<COMP1>,
>(
    comp: COMP,
    input: I,
    lower_threshold: L,
    upper_threshold: U,
    config: Config,
    rcc: &mut Rcc,
) -> WindowComparator<COMP1, COMP2, Disabled> {
    let (comp1, comp2) = comp.split(rcc);
    (comp1, comp2).window_comparator(input, lower_threshold, upper_threshold, config, &rcc.clocks)
}

pub fn window_comparator21<
    I: PositiveInput<COMP2>,
    L: NegativeInput<COMP1>,
    U: NegativeInput<COMP2>,
>(
    comp: COMP,
    input: I,
    lower_threshold: L,
    upper_threshold: U,
    config: Config,
    rcc: &mut Rcc,
) -> WindowComparator<COMP2, COMP1, Disabled> {
    let (comp1, comp2) = comp.split(rcc);
    (comp2, comp1).window_comparator(input, lower_threshold, upper_threshold, config, &rcc.clocks)
}

/// Enables the comparator peripheral, and splits the [`COMP`] into independent [`COMP1`] and [`COMP2`]
pub fn split(_comp: COMP, rcc: &mut Rcc) -> (COMP1, COMP2) {
    // Enable COMP, SYSCFG, VREFBUF clocks
    rcc.rb.apbenr2.modify(|_, w| w.syscfgen().set_bit());

    // Reset COMP, SYSCFG, VREFBUF
    rcc.rb.apbrstr2.modify(|_, w| w.syscfgrst().set_bit());
    rcc.rb.apbrstr2.modify(|_, w| w.syscfgrst().clear_bit());

    (COMP1 { _rb: PhantomData }, COMP2 { _rb: PhantomData })
}

pub trait ComparatorSplit {
    /// Enables the comparator peripheral, and splits the [`COMP`] into independent [`COMP1`] and [`COMP2`]
    fn split(self, rcc: &mut Rcc) -> (COMP1, COMP2);
}

impl ComparatorSplit for COMP {
    fn split(self, rcc: &mut Rcc) -> (COMP1, COMP2) {
        split(self, rcc)
    }
}

pub trait OutputPin<COMP> {
    fn setup(&self);
    fn release(self) -> Self;
}

macro_rules! output_pin_push_pull {
    ($COMP:ident, $pin:ty) => {
        impl OutputPin<$COMP> for $pin {
            fn setup(&self) {
                self.set_alt_mode(AltFunction::AF7)
            }

            fn release(self) -> Self {
                self.into_push_pull_output()
            }
        }
    };
}

macro_rules! output_pin_open_drain {
    ($COMP:ident, $pin:ty) => {
        impl OutputPin<$COMP> for $pin {
            fn setup(&self) {
                self.set_alt_mode(AltFunction::AF7)
            }

            fn release(self) -> Self {
                self.into_open_drain_output()
            }
        }
    };
}

output_pin_push_pull!(COMP1, gpioa::PA0<Output<PushPull>>);
output_pin_open_drain!(COMP1, gpioa::PA0<Output<OpenDrain>>);
output_pin_push_pull!(COMP1, gpioa::PA6<Output<PushPull>>);
output_pin_open_drain!(COMP1, gpioa::PA6<Output<OpenDrain>>);
output_pin_push_pull!(COMP1, gpioa::PA11<Output<PushPull>>);
output_pin_open_drain!(COMP1, gpioa::PA11<Output<OpenDrain>>);
output_pin_push_pull!(COMP1, gpiob::PB0<Output<PushPull>>);
output_pin_open_drain!(COMP1, gpiob::PB0<Output<OpenDrain>>);
output_pin_push_pull!(COMP1, gpiob::PB10<Output<PushPull>>);
output_pin_open_drain!(COMP1, gpiob::PB10<Output<OpenDrain>>);

output_pin_push_pull!(COMP2, gpioa::PA2<Output<PushPull>>);
output_pin_open_drain!(COMP2, gpioa::PA2<Output<OpenDrain>>);
output_pin_push_pull!(COMP2, gpioa::PA7<Output<PushPull>>);
output_pin_open_drain!(COMP2, gpioa::PA7<Output<OpenDrain>>);
output_pin_push_pull!(COMP2, gpioa::PA12<Output<PushPull>>);
output_pin_open_drain!(COMP2, gpioa::PA12<Output<OpenDrain>>);
output_pin_push_pull!(COMP2, gpiob::PB5<Output<PushPull>>);
output_pin_open_drain!(COMP2, gpiob::PB5<Output<OpenDrain>>);
output_pin_push_pull!(COMP2, gpiob::PB11<Output<PushPull>>);
output_pin_open_drain!(COMP2, gpiob::PB11<Output<OpenDrain>>);
