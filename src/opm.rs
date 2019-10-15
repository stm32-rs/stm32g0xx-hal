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

pub struct C1;
pub struct C2;
pub struct C3;
pub struct C4;
pub struct C5;
pub struct C6;

pub trait OpmPin<TIM> {
    type Channel;
    fn setup(&self);
}

pub trait OpmExt: Sized {
    fn opm<PIN>(self, _: PIN, rcc: &mut Rcc) -> Opm<Self, PIN::Channel>
    where
        PIN: OpmPin<Self>;
}

pub struct Opm<TIM, CHANNEL> {
    tim: TIM,
    clk: Hertz,
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
                pub fn config(&mut self, pulse_width: MicroSecond, delay: Option<MicroSecond>) {
                    let delay = delay.unwrap_or(0.us());

                    let period = pulse_width + delay;

                    let cycles_per_period = self.clk / period.into();
                    let psc = (cycles_per_period - 1) / 0xffff;

                    self.tim.ccer.modify(|_, w| w.$ccxe().set_bit());
                    self.tim.psc.write(|w| unsafe { w.psc().bits(psc as u16) });
                    let freq = (self.clk.0 / (psc + 1)).hz();
                    let reload = cycles_per_period / (psc + 1);
                    let compare = if delay.0 == 0 { 1 } else { delay.cycles(freq) };
                    unsafe {
                        self.tim.arr.write(|w| w.$arr().bits(reload as u16));
                        self.tim.$ccrx.write(|w| w.bits(compare));
                        $(
                            self.tim.arr.modify(|_, w| w.$arr_h().bits((reload >> 16) as u16));
                        )*
                        self.tim.$ccmrx_output().modify(|_, w| w.$ocxm().bits(7).$ocxfe().set_bit());
                    }
                }

                pub fn generate(&mut self) {
                    self.tim.cr1.write(|w| w.opm().set_bit().cen().set_bit());
                }
            }
        )+
    };
}

macro_rules! opm {
    ($($TIMX:ident: ($apbXenr:ident, $apbXrstr:ident, $timX:ident, $timXen:ident, $timXrst:ident),)+) => {
        $(
            impl OpmExt for $TIMX {
                fn opm<PIN>(self, pin: PIN, rcc: &mut Rcc) -> Opm<Self, PIN::Channel>
                where PIN: OpmPin<Self>
                {
                    $timX(self, pin, rcc)
                }
            }

            fn $timX<PIN>(tim: $TIMX, pin: PIN, rcc: &mut Rcc) -> Opm<$TIMX, PIN::Channel>
            where
                PIN: OpmPin<$TIMX>,
            {
                pin.setup();
                rcc.rb.$apbXenr.modify(|_, w| w.$timXen().set_bit());
                rcc.rb.$apbXrstr.modify(|_, w| w.$timXrst().set_bit());
                rcc.rb.$apbXrstr.modify(|_, w| w.$timXrst().clear_bit());
                Opm {
                    tim,
                    clk: rcc.clocks.apb_tim_clk,
                    _channel: PhantomData,
                }
            }
        )+
    }
}

opm_pins!(TIM1, [
    (C1, PA8<DefaultMode>, AltFunction::AF2),
    (C1, PC8<DefaultMode>, AltFunction::AF2),
    (C2, PA9<DefaultMode>, AltFunction::AF2),
    (C2, PB3<DefaultMode>, AltFunction::AF1),
    (C2, PC9<DefaultMode>, AltFunction::AF2),
    (C3, PA10<DefaultMode>, AltFunction::AF2),
    (C3, PB6<DefaultMode>, AltFunction::AF1),
    (C3, PC10<DefaultMode>, AltFunction::AF2),
    (C4, PA11<DefaultMode>, AltFunction::AF2),
    (C4, PC11<DefaultMode>, AltFunction::AF2),
]);

