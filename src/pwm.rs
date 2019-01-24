use core::marker::PhantomData;
use core::mem;

use crate::gpio::gpioa::*;
use crate::gpio::gpiob::*;
use crate::gpio::gpioc::*;
use crate::gpio::gpiod::*;
use crate::gpio::gpiof::*;
use crate::gpio::{AltFunction, DefaultMode};
use crate::rcc::Rcc;
use crate::stm32::{TIM1, TIM14, TIM15, TIM16, TIM17};
use crate::time::Hertz;
use hal;

pub struct C1;
pub struct C2;
pub struct C3;
pub struct C4;
pub struct C5;
pub struct C6;

pub trait Pins<TIM> {
    type Channel;
    fn setup(&self);
}

pub trait PwmExt: Sized {
    fn pwm<PINS, T>(self, _: PINS, frequency: T, rcc: &mut Rcc) -> PINS::Channel
    where
        PINS: Pins<Self>,
        T: Into<Hertz>;
}

pub struct Pwm<TIM, CHANNEL> {
    _channel: PhantomData<CHANNEL>,
    _tim: PhantomData<TIM>,
}

macro_rules! pwm16_channel {
    ($($TIMX:ident: ($CH:ty, $ccxe:ident, $ccmrx_output:ident, $ocxpe:ident, $ocxm:ident, $ccrx:ident),)+) => {
        $(
            impl hal::PwmPin for Pwm<$TIMX, $CH> {
                type Duty = u16;

                fn disable(&mut self) {
                    unsafe {
                        (*$TIMX::ptr()).ccer.modify(|_, w| w.$ccxe().clear_bit());
                    }
                }

                fn enable(&mut self) {
                    unsafe {
                        let tim = &*$TIMX::ptr();
                        tim.$ccmrx_output.modify(|_, w| w.$ocxpe().set_bit().$ocxm().bits(6));
                        tim.ccer.modify(|_, w| w.$ccxe().set_bit());
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

macro_rules! pwm_pins {
    ($TIMX:ident, [ $(($ch:ty, $pin:ty, $af_mode:expr),)+ ]) => {
        $(
            impl Pins<$TIMX> for $pin {
                type Channel = Pwm<$TIMX, $ch>;

                fn setup(&self) {
                    self.set_alt_mode($af_mode);
                }
            }
        )+
    };
}

macro_rules! pwm16 {
    ($($TIMX:ident: ($apbXenr:ident, $apbXrstr:ident, $timX:ident, $timXen:ident, $timXrst:ident, $arr:ident),)+) => {
        $(
            impl PwmExt for $TIMX {
                fn pwm<PINS, T>(self, pins: PINS, freq: T, rcc: &mut Rcc) -> PINS::Channel
                where
                    PINS: Pins<Self>,
                    T: Into<Hertz>,
                {
                    $timX(self, pins, freq.into(), rcc)
                }
            }

            fn $timX<PINS>(tim: $TIMX, pins: PINS, freq: Hertz, rcc: &mut Rcc) -> PINS::Channel
            where
                PINS: Pins<$TIMX>,
            {
                rcc.rb.$apbXenr.modify(|_, w| w.$timXen().set_bit());
                rcc.rb.$apbXrstr.modify(|_, w| w.$timXrst().set_bit());
                rcc.rb.$apbXrstr.modify(|_, w| w.$timXrst().clear_bit());
                let reload = rcc.clocks.apb_tim_clk.0 / freq.0;
                let psc = (reload - 1) / 0xffff;
                let arr = reload / (psc + 1);
                tim.psc.write(|w| unsafe { w.psc().bits(psc as u16) });
                tim.arr.write(|w| unsafe { w.$arr().bits(arr as u16) });
                tim.cr1.write(|w| w.cen().set_bit());
                pins.setup();
                unsafe { mem::uninitialized() }
            }
        )+
    }
}

pwm16_channel! {
    TIM1: (C1, cc1e, ccmr1_output, oc1pe, oc1m, ccr1),
    TIM1: (C2, cc2e, ccmr1_output, oc2pe, oc2m, ccr2),
    TIM1: (C3, cc3e, ccmr2_output, oc3pe, oc3m, ccr3),
    TIM1: (C4, cc4e, ccmr2_output, oc4pe, oc4m, ccr4),
    TIM1: (C5, cc5e, ccmr3_output, oc5pe, oc5m, ccr5),
    TIM1: (C6, cc6e, ccmr3_output, oc6pe, oc6m, ccr6),
    TIM14: (C1, cc1e, ccmr1_output, oc1pe, oc1m, ccr1),
    TIM15: (C1, cc1e, ccmr1_output, oc1pe, oc1m, ccr1),
    TIM16: (C1, cc1e, ccmr1_output, oc1pe, oc1m, ccr1),
    TIM17: (C1, cc1e, ccmr1_output, oc1pe, oc1m, ccr1),
}

pwm_pins!(TIM1, [
    (C1, PA8<DefaultMode>, AltFunction::AF2),
    (C1, PC8<DefaultMode>, AltFunction::AF2),
]);

pwm_pins!(TIM14, [
    (C1, PA4<DefaultMode>, AltFunction::AF4),
    (C1, PA7<DefaultMode>, AltFunction::AF4),
    (C1, PB1<DefaultMode>, AltFunction::AF0),
    (C1, PC12<DefaultMode>, AltFunction::AF2),
    (C1, PF0<DefaultMode>, AltFunction::AF2),
]);

pwm_pins!(TIM15, [
    (C1, PA2<DefaultMode>, AltFunction::AF5),
    (C1, PB14<DefaultMode>, AltFunction::AF5),
    (C1, PC1<DefaultMode>, AltFunction::AF2),
]);

pwm_pins!(TIM16, [
    (C1, PA6<DefaultMode>, AltFunction::AF5),
    (C1, PB8<DefaultMode>, AltFunction::AF2),
    (C1, PD0<DefaultMode>, AltFunction::AF2),
]);

pwm_pins!(TIM17, [
    (C1, PA7<DefaultMode>, AltFunction::AF6),
    (C1, PB9<DefaultMode>, AltFunction::AF2),
    (C1, PD1<DefaultMode>, AltFunction::AF2),
]);

pwm16! {
    TIM1: (apbenr2, apbrstr2, tim1, tim1en, tim1rst, arr),
    TIM14: (apbenr2, apbrstr2, tim14, tim14en, tim14rst, arr),
    TIM15: (apbenr2, apbrstr2, tim15, tim15en, tim15rst, arr),
    TIM16: (apbenr2, apbrstr2, tim16, tim16en, tim16rst, arr),
    TIM17: (apbenr2, apbrstr2, tim17, tim17en, tim17rst, arr),
}
