//! Delays
use cast::u32;
use core::cmp;
use cortex_m::peripheral::SYST;
use hal::blocking::delay::{DelayMs, DelayUs};

use crate::prelude::*;
use crate::rcc::Clocks;
use crate::time::{Hertz, MicroSecond};

/// System timer (SysTick) as a delay provider
pub struct Delay {
    clk: Hertz,
    syst: SYST,
}

impl Delay {
    /// Configures the system timer (SysTick) as a delay provider
    pub fn new(syst: SYST, clocks: &Clocks) -> Self {
        Delay {
            syst,
            clk: clocks.core_clk,
        }
    }

    pub fn delay<T>(&mut self, delay: T)
    where
        T: Into<MicroSecond>,
    {
        let mut ticks = delay.into().ticks(self.clk);
        while ticks > 0 {
            let reload = cmp::min(ticks, 0x00FF_FFFF);
            ticks -= reload;
            self.syst.set_reload(reload);
            self.syst.clear_current();
            self.syst.enable_counter();
            while !self.syst.has_wrapped() {}
            self.syst.disable_counter();
        }
    }

    /// Releases the system timer (SysTick) resource
    pub fn free(self) -> SYST {
        self.syst
    }
}

impl DelayUs<u32> for Delay {
    fn delay_us(&mut self, us: u32) {
        self.delay(us.us())
    }
}

impl DelayUs<u16> for Delay {
    fn delay_us(&mut self, us: u16) {
        self.delay_us(u32(us))
    }
}

impl DelayUs<u8> for Delay {
    fn delay_us(&mut self, us: u8) {
        self.delay_us(u32(us))
    }
}

impl DelayMs<u32> for Delay {
    fn delay_ms(&mut self, ms: u32) {
        self.delay_us(ms.saturating_mul(1_000_u32));
    }
}

impl DelayMs<u16> for Delay {
    fn delay_ms(&mut self, ms: u16) {
        self.delay_ms(u32(ms));
    }
}

impl DelayMs<u8> for Delay {
    fn delay_ms(&mut self, ms: u8) {
        self.delay_ms(u32(ms));
    }
}

pub trait DelayExt {
    fn delay(self, clocks: &Clocks) -> Delay;
}

impl DelayExt for SYST {
    fn delay(self, clocks: &Clocks) -> Delay {
        Delay::new(self, clocks)
    }
}
