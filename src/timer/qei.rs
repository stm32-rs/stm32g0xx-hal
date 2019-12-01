//! Quadrature Encoder Interface
use crate::hal::{self, Direction};
use crate::rcc::Rcc;
use crate::stm32::{TIM1, TIM2, TIM3};
use crate::timer::*;
use crate::timer::pins::TimerPin;

pub struct Qei<TIM, PINS> {
    tim: TIM,
    pins: PINS,
}

pub trait QeiPins<TIM> {
    fn setup(&self);
}

impl<TIM, P1, P2> QeiPins<TIM> for (P1, P2)
where
    P1: TimerPin<TIM, Channel = Channel1>,
    P2: TimerPin<TIM, Channel = Channel2>,
{
    fn setup(&self) {
        self.0.setup();
        self.1.setup();
    }
}

pub trait QeiExt<TIM, PINS>
where
    PINS: QeiPins<TIM>,
{
    fn qei(self, pins: PINS, rcc: &mut Rcc) -> Qei<TIM, PINS>;
}

macro_rules! qei {
    ($($TIMX:ident: ($tim:ident, $timXen:ident, $timXrst:ident, $apbenr:ident, $apbrstr:ident, $arr:ident, $cnt:ident),)+) => {
        $(
            impl<PINS> Qei<$TIMX, PINS> where PINS: QeiPins<$TIMX> {
                fn $tim(tim: $TIMX, pins: PINS, rcc: &mut Rcc) -> Self {
                    pins.setup();
                    // enable and reset peripheral to a clean slate state
                    rcc.rb.$apbenr.modify(|_, w| w.$timXen().set_bit());
                    rcc.rb.$apbrstr.modify(|_, w| w.$timXrst().set_bit());
                    rcc.rb.$apbrstr.modify(|_, w| w.$timXrst().clear_bit());

                    // Configure TxC1 and TxC2 as captures
                    tim.ccmr1_output().write(|w| unsafe {
                        w.cc1s().bits(0b01).cc2s().bits(0b01)
                    });

                    // Encoder mode 2.
                    tim.smcr.write(|w| unsafe { w.sms().bits(0b010) });

                    // Enable and configure to capture on rising edge
                    tim.ccer.write(|w| {
                        w.cc1e()
                            .set_bit()
                            .cc2e()
                            .set_bit()
                            .cc1p()
                            .clear_bit()
                            .cc2p()
                            .clear_bit()
                            .cc1np()
                            .clear_bit()
                            .cc2np()
                            .clear_bit()
                    });

                    tim.cr1.write(|w| w.cen().set_bit());
                    Qei { tim, pins }
                }

                pub fn release(self) -> ($TIMX, PINS) {
                    (self.tim, self.pins)
                }
            }

            impl<PINS> hal::Qei for Qei<$TIMX, PINS> {
                type Count = u16;

                fn count(&self) -> u16 {
                    self.tim.cnt.read().$cnt().bits()
                }

                fn direction(&self) -> Direction {
                    if self.tim.cr1.read().dir().bit_is_clear() {
                        hal::Direction::Upcounting
                    } else {
                        hal::Direction::Downcounting
                    }
                }
            }

            impl<PINS> QeiExt<$TIMX, PINS> for $TIMX where PINS: QeiPins<$TIMX> {
                fn qei(self, pins: PINS, rcc: &mut Rcc) -> Qei<$TIMX, PINS> {
                    Qei::$tim(self, pins, rcc)
                }
            }
        )+
    }
}

qei! {
    TIM1: (tim1,  tim1en, tim1rst, apbenr2, apbrstr2, arr, cnt),
    TIM2: (tim2,  tim2en, tim2rst, apbenr1, apbrstr1, arr_l, cnt_l),
    TIM3: (tim3,  tim3en, tim3rst, apbenr1, apbrstr1, arr_l, cnt_l),
}