opm_pins!(TIM2, [
    (C1, PA0<DefaultMode>, AltFunction::AF2),
    (C1, PA5<DefaultMode>, AltFunction::AF2),
    (C1, PA15<DefaultMode>, AltFunction::AF2),
    (C1, PC4<DefaultMode>, AltFunction::AF2),
    (C2, PA1<DefaultMode>, AltFunction::AF2),
    (C2, PB3<DefaultMode>, AltFunction::AF2),
    (C2, PC5<DefaultMode>, AltFunction::AF2),
    (C3, PA2<DefaultMode>, AltFunction::AF2),
    (C3, PB10<DefaultMode>, AltFunction::AF2),
    (C4, PA3<DefaultMode>, AltFunction::AF2),
    (C4, PB11<DefaultMode>, AltFunction::AF2),
    (C4, PC7<DefaultMode>, AltFunction::AF2),
]);

opm_pins!(TIM3, [
    (C1, PA6<DefaultMode>, AltFunction::AF1),
    (C1, PB4<DefaultMode>, AltFunction::AF1),
    (C1, PC6<DefaultMode>, AltFunction::AF1),
    (C2, PA7<DefaultMode>, AltFunction::AF1),
    (C2, PB5<DefaultMode>, AltFunction::AF1),
    (C2, PC7<DefaultMode>, AltFunction::AF1),
    (C3, PB0<DefaultMode>, AltFunction::AF1),
    (C3, PC8<DefaultMode>, AltFunction::AF1),
    (C4, PB1<DefaultMode>, AltFunction::AF1),
    (C4, PC9<DefaultMode>, AltFunction::AF1),
]);

opm_pins!(TIM14, [
    (C1, PA4<DefaultMode>, AltFunction::AF4),
    (C1, PA7<DefaultMode>, AltFunction::AF4),
    (C1, PB1<DefaultMode>, AltFunction::AF0),
    (C1, PC12<DefaultMode>, AltFunction::AF2),
    (C1, PF0<DefaultMode>, AltFunction::AF2),
]);

#[cfg(any(feature = "stm32g07x", feature = "stm32g081"))]
opm_pins!(TIM15, [
    (C1, PA2<DefaultMode>, AltFunction::AF5),
    (C1, PB14<DefaultMode>, AltFunction::AF5),
    (C1, PC1<DefaultMode>, AltFunction::AF2),
]);

opm_pins!(TIM16, [
    (C1, PA6<DefaultMode>, AltFunction::AF5),
    (C1, PB8<DefaultMode>, AltFunction::AF2),
    (C1, PD0<DefaultMode>, AltFunction::AF2),
]);

opm_pins!(TIM17, [
    (C1, PA7<DefaultMode>, AltFunction::AF6),
    (C1, PB9<DefaultMode>, AltFunction::AF2),
    (C1, PD1<DefaultMode>, AltFunction::AF2),
]);

opm_hal! {
    TIM1: (C1, cc1e, ccmr1_output, oc1m, oc1fe, ccr1, arr),
    TIM1: (C2, cc2e, ccmr1_output, oc2m, oc2fe, ccr2, arr),
    TIM1: (C3, cc3e, ccmr2_output, oc3m, oc3fe, ccr3, arr),
    TIM1: (C4, cc4e, ccmr2_output, oc4m, oc4fe, ccr4, arr),
    TIM2: (C1, cc1e, ccmr1_output, oc1m, oc1fe, ccr1, arr_l, arr_h),
    TIM2: (C2, cc2e, ccmr1_output, oc2m, oc2fe, ccr2, arr_l, arr_h),
    TIM2: (C3, cc3e, ccmr2_output, oc3m, oc3fe, ccr3, arr_l, arr_h),
    TIM2: (C4, cc4e, ccmr2_output, oc4m, oc4fe, ccr4, arr_l, arr_h),
    TIM3: (C1, cc1e, ccmr1_output, oc1m, oc1fe, ccr1, arr_l, arr_h),
    TIM3: (C2, cc2e, ccmr1_output, oc2m, oc2fe, ccr2, arr_l, arr_h),
    TIM3: (C3, cc3e, ccmr2_output, oc3m, oc3fe, ccr3, arr_l, arr_h),
    TIM3: (C4, cc4e, ccmr2_output, oc4m, oc4fe, ccr4, arr_l, arr_h),
    TIM14: (C1, cc1e, ccmr1_output, oc1m, oc1fe, ccr1, arr),
    TIM16: (C1, cc1e, ccmr1_output, oc1m, oc1fe, ccr1, arr),
    TIM17: (C1, cc1e, ccmr1_output, oc1m, oc1fe, ccr1, arr),
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
