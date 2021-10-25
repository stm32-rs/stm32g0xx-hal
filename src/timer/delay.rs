//! Delays
use core::cmp;
use cortex_m::peripheral::SYST;
use hal::blocking::delay::{DelayMs, DelayUs};

use crate::prelude::*;
use crate::rcc::*;
use crate::stm32::*;
use crate::time::{Hertz, MicroSecond};

/// Delay provider
pub struct Delay<TIM> {
    clk: Hertz,
    tim: TIM,
}

pub trait DelayExt<TIM> {
    fn delay(self, rcc: &mut Rcc) -> Delay<TIM>;
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
    fn delay(self, rcc: &mut Rcc) -> Delay<SYST> {
        Delay::syst(self, rcc)
    }
}

macro_rules! delays {
    ($($TIM:ident: $tim:ident,)+) => {
        $(
            impl Delay<$TIM> {
                /// Configures $TIM timer as a delay provider
                pub fn $tim(tim: $TIM, rcc: &mut Rcc) -> Self {
                    $TIM::enable(rcc);
                    $TIM::reset(rcc);

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
                fn delay(self, rcc: &mut Rcc) -> Delay<$TIM> {
                    Delay::$tim(self, rcc)
                }
            }
        )+
    }
}

delays! {
    TIM1: tim1,
    TIM3: tim3,
    TIM14: tim14,
    TIM16: tim16,
    TIM17: tim17,
}

#[cfg(feature = "stm32g0x1")]
delays! {
    TIM2: tim2,
}

#[cfg(any(feature = "stm32g070", feature = "stm32g071", feature = "stm32g081"))]
delays! {
    TIM6: tim6,
    TIM7: tim7,
    TIM15: tim15,
}
