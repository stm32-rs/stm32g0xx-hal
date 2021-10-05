//! Quadrature Encoder Interface
use crate::hal::{self, Direction};
use crate::rcc::*;

#[cfg(feature = "stm32g0x1")]
use crate::stm32::{TIM1, TIM2, TIM3};
#[cfg(feature = "stm32g0x0")]
use crate::stm32::{TIM1, TIM3};

use crate::timer::pins::TimerPin;
use crate::timer::*;

pub struct Qei<TIM, PINS> {
    tim: TIM,
    pins: PINS,
}

pub trait QeiPins<TIM> {
    fn setup(&self);
    fn release(self) -> Self;
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

    fn release(self) -> Self {
        (self.0.release(), self.1.release())
    }
}

pub trait QeiExt<TIM, PINS>
where
    PINS: QeiPins<TIM>,
{
    fn qei(self, pins: PINS, rcc: &mut Rcc) -> Qei<TIM, PINS>;
}

macro_rules! qei {
    ($($TIMX:ident: ($tim:ident, $arr:ident, $cnt:ident),)+) => {
        $(
            impl<PINS> Qei<$TIMX, PINS> where PINS: QeiPins<$TIMX> {
                fn $tim(tim: $TIMX, pins: PINS, rcc: &mut Rcc) -> Self {
                    // enable and reset peripheral to a clean slate state
                    $TIMX::enable(rcc);
                    $TIMX::reset(rcc);

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

                    pins.setup();

                    tim.cr1.write(|w| w.cen().set_bit());
                    Qei { tim, pins }
                }

                pub fn release(self) -> ($TIMX, PINS) {
                    (self.tim, self.pins.release())
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
    TIM1: (tim1, arr, cnt),
    TIM3: (tim3, arr_l, cnt_l),
}

#[cfg(feature = "stm32g0x1")]
qei! {
    TIM2: (tim2, arr_l, cnt_l),
}
