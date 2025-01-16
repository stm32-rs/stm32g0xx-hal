//! Timers
use crate::rcc::*;
use crate::stm32::*;
use crate::time::{Hertz, MicroSecond};
use core::marker::PhantomData;
use fugit::HertzU32;
use void::Void;

pub mod delay;
pub mod opm;
pub mod pins;
pub mod pwm;
pub mod qei;
pub mod stopwatch;

/// Hardware timers
pub struct Timer<TIM> {
    clk: Hertz,
    tim: TIM,
}

pub struct Channel<const N: usize>;

impl<const N: usize> Channel<N> {
    const N: usize = N;
}

type Channel1 = Channel<0>;
type Channel2 = Channel<1>;
type Channel3 = Channel<2>;
type Channel4 = Channel<3>;

pub struct TimerFrequencySettings {
    psc: u16,
    arr: u32,
}

impl TimerFrequencySettings {
    pub(crate) const fn from(target_freq: Hertz, clk: Hertz) -> Self {
        let ratio = clk.raw() / target_freq.raw();
        let psc = (ratio - 1) / 0xffff;
        let arr = ratio / (psc + 1) - 1;
        let psc = psc as u16;

        Self { psc, arr }
    }
}

pub trait TimerExt<TIM> {
    fn timer(self, rcc: &mut Rcc) -> Timer<TIM>;
}

pub(super) mod private {
    use crate::timer::MicroSecond;
    use cortex_m::peripheral::syst::SystClkSource;
    use fugit::HertzU32;
    use stm32g0::stm32g081::SYST;

    use super::{Rcc, TimerFrequencySettings};

    pub trait TimerCommon {
        fn init(&mut self, rcc: &mut Rcc);

        fn set_urs(&mut self);

        /// Starts listening
        fn listen(&mut self);

        /// Stops listening
        fn unlisten(&mut self);

        /// Gets timer counter current value
        fn get_current(&self) -> u32;

        fn start(&mut self, timeout: MicroSecond, clk: HertzU32);

        fn has_elapsed(&mut self) -> bool;

        fn clear_irq(&mut self);
    }

    impl TimerCommon for SYST {
        fn init(&mut self, _rcc: &mut Rcc) {
            self.set_clock_source(SystClkSource::Core);
        }

        fn set_urs(&mut self) {}

        /// Starts listening
        fn listen(&mut self) {
            self.enable_interrupt()
        }

        /// Stops listening
        fn unlisten(&mut self) {
            self.disable_interrupt()
        }

        /// Gets timer counter current value
        fn get_current(&self) -> u32 {
            SYST::get_current()
        }

        fn start(&mut self, timeout: MicroSecond, clk: HertzU32) {
            let cycles = crate::time::cycles(timeout, clk);
            assert!(cycles < 0x00ff_ffff);
            self.set_reload(cycles);
            self.clear_current();
            self.enable_counter();
        }

        /// NOTE This takes &mut self because the read operation maight be side effectful and might clear the bit of the read register for some timers (SYST).
        fn has_elapsed(&mut self) -> bool {
            self.has_wrapped()
        }

        fn clear_irq(&mut self) {}
    }

    pub trait TimerBase: TimerCommon {
        /// Pauses timer
        fn pause(&mut self);

        /// Resumes timer
        fn resume(&mut self);

        fn set_freq_settings(&mut self, freq_settings: TimerFrequencySettings);

        fn set_freq(&mut self, target_freq: HertzU32, clk: HertzU32);

        /// Resets counter value
        fn reset(&mut self);

        /// Returns the currently configured frequency
        fn freq(&self, clk: HertzU32) -> HertzU32;

        /// Generate an update event so that PSC and ARR values are copied into their
        /// shadow registers.
        fn force_update(&mut self);
    }
}

macro_rules! timers {
    ($($TIM:ident: ($tim:ident, $cnt:ident $(,$cnt_h:ident)*),)+) => {
        $(
            impl private::TimerCommon for $TIM {
                fn init(&mut self, rcc: &mut Rcc) {
                    $TIM::enable(rcc);
                    $TIM::reset(rcc);
                }

                fn set_urs(&mut self) {
                    // Set URS so that when we force_update, it will
                    // generate an update event *without* triggering an interrupt.
                    self.cr1().modify(|_, w| w.cen().clear_bit().urs().set_bit());
                }

                /// Starts listening
                fn listen(&mut self) {
                    self.dier().write(|w| w.uie().set_bit());
                }

                /// Stops listening
                fn unlisten(&mut self) {
                    self.dier().write(|w| w.uie().clear_bit());
                }

                /// Gets timer counter current value
                fn get_current(&self) -> u32 {
                    let _high = 0;
                    $(
                        let _high = self.cnt().read().$cnt_h().bits() as u32;
                    )*
                    let low = self.cnt().read().$cnt().bits() as u32;
                    low | (_high << 16)
                }

                fn start(&mut self, timeout: MicroSecond, clk: HertzU32) {
                    use private::TimerBase;

                    // Pause the counter.
                    TimerBase::pause(self);
                    // reset counter
                    TimerBase::reset(self);
                    // clear interrupt flag
                    self.clear_irq();

                    // Calculate counter configuration
                    let cycles = crate::time::cycles(timeout, clk);
                    let psc = cycles / 0xffff;
                    let arr = cycles / (psc + 1);
                    let psc = psc as u16;

                    TimerBase::set_freq_settings(self, TimerFrequencySettings { psc, arr });

                    // Generate an update event so that PSC and ARR values are copied into their
                    // shadow registers.
                    TimerBase::force_update(self);

                    TimerBase::resume(self);
                }

                fn has_elapsed(&mut self) -> bool {
                    self.sr().read().uif().bit_is_set()
                }

                /// Clears interrupt flag
                fn clear_irq(&mut self) {
                    self.sr().modify(|_, w| w.uif().clear_bit());
                }
            }

            impl private::TimerBase for $TIM {
                /// Pauses timer
                fn pause(&mut self) {
                    self.cr1().modify(|_, w| w.cen().clear_bit());
                }

                /// Resumes timer
                fn resume(&mut self) {
                    self.cr1().modify(|_, w| w.cen().set_bit());
                }

                fn set_freq_settings(&mut self, freq_settings: TimerFrequencySettings) {
                    unsafe {
                        self.psc().write(|w| w.psc().bits(freq_settings.psc as u16));
                        self.arr().write(|w| w.arr().bits((freq_settings.arr as u16).into()));
                    }
                }

                fn set_freq(&mut self, target_freq: Hertz, clk: Hertz) {
                    let freq_settings = TimerFrequencySettings::from(target_freq, clk);

                    self.set_freq_settings(freq_settings);
                }

                /// Resets counter value
                fn reset(&mut self) {
                    self.cnt().reset();
                }

                /// Returns the currently configured frequency
                fn freq(&self, clk: HertzU32) -> Hertz {
                    Hertz::from_raw(clk.raw()
                        / (self.psc().read().bits() + 1)
                        / (self.arr().read().bits() + 1))
                }

                fn force_update(&mut self) {
                    // Generate an update event so that PSC and ARR values are copied into their
                    // shadow registers.
                    self.egr().write(|w| w.ug().set_bit());
                }
            }
        )+
    }
}

