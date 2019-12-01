//! # One-pulse Mode
use core::marker::PhantomData;

use crate::gpio::gpioa::*;
use crate::gpio::gpiob::*;
use crate::gpio::gpioc::*;
use crate::gpio::gpiod::*;
use crate::gpio::gpiof::*;
use crate::gpio::{AltFunction, DefaultMode};
use crate::prelude::*;
use crate::rcc::Rcc;
use crate::stm32::*;
use crate::time::{Hertz, MicroSecond};

pub struct Channel1;
pub struct Channel2;
pub struct Channel3;
pub struct Channel4;
pub struct Channel5;
pub struct Channel6;

pub trait OpmPin<TIM> {
    type Channel;
    fn setup(&self);
}

pub trait OpmExt: Sized {
    fn opm<PIN>(self, _: PIN, pulse_width: MicroSecond, rcc: &mut Rcc) -> Opm<Self, PIN::Channel>
    where
        PIN: OpmPin<Self>;
}

pub struct Opm<TIM, CHANNEL> {
    rb: TIM,
    clk: Hertz,
    pulse_width: MicroSecond,
    delay: MicroSecond,
    _channel: PhantomData<CHANNEL>,
}

macro_rules! opm_pins {
    ($TIMX:ident, [ $(($ch:ty, $pin:ty, $af_mode:expr),)+ ]) => {
        $(
            impl OpmPin<$TIMX> for $pin {
                type Channel = $ch;

                fn setup(&self) {
                    self.set_alt_mode($af_mode);
                }
            }
        )+
    };
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

macro_rules! opm {
    ($($TIMX:ident: ($apbXenr:ident, $apbXrstr:ident, $timX:ident, $timXen:ident, $timXrst:ident),)+) => {
        $(
            impl OpmExt for $TIMX {
                fn opm<PIN>(self, pin: PIN, pulse_width: MicroSecond, rcc: &mut Rcc) -> Opm<Self, PIN::Channel>
                where PIN: OpmPin<Self>
                {
                    $timX(self, pin, pulse_width, rcc)
                }
            }

            fn $timX<PIN>(tim: $TIMX, pin: PIN, pulse_width: MicroSecond, rcc: &mut Rcc) -> Opm<$TIMX, PIN::Channel>
            where
                PIN: OpmPin<$TIMX>,
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

opm_pins!(TIM1, [
    (Channel1, PA8<DefaultMode>, AltFunction::AF2),
    (Channel1, PC8<DefaultMode>, AltFunction::AF2),
    (Channel2, PA9<DefaultMode>, AltFunction::AF2),
    (Channel2, PB3<DefaultMode>, AltFunction::AF1),
    (Channel2, PC9<DefaultMode>, AltFunction::AF2),
    (Channel3, PA10<DefaultMode>, AltFunction::AF2),
    (Channel3, PB6<DefaultMode>, AltFunction::AF1),
    (Channel3, PC10<DefaultMode>, AltFunction::AF2),
    (Channel4, PA11<DefaultMode>, AltFunction::AF2),
    (Channel4, PC11<DefaultMode>, AltFunction::AF2),
]);

opm_pins!(TIM2, [
    (Channel1, PA0<DefaultMode>, AltFunction::AF2),
    (Channel1, PA5<DefaultMode>, AltFunction::AF2),
    (Channel1, PA15<DefaultMode>, AltFunction::AF2),
    (Channel1, PC4<DefaultMode>, AltFunction::AF2),
    (Channel2, PA1<DefaultMode>, AltFunction::AF2),
    (Channel2, PB3<DefaultMode>, AltFunction::AF2),
    (Channel2, PC5<DefaultMode>, AltFunction::AF2),
    (Channel3, PA2<DefaultMode>, AltFunction::AF2),
    (Channel3, PB10<DefaultMode>, AltFunction::AF2),
    (Channel4, PA3<DefaultMode>, AltFunction::AF2),
    (Channel4, PB11<DefaultMode>, AltFunction::AF2),
    (Channel4, PC7<DefaultMode>, AltFunction::AF2),
]);

opm_pins!(TIM3, [
    (Channel1, PA6<DefaultMode>, AltFunction::AF1),
    (Channel1, PB4<DefaultMode>, AltFunction::AF1),
    (Channel1, PC6<DefaultMode>, AltFunction::AF1),
    (Channel2, PA7<DefaultMode>, AltFunction::AF1),
    (Channel2, PB5<DefaultMode>, AltFunction::AF1),
    (Channel2, PC7<DefaultMode>, AltFunction::AF1),
    (Channel3, PB0<DefaultMode>, AltFunction::AF1),
    (Channel3, PC8<DefaultMode>, AltFunction::AF1),
    (Channel4, PB1<DefaultMode>, AltFunction::AF1),
    (Channel4, PC9<DefaultMode>, AltFunction::AF1),
]);

opm_pins!(TIM14, [
    (Channel1, PA4<DefaultMode>, AltFunction::AF4),
    (Channel1, PA7<DefaultMode>, AltFunction::AF4),
    (Channel1, PB1<DefaultMode>, AltFunction::AF0),
    (Channel1, PC12<DefaultMode>, AltFunction::AF2),
    (Channel1, PF0<DefaultMode>, AltFunction::AF2),
]);

#[cfg(any(feature = "stm32g07x", feature = "stm32g081"))]
opm_pins!(TIM15, [
    (Channel1, PA2<DefaultMode>, AltFunction::AF5),
    (Channel1, PB14<DefaultMode>, AltFunction::AF5),
    (Channel1, PC1<DefaultMode>, AltFunction::AF2),
]);

opm_pins!(TIM16, [
    (Channel1, PA6<DefaultMode>, AltFunction::AF5),
    (Channel1, PB8<DefaultMode>, AltFunction::AF2),
    (Channel1, PD0<DefaultMode>, AltFunction::AF2),
]);

opm_pins!(TIM17, [
    (Channel1, PA7<DefaultMode>, AltFunction::AF6),
    (Channel1, PB9<DefaultMode>, AltFunction::AF2),
    (Channel1, PD1<DefaultMode>, AltFunction::AF2),
]);

opm_hal! {
    TIM1: (Channel1, cc1e, ccmr1_output, oc1m, oc1fe, ccr1, arr),
    TIM1: (Channel2, cc2e, ccmr1_output, oc2m, oc2fe, ccr2, arr),
    TIM1: (Channel3, cc3e, ccmr2_output, oc3m, oc3fe, ccr3, arr),
    TIM1: (Channel4, cc4e, ccmr2_output, oc4m, oc4fe, ccr4, arr),
    TIM2: (Channel1, cc1e, ccmr1_output, oc1m, oc1fe, ccr1, arr_l, arr_h),
    TIM2: (Channel2, cc2e, ccmr1_output, oc2m, oc2fe, ccr2, arr_l, arr_h),
    TIM2: (Channel3, cc3e, ccmr2_output, oc3m, oc3fe, ccr3, arr_l, arr_h),
    TIM2: (Channel4, cc4e, ccmr2_output, oc4m, oc4fe, ccr4, arr_l, arr_h),
    TIM3: (Channel1, cc1e, ccmr1_output, oc1m, oc1fe, ccr1, arr_l, arr_h),
    TIM3: (Channel2, cc2e, ccmr1_output, oc2m, oc2fe, ccr2, arr_l, arr_h),
    TIM3: (Channel3, cc3e, ccmr2_output, oc3m, oc3fe, ccr3, arr_l, arr_h),
    TIM3: (Channel4, cc4e, ccmr2_output, oc4m, oc4fe, ccr4, arr_l, arr_h),
    TIM14: (Channel1, cc1e, ccmr1_output, oc1m, oc1fe, ccr1, arr),
    TIM16: (Channel1, cc1e, ccmr1_output, oc1m, oc1fe, ccr1, arr),
    TIM17: (Channel1, cc1e, ccmr1_output, oc1m, oc1fe, ccr1, arr),
}

opm! {
    TIM1: (apbenr2, apbrstr2, tim1, tim1en, tim1rst),
    TIM2: (apbenr1, apbrstr1, tim2, tim2en, tim2rst),
    TIM3: (apbenr1, apbrstr1, tim3, tim3en, tim3rst),
    TIM14: (apbenr2, apbrstr2, tim14, tim14en, tim14rst),
    TIM16: (apbenr2, apbrstr2, tim16, tim16en, tim16rst),
    TIM17: (apbenr2, apbrstr2, tim17, tim17en, tim17rst),
}

#[cfg(any(feature = "stm32g07x", feature = "stm32g081"))]
opm! {
    TIM15: (apbenr2, apbrstr2, tim15, tim15en, tim15rst),
}
