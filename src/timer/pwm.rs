//! # Pulse Width Modulation
use core::marker::PhantomData;

use crate::rcc::*;
use crate::stm32::*;
use crate::time::Hertz;
use crate::timer::pins::TimerPin;
use crate::timer::*;
use embedded_hal::pwm::{ErrorKind, ErrorType, SetDutyCycle};

#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
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

enum ClockSource {
    ApbTim,
    #[allow(dead_code)]
    Pllq,
}

pub trait PwmExt: Sized {
    fn pwm(self, freq: Hertz, rcc: &mut Rcc) -> Pwm<Self>;
}

pub trait PwmQExt: Sized {
    // Configures PWM using PLLQ as a clock source. Panics if PLLQ was not
    // enabled when RCC was configured.
    fn pwm_q(self, freq: Hertz, rcc: &mut Rcc) -> Pwm<Self>;
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

impl<T: super::private::TimerBase> PwmExt for T {
    fn pwm(self, freq: Hertz, rcc: &mut Rcc) -> Pwm<Self> {
        Pwm::new(self, freq, rcc, ClockSource::ApbTim)
    }
}

impl<T: super::private::TimerBase> Pwm<T> {
    fn new(mut tim: T, freq: Hertz, rcc: &mut Rcc, clock_source: ClockSource) -> Pwm<T> {
        tim.init(rcc);

        let clk = match clock_source {
            ClockSource::ApbTim => {
                rcc.ccipr().modify(|_, w| w.tim1sel().clear_bit());
                rcc.clocks.apb_tim_clk
            }
            ClockSource::Pllq => {
                rcc.ccipr().modify(|_, w| w.tim1sel().set_bit());
                rcc.clocks.pll_clk.q.unwrap()
            }
        };

        tim.set_freq(freq, clk);
        tim.resume();

        Self { clk, tim }
    }

    /// Set the PWM frequency. Actual frequency may differ from
    /// requested due to precision of input clock. To check actual
    /// frequency, call freq.
    pub fn set_freq(&mut self, freq: Hertz) {
        self.tim.set_freq(freq, self.clk);
    }
    /// Starts listening
    pub fn listen(&mut self) {
        self.tim.listen();
    }

    /// Stops listening
    pub fn unlisten(&mut self) {
        self.tim.unlisten();
    }
    /// Clears interrupt flag
    pub fn clear_irq(&mut self) {
        self.tim.clear_irq();
    }

    /// Resets counter value
    pub fn reset(&mut self) {
        self.tim.reset();
    }

    /// Returns the currently configured frequency
    pub fn freq(&self) -> Hertz {
        self.tim.freq(self.clk)
    }
}

#[allow(unused_macros)]
macro_rules! pwm_q {
    ($($TIMX:ident: $timX:ident,)+) => {
        $(
            impl PwmQExt for $TIMX {
                fn pwm_q(self, freq: Hertz, rcc: &mut Rcc) -> Pwm<Self> {
                    Pwm::new(self, freq, rcc, ClockSource::Pllq)
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
            impl PwmPin<$TIMX, $CH> {
                pub fn disable(&mut self) {
                    unsafe {
                        (*$TIMX::ptr()).ccer().modify(|_, w| w.$ccxe().clear_bit());
                    }
                }

                pub fn enable(&mut self) {
                    unsafe {
                        let tim = &*$TIMX::ptr();
                        tim.$ccmrx_output().modify(|_, w| w.$ocxpe().set_bit().$ocxm().bits(6));
                        tim.ccer().modify(|_, w| w.$ccxe().set_bit());
                    }
                }


                pub fn get_duty(&self) -> u32 {
                    unsafe { (*$TIMX::ptr()).$ccrx().read().bits() }
                }

                pub fn get_max_duty(&self) -> u32 {
                    unsafe { (*$TIMX::ptr()).arr().read().bits() }

                }

                pub fn set_duty(&mut self, duty: u32) {
                    unsafe { (*$TIMX::ptr()).$ccrx().write(|w| w.bits(duty)); }
                }
            }

            impl ErrorType for PwmPin<$TIMX, $CH> {
                type Error = ErrorKind;
            }

            impl SetDutyCycle for PwmPin<$TIMX, $CH> {
                fn max_duty_cycle(&self) -> u16 {
                    self.get_max_duty() as u16
                }

                fn set_duty_cycle(&mut self, duty: u16) -> Result<(), Self::Error> {
                    self.set_duty(duty as u32);
                    Ok(())
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
            impl PwmPin<$TIMX, $CH> {
                pub fn disable(&mut self) {
                    unsafe {
                        (*$TIMX::ptr()).ccer().modify(|_, w| w.$ccxe().clear_bit());
                    }
                }

                pub fn enable(&mut self) {
                    unsafe {
                        let tim = &*$TIMX::ptr();
                        tim.$ccmrx_output().modify(|_, w| w.$ocxpe().set_bit().$ocxm().bits(6));
                        tim.ccer().modify(|_, w| w.$ccxe().set_bit());
                        $(
                            tim.ccer().modify(|_, w| w.$ccxne().bit(true));
                        )*
                        $(
                            tim.bdtr().modify(|_, w| w.$moe().set_bit());
                        )*
                    }
                }

                pub fn get_duty(&self) -> u16 {
                    unsafe { (*$TIMX::ptr()).$ccrx(<$CH>::N).read().$ccrx().bits() }
                }

                pub fn get_max_duty(&self) -> u16 {
                    unsafe { (*$TIMX::ptr()).arr().read().arr().bits() }
                }

                pub fn set_duty(&mut self, duty: u16) {
                    unsafe { (*$TIMX::ptr()).$ccrx(<$CH>::N).write(|w| w.$ccrx().bits(duty)); }
                }
            }

            impl ErrorType for PwmPin<$TIMX, $CH> {
                type Error = ErrorKind;
            }

            impl SetDutyCycle for PwmPin<$TIMX, $CH> {
                fn max_duty_cycle(&self) -> u16 {
                    self.get_max_duty() as u16
                }

                fn set_duty_cycle(&mut self, duty: u16) -> Result<(), Self::Error> {
                    self.set_duty(duty);
                    Ok(())
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
    TIM1:  (Channel1, cc1e: cc1ne, ccmr1_output, oc1pe, oc1m, ccr, moe),
    TIM1:  (Channel2, cc2e: cc2ne, ccmr1_output, oc2pe, oc2m, ccr, moe),
    TIM1:  (Channel3, cc3e: cc3ne, ccmr2_output, oc3pe, oc3m, ccr, moe),
    TIM1:  (Channel4, cc4e, ccmr2_output, oc4pe, oc4m, ccr, moe),
    TIM14: (Channel1, cc1e, ccmr1_output, oc1pe, oc1m, ccr),
    TIM16: (Channel1, cc1e: cc1ne, ccmr1_output, oc1pe, oc1m, ccr, moe),
    TIM17: (Channel1, cc1e: cc1ne, ccmr1_output, oc1pe, oc1m, ccr, moe),
}

#[cfg(any(feature = "stm32g070", feature = "stm32g071", feature = "stm32g081"))]
pwm_advanced_hal! {
    TIM15: (Channel1, cc1e: cc1ne, ccmr1_output, oc1pe, oc1m, ccr, moe),
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
