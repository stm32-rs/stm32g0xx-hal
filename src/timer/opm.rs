//! # One-pulse Mode
use crate::prelude::*;
use crate::rcc::*;
use crate::stm32::*;
use crate::time::{Hertz, MicroSecond};
use crate::timer::pins::TimerPin;
use crate::timer::*;
use core::marker::PhantomData;

pub trait OpmExt: Sized {
    fn opm(self, period: MicroSecond, rcc: &mut Rcc) -> Opm<Self>;
}

pub struct OpmPin<TIM, CH> {
    tim: PhantomData<TIM>,
    channel: PhantomData<CH>,
    delay: u32,
}

pub struct Opm<TIM> {
    tim: PhantomData<TIM>,
    clk: Hertz,
}

impl<TIM> Opm<TIM> {
    pub fn bind_pin<PIN>(&self, pin: PIN) -> OpmPin<TIM, PIN::Channel>
    where
        PIN: TimerPin<TIM>,
    {
        pin.setup();
        OpmPin {
            tim: PhantomData,
            channel: PhantomData,
            delay: 1,
        }
    }
}

macro_rules! opm {
    ($($TIMX:ident: ($timX:ident, $arr:ident $(,$arr_h:ident)*),)+) => {
        $(
            impl OpmExt for $TIMX {
                fn opm(self, pulse: MicroSecond, rcc: &mut Rcc) -> Opm<Self> {
                    $timX(self, pulse, rcc)
                }
            }

            fn $timX(_tim: $TIMX, pulse: MicroSecond, rcc: &mut Rcc) -> Opm<$TIMX> {
                $TIMX::enable(rcc);
                $TIMX::reset(rcc);

                let mut opm = Opm::<$TIMX> {
                    clk: rcc.clocks.apb_tim_clk,
                    tim: PhantomData,
                };
                opm.set_pulse(pulse);
                opm
            }

            impl Opm<$TIMX> {
                pub fn set_pulse(&mut self, pulse: MicroSecond) {
                    let cycles_per_period = self.clk / pulse.into();
                    let psc = (cycles_per_period - 1) / 0xffff;
                    let freq = (self.clk.0 / (psc + 1)).hz();
                    let reload = pulse.cycles(freq);
                    unsafe {
                        let tim = &*$TIMX::ptr();
                        tim.psc.write(|w| w.psc().bits(psc as u16));
                        tim.arr.write(|w| w.$arr().bits(reload as u16));
                        $(
                            tim.arr.modify(|_, w| w.$arr_h().bits((reload >> 16) as u16));
                        )*
                    }
                }

                pub fn generate(&mut self) {
                    let tim =  unsafe {&*$TIMX::ptr()};
                    tim.cr1.write(|w| w.opm().set_bit().cen().set_bit());
                }
            }
        )+
    }
}

macro_rules! opm_hal {
    ($($TIMX:ident:
        ($CH:ty, $ccxe:ident, $ccmrx_output:ident, $ocxm:ident, $ocxfe:ident, $ccrx:ident),)+
    ) => {
        $(
            impl OpmPin<$TIMX, $CH> {
                pub fn enable(&mut self) {
                    let tim =  unsafe {&*$TIMX::ptr()};
                    tim.ccer.modify(|_, w| w.$ccxe().set_bit());
                    self.setup();
                }

                pub fn disable(&mut self) {
                    let tim =  unsafe {&*$TIMX::ptr()};
                    tim.ccer.modify(|_, w| w.$ccxe().clear_bit());
                }

                pub fn get_max_delay(&mut self) -> u32 {
                    unsafe { (*$TIMX::ptr()).arr.read().bits() }
                }

                pub fn set_delay(&mut self, delay: u32) {
                    self.delay = delay;
                    self.setup();
                }

                fn setup(&mut self) {
                    unsafe {
                        let tim = &*$TIMX::ptr();
                        tim.$ccrx.write(|w| w.bits(self.delay));
                        tim.$ccmrx_output().modify(|_, w| w.$ocxm().bits(7).$ocxfe().set_bit());
                    }
                }
            }
        )+
    };
}

opm_hal! {
    TIM1: (Channel1, cc1e, ccmr1_output, oc1m, oc1fe, ccr1),
    TIM1: (Channel2, cc2e, ccmr1_output, oc2m, oc2fe, ccr2),
    TIM1: (Channel3, cc3e, ccmr2_output, oc3m, oc3fe, ccr3),
    TIM1: (Channel4, cc4e, ccmr2_output, oc4m, oc4fe, ccr4),
    TIM3: (Channel1, cc1e, ccmr1_output, oc1m, oc1fe, ccr1),
    TIM3: (Channel2, cc2e, ccmr1_output, oc2m, oc2fe, ccr2),
    TIM3: (Channel3, cc3e, ccmr2_output, oc3m, oc3fe, ccr3),
    TIM3: (Channel4, cc4e, ccmr2_output, oc4m, oc4fe, ccr4),
    TIM14: (Channel1, cc1e, ccmr1_output, oc1m, oc1fe, ccr1),
    TIM16: (Channel1, cc1e, ccmr1_output, oc1m, oc1fe, ccr1),
    TIM17: (Channel1, cc1e, ccmr1_output, oc1m, oc1fe, ccr1),
}

#[cfg(feature = "stm32g0x1")]
opm_hal! {
    TIM2: (Channel1, cc1e, ccmr1_output, oc1m, oc1fe, ccr1),
    TIM2: (Channel2, cc2e, ccmr1_output, oc2m, oc2fe, ccr2),
    TIM2: (Channel3, cc3e, ccmr2_output, oc3m, oc3fe, ccr3),
    TIM2: (Channel4, cc4e, ccmr2_output, oc4m, oc4fe, ccr4),
}

opm! {
    TIM1: (tim1, arr),
    TIM3: (tim3, arr_l, arr_h),
    TIM14: (tim14, arr),
    TIM16: (tim16, arr),
    TIM17: (tim17, arr),
}

#[cfg(feature = "stm32g0x1")]
opm! {
    TIM2: (tim2, arr_l, arr_h),
}

#[cfg(any(feature = "stm32g070", feature = "stm32g071", feature = "stm32g081"))]
opm! {
    TIM15: (tim15, arr),
}
