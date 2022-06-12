//! Timers
use crate::rcc::*;
use crate::stm32::*;
use crate::time::{Hertz, MicroSecond};
use cortex_m::peripheral::syst::SystClkSource;
use cortex_m::peripheral::SYST;
use hal::timer::{CountDown, Periodic};
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

pub struct Channel1;
pub struct Channel2;
pub struct Channel3;
pub struct Channel4;

/// System timer
impl Timer<SYST> {
    /// Configures the SYST clock as a periodic count down timer
    pub fn syst(mut syst: SYST, rcc: &mut Rcc) -> Self {
        syst.set_clock_source(SystClkSource::Core);
        Timer {
            tim: syst,
            clk: rcc.clocks.apb_tim_clk,
        }
    }

    /// Starts listening
    pub fn listen(&mut self) {
        self.tim.enable_interrupt()
    }

    /// Stops listening
    pub fn unlisten(&mut self) {
        self.tim.disable_interrupt()
    }

    pub fn get_current(&self) -> u32 {
        SYST::get_current()
    }
}

impl CountDown for Timer<SYST> {
    type Time = MicroSecond;

    fn start<T>(&mut self, timeout: T)
    where
        T: Into<MicroSecond>,
    {
        let cycles = timeout.into().cycles(self.clk);
        assert!(cycles < 0x00ff_ffff);
        self.tim.set_reload(cycles);
        self.tim.clear_current();
        self.tim.enable_counter();
    }

    fn wait(&mut self) -> nb::Result<(), Void> {
        if self.tim.has_wrapped() {
            Ok(())
        } else {
            Err(nb::Error::WouldBlock)
        }
    }
}

pub trait TimerExt<TIM> {
    fn timer(self, rcc: &mut Rcc) -> Timer<TIM>;
}

impl TimerExt<SYST> for SYST {
    fn timer(self, rcc: &mut Rcc) -> Timer<SYST> {
        Timer::syst(self, rcc)
    }
}

impl Periodic for Timer<SYST> {}

macro_rules! timers {
    ($($TIM:ident: ($tim:ident, $cnt:ident $(,$cnt_h:ident)*),)+) => {
        $(
            impl Timer<$TIM> {
                /// Configures a TIM peripheral as a periodic count down timer
                pub fn $tim(tim: $TIM, rcc: &mut Rcc) -> Self {
                    $TIM::enable(rcc);
                    $TIM::reset(rcc);

                    Timer {
                        tim,
                        clk: rcc.clocks.apb_tim_clk,
                    }
                }

                /// Pauses timer
                pub fn pause(&mut self) {
                    self.tim.cr1.modify(|_, w| w.cen().clear_bit());
                }

                /// Resumes timer
                pub fn resume(&mut self) {
                    self.tim.cr1.modify(|_, w| w.cen().set_bit());
                }

                /// Starts listening
                pub fn listen(&mut self) {
                    self.tim.dier.write(|w| w.uie().set_bit());
                }

                /// Stops listening
                pub fn unlisten(&mut self) {
                    self.tim.dier.write(|w| w.uie().clear_bit());
                }

                /// Clears interrupt flag
                pub fn clear_irq(&mut self) {
                    self.tim.sr.modify(|_, w| w.uif().clear_bit());
                }

                /// Resets counter value
                pub fn reset(&mut self) {
                    self.tim.cnt.reset();
                }

                /// Gets timer counter current value
                pub fn get_current(&self) -> u32 {
                    let _high = 0;
                    $(
                        let _high = self.tim.cnt.read().$cnt_h().bits() as u32;
                    )*
                    let low = self.tim.cnt.read().$cnt().bits() as u32;
                    low | (_high << 16)
                }

                /// Releases the TIM peripheral
                pub fn release(self) -> $TIM {
                    self.tim
                }
            }

            impl TimerExt<$TIM> for $TIM {
                fn timer(self, rcc: &mut Rcc) -> Timer<$TIM> {
                    Timer::$tim(self, rcc)
                }
            }

            impl CountDown for Timer<$TIM> {
                type Time = MicroSecond;

                fn start<T>(&mut self, timeout: T)
                where
                    T: Into<MicroSecond>,
                {
                    // Pause the counter. Also set URS so that when we set UG below, it will
                    // generate an update event *without* triggering an interrupt.
                    self.tim.cr1.modify(|_, w| w.cen().clear_bit().urs().set_bit());
                    // reset counter
                    self.tim.cnt.reset();
                    // clear interrupt flag
                    self.tim.sr.modify(|_, w| w.uif().clear_bit());

                    // Calculate counter configuration
                    let cycles = timeout.into().cycles(self.clk);
                    let psc = cycles / 0xffff;
                    let arr = cycles / (psc + 1);

                    self.tim.psc.write(|w| unsafe { w.psc().bits(psc as u16) });
                    self.tim.arr.write(|w| unsafe { w.bits(arr) });

                    // Generate an update event so that PSC and ARR values are copied into their
                    // shadow registers.
                    self.tim.egr.write(|w| w.ug().set_bit());

                    self.tim.cr1.modify(|_, w| w.cen().set_bit());
                }

                fn wait(&mut self) -> nb::Result<(), Void> {
                    if self.tim.sr.read().uif().bit_is_clear() {
                        Err(nb::Error::WouldBlock)
                    } else {
                        self.tim.sr.modify(|_, w| w.uif().clear_bit());
                        Ok(())
                    }
                }
            }

            impl Periodic for Timer<$TIM> {}
        )+
    }
}

timers! {
    TIM1: (tim1, cnt),
    TIM3: (tim3, cnt_l, cnt_h),
    TIM14: (tim14, cnt),
    TIM16: (tim16, cnt),
    TIM17: (tim17, cnt),
}

#[cfg(feature = "stm32g0x1")]
timers! {
    TIM2: (tim2, cnt_l, cnt_h),
}

#[cfg(any(feature = "stm32g070", feature = "stm32g071", feature = "stm32g081"))]
timers! {
    TIM6: (tim6, cnt),
    TIM7: (tim7, cnt),
    TIM15: (tim15, cnt),
}
