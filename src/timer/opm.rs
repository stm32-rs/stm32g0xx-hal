//! # One-pulse Mode
use crate::prelude::*;
use crate::rcc::Rcc;
use crate::stm32::*;
use crate::time::{Hertz, MicroSecond};
use crate::timer::pins::TimerPin;
use crate::timer::*;
use core::marker::PhantomData;

pub trait OpmExt: Sized {
    fn opm<PIN>(self, _: PIN, pulse_width: MicroSecond, rcc: &mut Rcc) -> Opm<Self, PIN::Channel>
    where
        PIN: TimerPin<Self>;
}

pub struct Opm<TIM, CHANNEL> {
    rb: TIM,
    clk: Hertz,
    pulse_width: MicroSecond,
    delay: MicroSecond,
    _channel: PhantomData<CHANNEL>,
}

macro_rules! opm {
    ($($TIMX:ident: ($apbXenr:ident, $apbXrstr:ident, $timX:ident, $timXen:ident, $timXrst:ident),)+) => {
        $(
            impl OpmExt for $TIMX {
                fn opm<PIN>(self, pin: PIN, pulse_width: MicroSecond, rcc: &mut Rcc) -> Opm<Self, PIN::Channel>
                where
                    PIN: TimerPin<Self>
                {
                    $timX(self, pin, pulse_width, rcc)
                }
            }

            fn $timX<PIN>(tim: $TIMX, pin: PIN, pulse_width: MicroSecond, rcc: &mut Rcc) -> Opm<$TIMX, PIN::Channel>
            where
                PIN: TimerPin<$TIMX>,
            {
                pin.setup();
                rcc.rb.$apbXenr.modify(|_, w| w.$timXen().set_bit());
                rcc.rb.$apbXrstr.modify(|_, w| w.$timXrst().set_bit());
                rcc.rb.$apbXrstr.modify(|_, w| w.$timXrst().clear_bit());
                Opm {
                    rb: tim,
                    clk: rcc.clocks.apb_tim_clk,
                    pulse_width,
                    delay: 0.us(),
                    _channel: PhantomData,
                }
            }
        )+
    }
}

macro_rules! opm_hal {
    ($($TIMX:ident:
        ($CH:ty, $ccxe:ident, $ccmrx_output:ident, $ocxm:ident, $ocxfe:ident, $ccrx:ident, $arr:ident $(,$arr_h:ident)*),)+
    ) => {
        $(
            impl Opm<$TIMX, $CH> {
                pub fn enable (&mut self) {
                    self.rb.ccer.modify(|_, w| w.$ccxe().set_bit());
                    self.setup();
                }

                pub fn disable (&mut self) {
                    self.rb.ccer.modify(|_, w| w.$ccxe().clear_bit());
                }

                pub fn generate(&mut self) {
                    self.rb.cr1.write(|w| w.opm().set_bit().cen().set_bit());
                }

                pub fn set_pulse_width<T> (&mut self, pulse_width: T)
                where
                    T: Into<MicroSecond>
                {
                    self.pulse_width = pulse_width.into();
                    self.setup();
                }

                pub fn set_delay<T> (&mut self, delay: T)
                where
                    T: Into<MicroSecond>
                {
                    self.delay = delay.into();
                    self.setup();
                }

                fn setup (&mut self) {
                    let period = self.pulse_width + self.delay;

                    let cycles_per_period = self.clk / period.into();
                    let psc = (cycles_per_period - 1) / 0xffff;

                    self.rb.psc.write(|w| unsafe { w.psc().bits(psc as u16) });
                    let freq = (self.clk.0 / (psc + 1)).hz();
                    let reload = cycles_per_period / (psc + 1);
                    let compare = if self.delay.0 > 0 {
                        self.delay.cycles(freq)
                    } else {
                        1
                    };
                    unsafe {
                        self.rb.arr.write(|w| w.$arr().bits(reload as u16));
                        self.rb.$ccrx.write(|w| w.bits(compare));
                        $(
                            self.rb.arr.modify(|_, w| w.$arr_h().bits((reload >> 16) as u16));
                        )*
                        self.rb.$ccmrx_output().modify(|_, w| w.$ocxm().bits(7).$ocxfe().set_bit());
                    }
                }
            }
        )+
    };
}

opm_hal! {
    TIM1: (Channel1, cc1e, ccmr1_output, oc1m, oc1fe, ccr1, arr),
    TIM1: (Channel2, cc2e, ccmr1_output, oc2m, oc2fe, ccr2, arr),
    TIM1: (Channel3, cc3e, ccmr2_output, oc3m, oc3fe, ccr3, arr),
    TIM1: (Channel4, cc4e, ccmr2_output, oc4m, oc4fe, ccr4, arr),
    TIM3: (Channel1, cc1e, ccmr1_output, oc1m, oc1fe, ccr1, arr_l, arr_h),
    TIM3: (Channel2, cc2e, ccmr1_output, oc2m, oc2fe, ccr2, arr_l, arr_h),
    TIM3: (Channel3, cc3e, ccmr2_output, oc3m, oc3fe, ccr3, arr_l, arr_h),
    TIM3: (Channel4, cc4e, ccmr2_output, oc4m, oc4fe, ccr4, arr_l, arr_h),
    TIM14: (Channel1, cc1e, ccmr1_output, oc1m, oc1fe, ccr1, arr),
    TIM16: (Channel1, cc1e, ccmr1_output, oc1m, oc1fe, ccr1, arr),
    TIM17: (Channel1, cc1e, ccmr1_output, oc1m, oc1fe, ccr1, arr),
}

#[cfg(feature = "stm32g0x1")]
opm_hal! {
    TIM2: (Channel1, cc1e, ccmr1_output, oc1m, oc1fe, ccr1, arr_l, arr_h),
    TIM2: (Channel2, cc2e, ccmr1_output, oc2m, oc2fe, ccr2, arr_l, arr_h),
    TIM2: (Channel3, cc3e, ccmr2_output, oc3m, oc3fe, ccr3, arr_l, arr_h),
    TIM2: (Channel4, cc4e, ccmr2_output, oc4m, oc4fe, ccr4, arr_l, arr_h),
}

#[cfg(any(feature = "stm32g070", feature = "stm32g071", feature = "stm32g081"))]
opm_hal! {
    TIM15: (Channel1, cc1e, ccmr1_output, oc1m, oc1fe, ccr1, arr),
    TIM15: (Channel2, cc2e, ccmr1_output, oc2m, oc2fe, ccr2, arr),
}

opm! {
    TIM1: (apbenr2, apbrstr2, tim1, tim1en, tim1rst),
    TIM3: (apbenr1, apbrstr1, tim3, tim3en, tim3rst),
    TIM14: (apbenr2, apbrstr2, tim14, tim14en, tim14rst),
    TIM16: (apbenr2, apbrstr2, tim16, tim16en, tim16rst),
    TIM17: (apbenr2, apbrstr2, tim17, tim17en, tim17rst),
}

#[cfg(feature = "stm32g0x1")]
opm! {
    TIM2: (apbenr1, apbrstr1, tim2, tim2en, tim2rst),
}

#[cfg(any(feature = "stm32g070", feature = "stm32g071", feature = "stm32g081"))]
opm! {
    TIM15: (apbenr2, apbrstr2, tim15, tim15en, tim15rst),
}
