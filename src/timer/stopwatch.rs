use crate::rcc::Rcc;
use crate::stm32::*;
use crate::time::{Hertz, Instant, MicroSecond};

pub trait StopwatchExt<TIM> {
    fn stopwatch(self, rcc: &mut Rcc) -> Stopwatch<TIM>;
}

pub struct Stopwatch<TIM> {
    clk: Hertz,
    tim: TIM,
}

macro_rules! stopwatches {
    ($($TIM:ident: ($tim:ident, $timXen:ident, $timXrst:ident, $apbenr:ident, $apbrstr:ident ),)+) => {
        $(
            impl Stopwatch<$TIM> {
                pub fn $tim(tim: $TIM, rcc: &mut Rcc) -> Self {
                    assert!(rcc.clocks.apb_tim_clk.0 > 1_000_000);
                    rcc.rb.$apbenr.modify(|_, w| w.$timXen().set_bit());
                    rcc.rb.$apbrstr.modify(|_, w| w.$timXrst().set_bit());
                    rcc.rb.$apbrstr.modify(|_, w| w.$timXrst().clear_bit());
                    tim.cr1.modify(|_, w| w.cen().set_bit());
                    Stopwatch {
                        tim,
                        clk: rcc.clocks.apb_tim_clk,
                    }
                }

                /// Overrides the counter clock input frequency
                ///
                /// Useful if the APB Timer Clock changes after the `Stopwatch` is created or
                /// to deliberately speed up or slow down the `Stopwatch` from actual measured time.
                pub fn set_clock<T>(&mut self, clk: T)
                where
                    T: Into<Hertz>,
                {
                    let clk = clk.into();
                    assert!(clk.0 > 1_000_000);
                    self.clk = clk;
                }

                /// Set the prescaler which divides the input clock frequency before the counter
                ///
                /// The counter frequency is equal to the input clock divided by the prescaler + 1.
                pub fn set_prescaler(&mut self, prescaler: u16) {
                    self.tim.psc.write(|w| unsafe { w.psc().bits(prescaler) });
                    self.tim.egr.write(|w| w.ug().set_bit());
                }

                pub fn reset(&mut self) {
                    self.tim.cnt.reset();
                }

                pub fn pause(&mut self) {
                    self.tim.cr1.modify(|_, w| w.cen().clear_bit());
                }

                pub fn resume(&mut self) {
                    self.tim.cr1.modify(|_, w| w.cen().set_bit());
                }

                pub fn release(self) -> $TIM {
                    self.tim
                }

                pub fn now(&self) -> Instant {
                    Instant(self.tim.cnt.read().bits())
                }

                pub fn elapsed(&self, ts: Instant) -> MicroSecond {
                    let now = self.now().0;
                    let cycles = now.wrapping_sub(ts.0);
                    self.clk.duration(cycles)
                }

                pub fn trace<F>(&self, mut closure: F) -> MicroSecond
                where
                    F: FnMut(),
                {
                    let started = self.now().0;
                    closure();
                    let now = self.now().0;
                    self.clk.duration(now.wrapping_sub(started))
                }
            }

            impl StopwatchExt<$TIM> for $TIM {
                fn stopwatch(self, rcc: &mut Rcc) -> Stopwatch<$TIM> {
                    Stopwatch::$tim(self, rcc)
                }
            }
        )+
    }
}

stopwatches! {
    TIM1: (tim1, tim1en, tim1rst, apbenr2, apbrstr2),
    TIM3: (tim3, tim3en, tim3rst, apbenr1, apbrstr1),
    TIM14: (tim14, tim14en, tim14rst, apbenr2, apbrstr2),
    TIM16: (tim16, tim16en, tim16rst, apbenr2, apbrstr2),
    TIM17: (tim17, tim17en, tim17rst, apbenr2, apbrstr2),
}

#[cfg(feature = "stm32g0x1")]
stopwatches! {
    TIM2: (tim2, tim2en, tim2rst, apbenr1, apbrstr1),
}

#[cfg(any(feature = "stm32g070", feature = "stm32g071", feature = "stm32g081"))]
stopwatches! {
    TIM6: (tim6, tim6en, tim6rst, apbenr1, apbrstr1),
    TIM7: (tim7, tim7en, tim7rst, apbenr1, apbrstr1),
    TIM15: (tim15, tim15en, tim15rst, apbenr2, apbrstr2),
}
