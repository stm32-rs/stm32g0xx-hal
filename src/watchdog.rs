use crate::prelude::*;
use crate::rcc::Rcc;
use crate::stm32::{IWDG, WWDG};
use crate::time::{Hertz, MicroSecond};
use hal::watchdog;

pub struct IndependedWatchdog {
    iwdg: IWDG,
}

impl watchdog::Watchdog for IndependedWatchdog {
    fn feed(&mut self) {
        self.iwdg.kr.write(|w| unsafe { w.key().bits(0xaaaa) });
    }
}

impl watchdog::WatchdogEnable for IndependedWatchdog {
    type Time = MicroSecond;

    fn start<T>(&mut self, period: T)
    where
        T: Into<MicroSecond>,
    {
        let mut ticks = period.into().ticks(16_384_u32.hz());
        let mut psc = 0_u8;
        let mut reload = 0;
        while psc < 6 {
            reload = ticks;
            if reload <= 0xfff {
                break;
            }
            psc += 1;
            ticks /= 2;
        }

        // Enable watchdog
        self.iwdg.kr.write(|w| unsafe { w.key().bits(0xcccc) });

        // Enable access to RLR/PR
        self.iwdg.kr.write(|w| unsafe { w.key().bits(0x5555) });

        self.iwdg.pr.write(|w| unsafe { w.pr().bits(psc) });
        self.iwdg
            .rlr
            .write(|w| unsafe { w.rl().bits(reload as u16) });

        while self.iwdg.sr.read().bits() > 0 {}

        self.iwdg.kr.write(|w| unsafe { w.key().bits(0xaaaa) });
    }
}

pub trait IndependedWatchdogExt {
    fn watchdog(self) -> IndependedWatchdog;
}

impl IndependedWatchdogExt for IWDG {
    fn watchdog(self) -> IndependedWatchdog {
        IndependedWatchdog { iwdg: self }
    }
}

pub struct WindowWatchdog {
    wwdg: WWDG,
    clk: Hertz,
}

impl watchdog::Watchdog for WindowWatchdog {
    fn feed(&mut self) {
        self.wwdg.cr.write(|w| unsafe { w.t().bits(0xff) });
    }
}

impl WindowWatchdog {
    pub fn set_window<T>(&mut self, window: T)
    where
        T: Into<MicroSecond>,
    {
        let mut ticks = window.into().ticks(self.clk);
        let mut psc = 0u8;
        let mut window = 0;
        while psc < 8 {
            window = ticks;
            if window <= 0x40 {
                break;
            }
            psc += 1;
            ticks /= 2;
        }
        assert!(window <= 0x40);
        self.wwdg
            .cfr
            .write(|w| unsafe { w.wdgtb().bits(psc).w().bits(window as u8) });
    }

    pub fn listen(&mut self) {
        self.wwdg.cfr.write(|w| w.ewi().set_bit());
    }

    pub fn unlisten(&mut self) {
        self.wwdg.cfr.write(|w| w.ewi().clear_bit());
    }
}

impl watchdog::WatchdogEnable for WindowWatchdog {
    type Time = MicroSecond;

    fn start<T>(&mut self, period: T)
    where
        T: Into<MicroSecond>,
    {
        self.set_window(period);
        self.feed();
        self.wwdg.cr.write(|w| w.wdga().set_bit());
    }
}

pub trait WindowWatchdogExt {
    fn watchdog(self, rcc: &mut Rcc) -> WindowWatchdog;
}

impl WindowWatchdogExt for WWDG {
    fn watchdog(self, rcc: &mut Rcc) -> WindowWatchdog {
        rcc.rb.apbenr1.modify(|_, w| w.wwdgen().set_bit());
        let clk = rcc.clocks.apb_clk.0 / 4096;
        WindowWatchdog {
            wwdg: self,
            clk: clk.hz(),
        }
    }
}
