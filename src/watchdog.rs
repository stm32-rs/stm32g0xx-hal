use crate::prelude::*;
use crate::rcc::{Enable, Rcc};
use crate::stm32::{IWDG, WWDG};
use crate::time::{Hertz, MicroSecond};
use hal::watchdog;

pub struct IndependedWatchdog {
    iwdg: IWDG,
}

impl IndependedWatchdog {
    pub fn feed(&mut self) {
        self.iwdg.kr.write(|w| unsafe { w.key().bits(0xaaaa) });
    }

    pub fn start(&mut self, period: MicroSecond) {
        let mut cycles = crate::time::cycles(period, 16_384.Hz());
        let mut psc = 0;
        let mut reload = 0;
        while psc < 6 {
            reload = cycles;
            if reload <= 0xfff {
                break;
            }
            psc += 1;
            cycles /= 2;
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

impl watchdog::Watchdog for IndependedWatchdog {
    fn feed(&mut self) {
        self.feed();
    }
}

impl watchdog::WatchdogEnable for IndependedWatchdog {
    type Time = MicroSecond;

    fn start<T>(&mut self, period: T)
    where
        T: Into<MicroSecond>,
    {
        self.start(period.into())
    }
}

pub trait IWDGExt {
    fn constrain(self) -> IndependedWatchdog;
}

impl IndependedWatchdog {
    pub fn release(self) -> IWDG {
        self.iwdg
    }
}

impl IWDGExt for IWDG {
    fn constrain(self) -> IndependedWatchdog {
        IndependedWatchdog { iwdg: self }
    }
}

pub struct WindowWatchdog {
    wwdg: WWDG,
    clk: Hertz,
}

impl WindowWatchdog {
    pub fn feed(&mut self) {
        self.wwdg.cr.write(|w| unsafe { w.t().bits(0xff) });
    }

    pub fn set_window(&mut self, window: MicroSecond) {
        let mut cycles = crate::time::cycles(window, self.clk);
        let mut psc = 0u8;
        let mut window = 0;
        while psc < 8 {
            window = cycles;
            if window <= 0x40 {
                break;
            }
            psc += 1;
            cycles /= 2;
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

    pub fn release(self) -> WWDG {
        self.wwdg
    }

    pub fn start(&mut self, period: MicroSecond) {
        self.set_window(period);
        self.feed();
        self.wwdg.cr.write(|w| w.wdga().set_bit());
    }
}

impl watchdog::Watchdog for WindowWatchdog {
    fn feed(&mut self) {
        self.feed();
    }
}

impl watchdog::WatchdogEnable for WindowWatchdog {
    type Time = MicroSecond;

    fn start<T>(&mut self, period: T)
    where
        T: Into<MicroSecond>,
    {
        self.start(period.into())
    }
}

pub trait WWDGExt {
    fn constrain(self, rcc: &mut Rcc) -> WindowWatchdog;
}

impl WWDGExt for WWDG {
    fn constrain(self, rcc: &mut Rcc) -> WindowWatchdog {
        WWDG::enable(rcc);
        let clk = rcc.clocks.apb_clk.raw() / 4096;
        WindowWatchdog {
            wwdg: self,
            clk: clk.Hz(),
        }
    }
}
