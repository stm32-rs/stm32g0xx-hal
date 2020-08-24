//! External interrupt controller
use crate::gpio::SignalEdge;
use crate::stm32::EXTI;

/// EXTI trigger event
#[derive(PartialEq, PartialOrd, Clone, Copy)]
pub enum Event {
    GPIO0 = 0,
    GPIO1 = 1,
    GPIO2 = 2,
    GPIO3 = 3,
    GPIO4 = 4,
    GPIO5 = 5,
    GPIO6 = 6,
    GPIO7 = 7,
    GPIO8 = 8,
    GPIO9 = 9,
    GPIO10 = 10,
    GPIO11 = 11,
    GPIO12 = 12,
    GPIO13 = 13,
    GPIO14 = 14,
    GPIO15 = 15,
    PVD = 16,
    #[cfg(any(feature = "stm32g071", feature = "stm32g081"))]
    COMP1 = 17,
    #[cfg(any(feature = "stm32g071", feature = "stm32g081"))]
    COMP2 = 18,
    RTC = 19,
    TAMP = 21,
    I2C1 = 23,
    USART1 = 25,
    USART2 = 26,
    #[cfg(any(feature = "stm32g071", feature = "stm32g081"))]
    CEC = 27,
    LPUART1 = 28,
    LPTIM1 = 29,
    LPTIM2 = 30,
    LSE_CSS = 31,
    #[cfg(any(feature = "stm32g071", feature = "stm32g081"))]
    UCPD1 = 32,
    #[cfg(any(feature = "stm32g071", feature = "stm32g081"))]
    UCPD2 = 33,
}

impl Event {
    pub(crate) fn from_code(n: u8) -> Event {
        match n {
            0 => Event::GPIO0,
            1 => Event::GPIO1,
            2 => Event::GPIO2,
            3 => Event::GPIO3,
            4 => Event::GPIO4,
            5 => Event::GPIO5,
            6 => Event::GPIO6,
            7 => Event::GPIO7,
            8 => Event::GPIO8,
            9 => Event::GPIO9,
            10 => Event::GPIO10,
            11 => Event::GPIO11,
            12 => Event::GPIO12,
            13 => Event::GPIO13,
            14 => Event::GPIO14,
            15 => Event::GPIO15,
            _ => unreachable!(),
        }
    }
}

#[cfg(any(feature = "stm32g071", feature = "stm32g081"))]
const TRIGGER_MAX: u8 = 18;
#[cfg(any(feature = "stm32g031", feature = "stm32g041"))]
const TRIGGER_MAX: u8 = 16;
#[cfg(any(feature = "stm32g030", feature = "stm32g070"))]
const TRIGGER_MAX: u8 = 15;

pub trait ExtiExt {
    fn wakeup(&self, ev: Event);
    fn listen(&self, ev: Event, edge: SignalEdge);
    fn unlisten(&self, ev: Event);
    fn is_pending(&self, ev: Event, edge: SignalEdge) -> bool;
    fn unpend(&self, ev: Event);
}

impl ExtiExt for EXTI {
    fn listen(&self, ev: Event, edge: SignalEdge) {
        let line = ev as u8;
        assert!(line <= TRIGGER_MAX);
        let mask = 1 << line;
        match edge {
            SignalEdge::Rising => {
                self.rtsr1.modify(|r, w| unsafe { w.bits(r.bits() | mask) });
            }
            SignalEdge::Falling => {
                self.ftsr1.modify(|r, w| unsafe { w.bits(r.bits() | mask) });
            }
            SignalEdge::All => {
                self.rtsr1.modify(|r, w| unsafe { w.bits(r.bits() | mask) });
                self.ftsr1.modify(|r, w| unsafe { w.bits(r.bits() | mask) });
            }
        }
        self.wakeup(ev);
    }

    fn wakeup(&self, ev: Event) {
        #[cfg(any(
            feature = "stm32g030",
            feature = "stm32g070",
            feature = "stm32g031",
            feature = "stm32g041"
        ))]
        self.imr1
            .modify(|r, w| unsafe { w.bits(r.bits() | 1 << ev as u8) });

        #[cfg(any(feature = "stm32g071", feature = "stm32g081"))]
        match ev as u8 {
            line if line < 32 => self
                .imr1
                .modify(|r, w| unsafe { w.bits(r.bits() | 1 << line) }),
            line => self
                .imr2
                .modify(|r, w| unsafe { w.bits(r.bits() | 1 << (line - 32)) }),
        }
    }

    fn unlisten(&self, ev: Event) {
        self.unpend(ev);

        #[cfg(any(
            feature = "stm32g030",
            feature = "stm32g070",
            feature = "stm32g031",
            feature = "stm32g041"
        ))]
        {
            let line = ev as u8;
            let mask = !(1 << line);
            self.imr1.modify(|r, w| unsafe { w.bits(r.bits() & mask) });
            if line <= TRIGGER_MAX {
                self.rtsr1.modify(|r, w| unsafe { w.bits(r.bits() & mask) });
                self.ftsr1.modify(|r, w| unsafe { w.bits(r.bits() & mask) });
            }
        }

        #[cfg(any(feature = "stm32g071", feature = "stm32g081"))]
        match ev as u8 {
            line if line < 32 => {
                let mask = !(1 << line);
                self.imr1.modify(|r, w| unsafe { w.bits(r.bits() & mask) });
                if line <= TRIGGER_MAX {
                    self.rtsr1.modify(|r, w| unsafe { w.bits(r.bits() & mask) });
                    self.ftsr1.modify(|r, w| unsafe { w.bits(r.bits() & mask) });
                }
            }
            line => {
                let mask = !(1 << (line - 32));
                self.imr2.modify(|r, w| unsafe { w.bits(r.bits() & mask) })
            }
        }
    }

    fn is_pending(&self, ev: Event, edge: SignalEdge) -> bool {
        let line = ev as u8;
        if line > TRIGGER_MAX {
            return false;
        }
        let mask = 1 << line;
        match edge {
            SignalEdge::Rising => self.rpr1.read().bits() & mask != 0,
            SignalEdge::Falling => self.fpr1.read().bits() & mask != 0,
            SignalEdge::All => {
                (self.rpr1.read().bits() & mask != 0) && (self.fpr1.read().bits() & mask != 0)
            }
        }
    }

    fn unpend(&self, ev: Event) {
        let line = ev as u8;
        if line <= TRIGGER_MAX {
            self.rpr1.modify(|_, w| unsafe { w.bits(1 << line) });
            self.fpr1.modify(|_, w| unsafe { w.bits(1 << line) });
        }
    }
}