impl<T: private::TimerCommon> TimerExt<T> for T {
    fn timer(self, rcc: &mut Rcc) -> Timer<T> {
        Timer::new(self, rcc)
    }
}

impl<T: private::TimerCommon> Timer<T> {
    /// Configures a TIM peripheral as a periodic count down timer
    pub fn new(mut tim: T, rcc: &mut Rcc) -> Self {
        tim.init(rcc);

        tim.set_urs();

        Timer {
            tim,
            clk: rcc.clocks.apb_tim_clk,
        }
    }

    /// Starts listening
    pub fn listen(&mut self) {
        self.tim.listen();
    }

    /// Stops listening
    pub fn unlisten(&mut self) {
        self.tim.unlisten();
    }

    /// Gets timer counter current value
    pub fn get_current(&self) -> u32 {
        self.tim.get_current()
    }

    pub fn wait(&mut self) -> nb::Result<(), Void> {
        if !self.tim.has_elapsed() {
            Err(nb::Error::WouldBlock)
        } else {
            self.tim.clear_irq();
            Ok(())
        }
    }

    /// Releases the TIM peripheral
    pub fn release(self) -> T {
        self.tim
    }
}

impl<T: private::TimerBase> Timer<T> {
    /// Pauses timer
    pub fn pause(&mut self) {
        self.tim.pause();
    }

    /// Resumes timer
    pub fn resume(&mut self) {
        self.tim.resume();
    }

    /// Clears interrupt flag
    pub fn clear_irq(&mut self) {
        self.tim.clear_irq();
    }

    /// Resets counter value
    pub fn reset(&mut self) {
        self.tim.reset();
    }
}

#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ExternalClockMode {
    Mode1,
    Mode2,
}

pub trait ExternalClock {
    fn mode(&self) -> ExternalClockMode;
}

macro_rules! timers_external_clocks {
    ($($TIM:ident: ($tim:ident, $sms:ident $(,$ece:ident)*),)+) => {
        $(
            impl Timer<$TIM> {
                pub fn use_external_clock<C: ExternalClock>(&mut self, clk: C, freq: Hertz) {
                    self.clk = freq;
                    match clk.mode() {
                        ExternalClockMode::Mode1 => {
                            self.tim.smcr().modify(|_, w| unsafe { w.$sms().bits(0b111) });
                            $(
                                self.tim.smcr().modify(|_, w| w.$ece().clear_bit());
                            )*
                        },
                        ExternalClockMode::Mode2 => {
                            self.tim.smcr().modify(|_, w| unsafe { w.$sms().bits(0b0) });
                            $(
                                self.tim.smcr().modify(|_, w| w.$ece().set_bit());
                            )*
                        },
                    }
                }
            }
        )+
    }
}

#[cfg(not(any(feature = "stm32g0b1", feature = "stm32g0c1")))]
timers_external_clocks! {
    TIM1: (tim1, sms, ece),
    TIM3: (tim3, sms, ece),
}

#[cfg(any(feature = "stm32g0b1", feature = "stm32g0c1"))]
timers_external_clocks! {
    TIM1: (tim1, sms1, ece),
    TIM2: (tim2, sms1, ece),
    TIM3: (tim3, sms1, ece),
}

#[cfg(not(any(feature = "stm32g0b1", feature = "stm32g0c1")))]
#[cfg(feature = "stm32g0x1")]
timers_external_clocks! {
    TIM2: (tim2, sms, ece),
}

#[cfg(any(
    feature = "stm32g070",
    feature = "stm32g071",
    feature = "stm32g0b1",
    feature = "stm32g0c1"
))]
timers_external_clocks! {
    TIM15: (tim15, sms1),
}

timers! {
    TIM1: (tim1, cnt),
    TIM3: (tim3, cnt),
    TIM14: (tim14, cnt),
    TIM16: (tim16, cnt),
    TIM17: (tim17, cnt),
}

#[cfg(feature = "stm32g0x1")]
timers! {
    TIM2: (tim2, cnt),
}

#[cfg(any(feature = "stm32g070", feature = "stm32g071", feature = "stm32g081"))]
timers! {
    TIM6: (tim6, cnt),
    TIM7: (tim7, cnt),
    TIM15: (tim15, cnt),
}
