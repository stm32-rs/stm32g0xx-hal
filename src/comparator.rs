//! Comparator
use crate::rcc::Rcc;
use crate::stm32::COMP;

pub struct Config;

pub struct Comparator {
    rb: COMP,
}

pub fn comparator(comp: COMP, cfg: Config, rcc: &mut Rcc) -> Comparator {
    // Enable COMP clocks
    rcc.rb.apbenr2.modify(|_, w| w.syscfgen().set_bit());

    // Reset COMP
    rcc.rb.apbrstr2.modify(|_, w| w.syscfgrst().set_bit());
    rcc.rb.apbrstr2.modify(|_, w| w.syscfgrst().clear_bit());

    Comparator { rb: comp }
}

pub trait ComparatorExt {
    fn constrain(self, cfg: Config, rcc: &mut Rcc) -> Comparator;
}

impl ComparatorExt for COMP {
    fn constrain(self, cfg: Config, rcc: &mut Rcc) -> Comparator {
        comparator(self, cfg, rcc)
    }
}
