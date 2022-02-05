//! Power control

use crate::{
    rcc::{Enable, Rcc},
    stm32::PWR,
};

pub enum LowPowerMode {
    StopMode1 = 0b000,
    StopMode2 = 0b001,
    Standby = 0b011,
    Shutdown = 0b111,
}

pub enum PowerMode {
    Run,
    LowPower(LowPowerMode),
    UltraLowPower(LowPowerMode),
}

pub struct Power {
    rb: PWR,
}

impl Power {
    pub fn new(pwr: PWR, rcc: &mut Rcc) -> Self {
        PWR::enable(rcc);
        Self { rb: pwr }
    }

    pub fn set_mode(&mut self, mode: PowerMode) {
        match mode {
            PowerMode::Run => {
                self.rb.cr1.modify(|_, w| w.lpr().clear_bit());
                while !self.rb.sr2.read().reglpf().bit_is_clear() {}
            }
            PowerMode::LowPower(sm) => {
                self.rb.cr3.modify(|_, w| w.ulpen().clear_bit());
                self.rb
                    .cr1
                    .modify(|_, w| unsafe { w.lpr().set_bit().lpms().bits(sm as u8) });
                while !self.rb.sr2.read().reglps().bit_is_set()
                    || !self.rb.sr2.read().reglpf().bit_is_set()
                {}
            }
            PowerMode::UltraLowPower(sm) => {
                self.rb.cr3.modify(|_, w| w.ulpen().set_bit());
                self.rb
                    .cr1
                    .modify(|_, w| unsafe { w.lpr().set_bit().lpms().bits(sm as u8) });
                while !self.rb.sr2.read().reglps().bit_is_set()
                    || !self.rb.sr2.read().reglpf().bit_is_set()
                {}
            }
        }
    }
}

pub trait PowerExt {
    fn constrain(self, rcc: &mut Rcc) -> Power;
}

impl PowerExt for PWR {
    fn constrain(self, rcc: &mut Rcc) -> Power {
        Power::new(self, rcc)
    }
}
