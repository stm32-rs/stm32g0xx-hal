//! Power control

use crate::{
    gpio::*,
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

pub enum WakeUpLine {
    Internal,
    WKUP1,
    WKUP2,
    // TODO: missing in SVD file
    // WKUP3,
    WKUP4,
    WKUP5,
    WKUP6,
}

pub struct Power {
    rb: PWR,
}

impl Power {
    pub fn new(pwr: PWR, rcc: &mut Rcc) -> Self {
        PWR::enable(rcc);
        Self { rb: pwr }
    }

    pub fn get_standby_flag(&mut self) -> bool {
        self.rb.sr1.read().sbf().bit_is_set()
    }

    pub fn get_wakeup_flag<L: Into<WakeUpLine>>(&self, lane: L) -> bool {
        match lane.into() {
            WakeUpLine::WKUP1 => self.rb.sr1.read().wuf1().bit_is_set(),
            WakeUpLine::WKUP2 => self.rb.sr1.read().wuf2().bit_is_set(),
            WakeUpLine::WKUP4 => self.rb.sr1.read().wuf4().bit_is_set(),
            WakeUpLine::WKUP5 => self.rb.sr1.read().wuf5().bit_is_set(),
            WakeUpLine::WKUP6 => self.rb.sr1.read().wuf6().bit_is_set(),
            _ => false,
        }
    }

    pub fn clear_wakeup_flag<L: Into<WakeUpLine>>(&mut self, lane: L) {
        match lane.into() {
            WakeUpLine::WKUP1 => self.rb.scr.write(|w| w.cwuf1().set_bit()),
            WakeUpLine::WKUP2 => self.rb.scr.write(|w| w.cwuf2().set_bit()),
            WakeUpLine::WKUP4 => self.rb.scr.write(|w| w.cwuf4().set_bit()),
            WakeUpLine::WKUP5 => self.rb.scr.write(|w| w.cwuf5().set_bit()),
            WakeUpLine::WKUP6 => self.rb.scr.write(|w| w.cwuf6().set_bit()),
            _ => {}
        }
    }

    pub fn clear_standby_flag(&mut self) {
        self.rb.scr.write(|w| w.csbf().set_bit());
    }

    pub fn enable_wakeup_lane<L: Into<WakeUpLine>>(&mut self, lane: L, edge: SignalEdge) {
        assert!(edge != SignalEdge::All);

        let edge = edge == SignalEdge::Falling;
        match lane.into() {
            WakeUpLine::WKUP1 => {
                self.rb.cr3.modify(|_, w| w.ewup1().set_bit());
                self.rb.cr4.modify(|_, w| w.wp1().bit(edge));
            }
            WakeUpLine::WKUP2 => {
                self.rb.cr3.modify(|_, w| w.ewup2().set_bit());
                self.rb.cr4.modify(|_, w| w.wp2().bit(edge));
            }
            WakeUpLine::WKUP4 => {
                self.rb.cr3.modify(|_, w| w.ewup4().set_bit());
                self.rb.cr4.modify(|_, w| w.wp4().bit(edge));
            }
            WakeUpLine::WKUP5 => {
                self.rb.cr3.modify(|_, w| w.ewup5().set_bit());
                self.rb.cr4.modify(|_, w| w.wp4().bit(edge));
            }
            WakeUpLine::WKUP6 => {
                self.rb.cr3.modify(|_, w| w.ewup6().set_bit());
                self.rb.cr4.modify(|_, w| w.wp5().bit(edge));
            }
            WakeUpLine::Internal => self.rb.cr3.modify(|_, w| w.eiwul().set_bit()),
        }
    }

    pub fn disable_wakeup_lane<L: Into<WakeUpLine>>(&mut self, lane: L) {
        match lane.into() {
            WakeUpLine::WKUP1 => self.rb.cr3.modify(|_, w| w.ewup1().clear_bit()),
            WakeUpLine::WKUP2 => self.rb.cr3.modify(|_, w| w.ewup2().clear_bit()),
            WakeUpLine::WKUP4 => self.rb.cr3.modify(|_, w| w.ewup4().clear_bit()),
            WakeUpLine::WKUP5 => self.rb.cr3.modify(|_, w| w.ewup5().clear_bit()),
            WakeUpLine::WKUP6 => self.rb.cr3.modify(|_, w| w.ewup6().clear_bit()),
            WakeUpLine::Internal => self.rb.cr3.modify(|_, w| w.eiwul().clear_bit()),
        }
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

macro_rules! wakeup_pins {
    ($($PIN:path: $line:expr,)+) => {
        $(
            impl<M> From<&$PIN> for WakeUpLine {
                fn from(_: &$PIN) -> Self {
                    $line
                 }
            }
        )+
    }
}

wakeup_pins! {
    gpioa::PA0<M>: WakeUpLine::WKUP1,
    gpioa::PA4<M>: WakeUpLine::WKUP2,
    gpioc::PC13<M>: WakeUpLine::WKUP2,
    gpioa::PA2<M>: WakeUpLine::WKUP4,
    gpioc::PC5<M>: WakeUpLine::WKUP5,
    gpiob::PB5<M>: WakeUpLine::WKUP6,
}

pub trait PowerExt {
    fn constrain(self, rcc: &mut Rcc) -> Power;
}

impl PowerExt for PWR {
    fn constrain(self, rcc: &mut Rcc) -> Power {
        Power::new(self, rcc)
    }
}
