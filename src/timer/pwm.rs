//! # Pulse Width Modulation
use core::marker::PhantomData;

use crate::rcc::Rcc;
use crate::stm32::*;
use crate::time::Hertz;
use crate::timer::pins::TimerPin;
use crate::timer::*;
use hal;

pub struct Pwm<TIM> {
    tim: PhantomData<TIM>,
}

pub struct PwmPin<TIM, CH> {
   tim: PhantomData<TIM>,
   channel: PhantomData<CH>,
}

pub trait PwmExt: Sized {
    fn pwm<T>(self, freq: T, rcc: &mut Rcc) -> Pwm<Self>
    where
        T: Into<Hertz>;
}

impl<TIM> Pwm<TIM> {
    pub fn bind_pin<PIN>(&self, pin: PIN) -> PwmPin<TIM, PIN::Channel>
    where
        PIN: TimerPin<TIM>
    {
        pin.setup();
        PwmPin {
            tim: PhantomData,
            channel: PhantomData,
        }
    }
}

macro_rules! pwm {
    ($($TIMX:ident: ($apbXenr:ident, $apbXrstr:ident, $timX:ident, $timXen:ident, $timXrst:ident, $arr:ident $(,$arr_h:ident)*),)+) => {
        $(
            impl PwmExt for $TIMX {
                fn pwm<T>(self, freq: T, rcc: &mut Rcc) -> Pwm<Self>
                where
                    T: Into<Hertz>,
                {
                    $timX(self, freq, rcc)
                }
            }

            fn $timX<T>(tim: $TIMX, freq: T, rcc: &mut Rcc) -> Pwm<$TIMX>
            where
                T: Into<Hertz>,
            {
                rcc.rb.$apbXenr.modify(|_, w| w.$timXen().set_bit());
                rcc.rb.$apbXrstr.modify(|_, w| w.$timXrst().set_bit());
                rcc.rb.$apbXrstr.modify(|_, w| w.$timXrst().clear_bit());
                let ratio = rcc.clocks.apb_tim_clk / freq.into();
                let psc = (ratio - 1) / 0xffff;
                let arr = ratio / (psc + 1);
                tim.psc.write(|w| unsafe { w.psc().bits(psc as u16) });
                tim.arr.write(|w| unsafe { w.$arr().bits(arr as u16) });
                $(
                    tim.arr.modify(|_, w| unsafe { w.$arr_h().bits((arr >> 16) as u16) });
                )*
                tim.cr1.write(|w| w.cen().set_bit());
                Pwm {
                    tim: PhantomData
                }
            }
        )+
    }
}

macro_rules! pwm_hal {
    ($($TIMX:ident:
        ($CH:ty, $ccxe:ident, $ccmrx_output:ident, $ocxpe:ident, $ocxm:ident, $ccrx:ident, $ccrx_l:ident, $ccrx_h:ident),)+
    ) => {
        $(
            impl hal::PwmPin for PwmPin<$TIMX, $CH> {
                type Duty = u32;

                fn disable(&mut self) {
                    unsafe {
                        (*$TIMX::ptr()).ccer.modify(|_, w| w.$ccxe().clear_bit());
                    }
                }

                fn enable(&mut self) {
                    unsafe {
                        let tim = &*$TIMX::ptr();
                        tim.$ccmrx_output().modify(|_, w| w.$ocxpe().set_bit().$ocxm().bits(6));
                        tim.ccer.modify(|_, w| w.$ccxe().set_bit());
                    }
                }

                fn get_duty(&self) -> u32 {
                    unsafe { (*$TIMX::ptr()).$ccrx.read().bits() }
                }

                fn get_max_duty(&self) -> u32 {
                    unsafe { (*$TIMX::ptr()).arr.read().bits() }
                }

                fn set_duty(&mut self, duty: u32) {
                    unsafe { (*$TIMX::ptr()).$ccrx.write(|w| w.bits(duty)) }
                }
            }
        )+
    };

    ($($TIMX:ident:
        ($CH:ty, $ccxe:ident, $ccmrx_output:ident, $ocxpe:ident, $ocxm:ident, $ccrx:ident $(,$moe:ident)*),)+
    ) => {
        $(
            impl hal::PwmPin for PwmPin<$TIMX, $CH> {
                type Duty = u16;

                fn disable(&mut self) {
                    unsafe {
                        (*$TIMX::ptr()).ccer.modify(|_, w| w.$ccxe().clear_bit());
                    }
                }

                fn enable(&mut self) {
                    unsafe {
                        let tim = &*$TIMX::ptr();
                        tim.$ccmrx_output().modify(|_, w| w.$ocxpe().set_bit().$ocxm().bits(6));
                        tim.ccer.modify(|_, w| w.$ccxe().set_bit());
                        $(
                            tim.bdtr.modify(|_, w| w.$moe().set_bit());
                        )*
                    }
                }

                fn get_duty(&self) -> u16 {
                    unsafe { (*$TIMX::ptr()).$ccrx.read().$ccrx().bits() }
                }

                fn get_max_duty(&self) -> u16 {
                    unsafe { (*$TIMX::ptr()).arr.read().arr().bits() }
                }

                fn set_duty(&mut self, duty: u16) {
                    unsafe { (*$TIMX::ptr()).$ccrx.write(|w| w.$ccrx().bits(duty)) }
                }
            }
        )+
    };
}

