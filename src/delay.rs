//! Delays
use core::cmp;
use cortex_m::peripheral::SYST;
use hal::blocking::delay::{DelayMs, DelayUs};

use crate::prelude::*;
use crate::rcc::Rcc;
use crate::stm32::*;
use crate::time::{Hertz, MicroSecond};

/// Delay provider
pub struct Delay<TIM> {
    clk: Hertz,
    tim: TIM,
}

pub trait DelayExt<TIM> {
    fn delay(self, rcc: &Rcc) -> Delay<TIM>;
}

impl Delay<SYST> {
    /// Configures the system timer (SysTick) as a delay provider
    pub fn syst(syst: SYST, rcc: &Rcc) -> Self {
        Delay {
            tim: syst,
            clk: rcc.clocks.core_clk,
        }
    }

    pub fn delay<T>(&mut self, delay: T)
    where
        T: Into<MicroSecond>,
    {
        let mut cycles = delay.into().cycles(self.clk);
        while cycles > 0 {
            let reload = cmp::min(cycles, 0x00ff_ffff);
            cycles -= reload;
            self.tim.set_reload(reload);
            self.tim.clear_current();
            self.tim.enable_counter();
            while !self.tim.has_wrapped() {}
            self.tim.disable_counter();
        }
    }

    /// Releases the system timer (SysTick) resource
    pub fn release(self) -> SYST {
        self.tim
    }
}

impl DelayUs<u32> for Delay<SYST> {
    fn delay_us(&mut self, us: u32) {
        self.delay(us.us())
    }
}

impl DelayUs<u16> for Delay<SYST> {
    fn delay_us(&mut self, us: u16) {
        self.delay_us(us as u32)
    }
}

impl DelayUs<u8> for Delay<SYST> {
    fn delay_us(&mut self, us: u8) {
        self.delay_us(us as u32)
    }
}

impl DelayMs<u32> for Delay<SYST> {
    fn delay_ms(&mut self, ms: u32) {
        self.delay_us(ms.saturating_mul(1_000));
    }
}

impl DelayMs<u16> for Delay<SYST> {
    fn delay_ms(&mut self, ms: u16) {
        self.delay_ms(ms as u32);
    }
}

impl DelayMs<u8> for Delay<SYST> {
    fn delay_ms(&mut self, ms: u8) {
        self.delay_ms(ms as u32);
    }
}

impl DelayExt<SYST> for SYST {
    fn delay(self, rcc: &Rcc) -> Delay<SYST> {
        Delay::syst(self, rcc)
    }
}

macro_rules! delays {
    ($($TIM:ident: ($tim:ident, $timXen:ident, $timXrst:ident, $apbenr:ident, $apbrstr:ident),)+) => {
        $(
            impl Delay<$TIM> {
                /// Configures $TIM timer as a delay provider
                pub fn $tim(tim: $TIM, rcc: &Rcc) -> Self {
                    rcc.rb.$apbenr.modify(|_, w| w.$timXen().set_bit());
                    rcc.rb.$apbrstr.modify(|_, w| w.$timXrst().set_bit());
                    rcc.rb.$apbrstr.modify(|_, w| w.$timXrst().clear_bit());
                    Delay {
                        tim,
                        clk: rcc.clocks.apb_tim_clk,
                    }
                }

                pub fn delay<T>(&mut self, delay: T)
                where
                    T: Into<MicroSecond>,
                {
                    let mut cycles = delay.into().cycles(self.clk);
                    while cycles > 0 {
                        let reload = cmp::min(cycles, 0xffff);
                        cycles -= reload;
                        self.tim.arr.write(|w| unsafe { w.bits(reload) });
                        self.tim.cnt.reset();
                        self.tim.cr1.modify(|_, w| w.cen().set_bit().urs().set_bit());
                        while self.tim.sr.read().uif().bit_is_clear() {}
                        self.tim.sr.modify(|_, w| w.uif().clear_bit());
                        self.tim.cr1.modify(|_, w| w.cen().clear_bit());
                    }
                }

                pub fn release(self) -> $TIM {
                    self.tim
                }
            }

            impl DelayUs<u32> for Delay<$TIM> {
                fn delay_us(&mut self, us: u32) {
                    self.delay(us.us())
                }
            }

            impl DelayUs<u16> for Delay<$TIM> {
                fn delay_us(&mut self, us: u16) {
                    self.delay_us(us as u32)
                }
            }

            impl DelayUs<u8> for Delay<$TIM> {
                fn delay_us(&mut self, us: u8) {
                    self.delay_us(us as u32)
                }
            }

            impl DelayMs<u32> for Delay<$TIM> {
                fn delay_ms(&mut self, ms: u32) {
                    self.delay_us(ms.saturating_mul(1_000));
                }
            }

            impl DelayMs<u16> for Delay<$TIM> {
                fn delay_ms(&mut self, ms: u16) {
                    self.delay_ms(ms as u32);
                }
            }

            impl DelayMs<u8> for Delay<$TIM> {
                fn delay_ms(&mut self, ms: u8) {
                    self.delay_ms(ms as u32);
                }
            }

            impl DelayExt<$TIM> for $TIM {
                fn delay(self, rcc: &Rcc) -> Delay<$TIM> {
                    Delay::$tim(self, rcc)
                }
            }
        )+
    }
}

delays! {
    TIM1: (tim1, tim1en, tim1rst, apbenr2, apbrstr2),
    TIM2: (tim2, tim2en, tim2rst, apbenr1, apbrstr1),
    TIM3: (tim3, tim3en, tim3rst, apbenr1, apbrstr1),
    TIM14: (tim14, tim14en, tim14rst, apbenr2, apbrstr2),
    TIM16: (tim16, tim16en, tim16rst, apbenr2, apbrstr2),
    TIM17: (tim17, tim17en, tim17rst, apbenr2, apbrstr2),
}

#[cfg(any(feature = "stm32g07x", feature = "stm32g081"))]
delays! {
    TIM6: (tim6, tim6en, tim6rst, apbenr1, apbrstr1),
    TIM7: (tim7, tim7en, tim7rst, apbenr1, apbrstr1),
    TIM15: (tim15, tim15en, tim15rst, apbenr2, apbrstr2),
}
