//! # Pulse Width Modulation
use core::marker::PhantomData;

use crate::rcc::*;
use crate::stm32::*;
use crate::time::Hertz;
use crate::timer::pins::TimerPin;
use crate::timer::*;

pub enum OutputCompareMode {
    Frozen = 0,
    MatchPos = 1,
    MatchNeg = 2,
    MatchToggle = 3,
    ForceLow = 4,
    ForceHigh = 5,
    PwmMode1 = 6,
    PmwMode2 = 7,
    OpmMode1 = 8,
    OomMode2 = 9,
    CombinedMode1 = 12,
    CombinedMode2 = 13,
    AsyncMode1 = 14,
    AsyncMode2 = 15,
}

pub struct Pwm<TIM> {
    clk: Hertz,
    tim: TIM,
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

pub trait PwmPinMode {
    fn set_compare_mode(&mut self, mode: OutputCompareMode);
}

impl<TIM> Pwm<TIM> {
    pub fn bind_pin<PIN>(&self, pin: PIN) -> PwmPin<TIM, PIN::Channel>
    where
        PIN: TimerPin<TIM>,
    {
        pin.setup();
        PwmPin {
            tim: PhantomData,
            channel: PhantomData,
        }
    }
}

macro_rules! pwm {
    ($($TIMX:ident: ($timX:ident, $arr:ident $(,$arr_h:ident)*),)+) => {
        $(
            impl PwmExt for $TIMX {
                fn pwm<T>(self, freq: T, rcc: &mut Rcc) -> Pwm<Self>
                where
                    T: Into<Hertz>,
                {
                    $timX(self, freq, rcc)
                }
            }

            fn $timX<F: Into<Hertz>>(tim: $TIMX, freq: F, rcc: &mut Rcc) -> Pwm<$TIMX> {
                $TIMX::enable(rcc);
                $TIMX::reset(rcc);

                let mut pwm = Pwm::<$TIMX> {
                    clk: rcc.clocks.apb_tim_clk,
                    tim,
                };
                pwm.set_freq(freq);
                pwm
            }

            impl Pwm<$TIMX> {
                pub fn set_freq<F: Into<Hertz>>(&mut self, freq: F) {
                    let ratio = self.clk / freq.into();
                    let psc = (ratio - 1) / 0xffff;
                    let arr = ratio / (psc + 1) - 1;

                    unsafe {
                        self.tim.psc.write(|w| w.psc().bits(psc as u16));
                        self.tim.arr.write(|w| w.$arr().bits(arr as u16));
                        $(
                            self.tim.arr.modify(|_, w| w.$arr_h().bits((arr >> 16) as u16));
                        )*
                        self.tim.cr1.write(|w| w.cen().set_bit())
                    }
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
            }
        )+
    }
}

#[cfg(any(feature = "stm32g0x1", feature = "stm32g070"))]
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
}

macro_rules! pwm_advanced_hal {
    ($($TIMX:ident: (
        $CH:ty,
        $ccxe:ident $(: $ccxne:ident)*,
        $ccmrx_output:ident,
        $ocxpe:ident,
        $ocxm:ident,
        $ccrx:ident
        $(, $moe:ident)*
    ) ,)+
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
                            tim.ccer.modify(|_, w| w.$ccxne().bit(true));
                        )*
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

            impl PwmPinMode for PwmPin<$TIMX, $CH>{
                fn set_compare_mode(&mut self, mode: OutputCompareMode) {
                    unsafe {
                        let tim = &*$TIMX::ptr();
                        tim.$ccmrx_output().modify(|_, w| w.$ocxm().bits(mode as u8));
                    }
                }
            }
        )+
    };
}

pwm_advanced_hal! {
    TIM1:  (Channel1, cc1e: cc1ne, ccmr1_output, oc1pe, oc1m, ccr1, moe),
    TIM1:  (Channel2, cc2e: cc2ne, ccmr1_output, oc2pe, oc2m, ccr2, moe),
    TIM1:  (Channel3, cc3e: cc3ne, ccmr2_output, oc3pe, oc3m, ccr3, moe),
    TIM1:  (Channel4, cc4e, ccmr2_output, oc4pe, oc4m, ccr4, moe),
    TIM14: (Channel1, cc1e, ccmr1_output, oc1pe, oc1m, ccr1),
    TIM16: (Channel1, cc1e: cc1ne, ccmr1_output, oc1pe, oc1m, ccr1, moe),
    TIM17: (Channel1, cc1e: cc1ne, ccmr1_output, oc1pe, oc1m, ccr1, moe),
}

#[cfg(any(feature = "stm32g070", feature = "stm32g071", feature = "stm32g081"))]
pwm_advanced_hal! {
    TIM15: (Channel1, cc1e: cc1ne, ccmr1_output, oc1pe, oc1m, ccr1, moe),
}

#[cfg(feature = "stm32g0x1")]
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

#[cfg(feature = "stm32g070")]
pwm_hal! {
    TIM3: (Channel1, cc1e, ccmr1_output, oc1pe, oc1m, ccr1, ccr1_l, ccr1_h),
    TIM3: (Channel2, cc2e, ccmr1_output, oc2pe, oc2m, ccr2, ccr2_l, ccr2_h),
    TIM3: (Channel3, cc3e, ccmr2_output, oc3pe, oc3m, ccr3, ccr3_l, ccr3_h),
    TIM3: (Channel4, cc4e, ccmr2_output, oc4pe, oc4m, ccr4, ccr4_l, ccr4_h),
}

pwm! {
    TIM1: (tim1, arr),
    TIM3: (tim3, arr_l, arr_h),
    TIM14: (tim14, arr),
    TIM16: (tim16, arr),
    TIM17: (tim17, arr),
}

#[cfg(feature = "stm32g0x1")]
pwm! {
    TIM2: (tim2, arr_l, arr_h),
}

#[cfg(any(feature = "stm32g070", feature = "stm32g071", feature = "stm32g081"))]
pwm! {
    TIM15: (tim15, arr),
}
