use crate::rcc::*;
use crate::stm32::*;
use crate::time::{duration, Hertz, Instant, MicroSecond};

pub trait StopwatchExt<TIM> {
    fn stopwatch(self, rcc: &mut Rcc) -> Stopwatch<TIM>;
}

pub struct Stopwatch<TIM> {
    clk: Hertz,
    tim: TIM,
}

macro_rules! stopwatches {
    ($($TIM:ident: $tim:ident, $depth:ident,)+) => {
        $(
            impl Stopwatch<$TIM> {
                pub fn $tim(tim: $TIM, rcc: &mut Rcc) -> Self {
                    assert!(rcc.clocks.apb_tim_clk.raw() > 1_000_000);
                    $TIM::enable(rcc);
                    $TIM::reset(rcc);

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
                pub fn set_clock(&mut self, clk: Hertz) {
                    assert!(clk.raw() > 1_000_000);
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
                    Instant::from_ticks(self.tim.cnt.read().bits())
                }

                pub fn elapsed(&self, ts: Instant) -> MicroSecond {
                    let now = self.now().ticks();
                    let cycles = (now as $depth).wrapping_sub(ts.ticks() as $depth) as u32;
                    duration(self.clk, cycles * (1 + self.tim.psc.read().bits()))
                }

                pub fn trace<F>(&self, mut closure: F) -> MicroSecond
                where
                    F: FnMut(),
                {
                    let started = self.now().ticks();
                    closure();
                    let now = self.now().ticks();
                    duration(self.clk, now.wrapping_sub(started) * (1 + self.tim.psc.read().bits()))
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
    TIM1: tim1, u16,
    TIM3: tim3, u16,
    TIM14: tim14, u16,
    TIM16: tim16, u16,
    TIM17: tim17, u16,
}

#[cfg(feature = "stm32g0x1")]
stopwatches! {
    TIM2: tim2, u32,
}

#[cfg(any(feature = "stm32g070", feature = "stm32g071", feature = "stm32g081"))]
stopwatches! {
    TIM6: tim6, u16,
    TIM7: tim7, u16,
    TIM15: tim15, u16,
}