pwm_hal! {
    TIM1:  (Channel1, cc1e, ccmr1_output, oc1pe, oc1m, ccr1, moe),
    TIM1:  (Channel2, cc2e, ccmr1_output, oc2pe, oc2m, ccr2, moe),
    TIM1:  (Channel3, cc3e, ccmr2_output, oc3pe, oc3m, ccr3, moe),
    TIM1:  (Channel4, cc4e, ccmr2_output, oc4pe, oc4m, ccr4, moe),
    TIM14: (Channel1, cc1e, ccmr1_output, oc1pe, oc1m, ccr1),
    TIM16: (Channel1, cc1e, ccmr1_output, oc1pe, oc1m, ccr1, moe),
    TIM17: (Channel1, cc1e, ccmr1_output, oc1pe, oc1m, ccr1, moe),
}

pwm_hal! {
    TIM2: (Channel1, cc1e, ccmr1_output, oc1pe, oc1m, ccr1, ccr1_l, ccr1_h),
    TIM2: (Channel2, cc2e, ccmr1_output, oc2pe, oc2m, ccr2, ccr2_l, ccr2_h),
    TIM2: (Channel3, cc3e, ccmr2_output, oc3pe, oc3m, ccr3, ccr3_l, ccr3_h),
    TIM2: (Channel4, cc4e, ccmr2_output, oc4pe, oc4m, ccr4, ccr4_l, ccr4_h),
    TIM3: (Channel1, cc1e, ccmr1_output, oc1pe, oc1m, ccr1, ccr1_l, ccr1_h),
    TIM3: (Channel2, cc2e, ccmr1_output, oc2pe, oc2m, ccr2, ccr2_l, ccr2_h),
    TIM3: (Channel3, cc3e, ccmr2_output, oc3pe, oc3m, ccr3, ccr3_l, ccr3_h),
    TIM3: (Channel4, cc4e, ccmr2_output, oc4pe, oc4m, ccr4, ccr4_l, ccr4_h),
}

#[cfg(any(feature = "stm32g07x", feature = "stm32g081"))]
pwm_hal! {
    TIM15: (Channel1, cc1e, ccmr1_output, oc1pe, oc1m, ccr1, moe),
}

pwm! {
    TIM1: (apbenr2, apbrstr2, tim1, tim1en, tim1rst, arr),
    TIM2: (apbenr1, apbrstr1, tim2, tim2en, tim2rst, arr_l, arr_h),
    TIM3: (apbenr1, apbrstr1, tim3, tim3en, tim3rst, arr_l, arr_h),
    TIM14: (apbenr2, apbrstr2, tim14, tim14en, tim14rst, arr),
    TIM16: (apbenr2, apbrstr2, tim16, tim16en, tim16rst, arr),
    TIM17: (apbenr2, apbrstr2, tim17, tim17en, tim17rst, arr),
}

#[cfg(any(feature = "stm32g07x", feature = "stm32g081"))]
pwm! {
    TIM15: (apbenr2, apbrstr2, tim15, tim15en, tim15rst, arr),
}
