use crate::stm32::{rcc, FLASH, PWR, RCC};
use crate::time::{Hertz, U32Ext};

mod clockout;
mod config;
mod enable;

pub use clockout::*;
pub use config::*;

/// HSI speed
pub const HSI_FREQ: u32 = 16_000_000;

/// Clock frequencies
#[derive(Clone, Copy)]
pub struct Clocks {
    /// System frequency
    pub sys_clk: Hertz,
    /// Core frequency
    pub core_clk: Hertz,
    /// AHB frequency
    pub ahb_clk: Hertz,
    /// APB frequency
    pub apb_clk: Hertz,
    /// APB timers frequency
    pub apb_tim_clk: Hertz,
    /// PLL frequency
    pub pll_clk: PLLClocks,
}

/// PLL Clock frequencies
#[derive(Clone, Copy)]
pub struct PLLClocks {
    /// R frequency
    pub r: Hertz,
    /// Q frequency
    pub q: Option<Hertz>,
    /// P frequency
    pub p: Option<Hertz>,
}

impl Default for Clocks {
    fn default() -> Clocks {
        Clocks {
            sys_clk: 16.mhz(),
            ahb_clk: 16.mhz(),
            core_clk: 2.mhz(),
            apb_clk: 16.mhz(),
            apb_tim_clk: 16.mhz(),
            pll_clk: PLLClocks {
                r: 64.mhz(),
                q: None,
                p: None,
            },
        }
    }
}

/// Reset pin mode and remap
pub enum ResetMode {
    /// Reset Input only: a low level on the NRST pin generates system reset, internal RESET not propagated to the NSRT pin
    ResetInput = 0b01,
    /// GPIO: standard GPIO pad functionality, only internal RESET possible
    GPIO = 0b10,
    /// Bidirectional reset: NRST pin configured in reset input/output mode (legacy mode)
    Bidirectional = 0b11,
}

/// Constrained RCC peripheral
pub struct Rcc {
    /// Clock configuration
    pub clocks: Clocks,
    pub(crate) rb: RCC,
}

impl core::ops::Deref for Rcc {
    type Target = RCC;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.rb
    }
}

impl Rcc {
    /// Apply clock configuration
    pub fn freeze(self, rcc_cfg: Config) -> Self {
        let pll_clk = self.config_pll(rcc_cfg.pll_cfg);

        let (sys_clk, sw_bits) = match rcc_cfg.sys_mux {
            SysClockSrc::HSE(freq) => {
                self.enable_hse(false);
                (freq, 0b001)
            }
            SysClockSrc::HSE_BYPASS(freq) => {
                self.enable_hse(true);
                (freq, 0b001)
            }
            SysClockSrc::PLL => (pll_clk.r, 0b010),
            SysClockSrc::LSE(freq) => {
                self.enable_lse(false);
                (freq, 0b100)
            }
            SysClockSrc::LSE_BYPASS(freq) => {
                self.enable_lse(true);
                (freq, 0b100)
            }
            SysClockSrc::LSI => {
                self.enable_lsi();
                (32_768.hz(), 0b011)
            }
            SysClockSrc::HSI(prs) => {
                self.enable_hsi();
                let (freq, div_bits) = match prs {
                    Prescaler::Div2 => (HSI_FREQ / 2, 0b001),
                    Prescaler::Div4 => (HSI_FREQ / 4, 0b010),
                    Prescaler::Div8 => (HSI_FREQ / 8, 0b011),
                    Prescaler::Div16 => (HSI_FREQ / 16, 0b100),
                    Prescaler::Div32 => (HSI_FREQ / 32, 0b101),
                    Prescaler::Div64 => (HSI_FREQ / 64, 0b110),
                    Prescaler::Div128 => (HSI_FREQ / 128, 0b111),
                    _ => (HSI_FREQ, 0b000),
                };
                self.cr.write(|w| unsafe { w.hsidiv().bits(div_bits) });
                (freq.hz(), 0b000)
            }
        };

        let sys_freq = sys_clk.0;
        let (ahb_freq, ahb_psc_bits) = match rcc_cfg.ahb_psc {
            Prescaler::Div2 => (sys_freq / 2, 0b1000),
            Prescaler::Div4 => (sys_freq / 4, 0b1001),
            Prescaler::Div8 => (sys_freq / 8, 0b1010),
            Prescaler::Div16 => (sys_freq / 16, 0b1011),
            Prescaler::Div64 => (sys_freq / 64, 0b1100),
            Prescaler::Div128 => (sys_freq / 128, 0b1101),
            Prescaler::Div256 => (sys_freq / 256, 0b1110),
            Prescaler::Div512 => (sys_freq / 512, 0b1111),
            _ => (sys_clk.0, 0b0000),
        };
        let (apb_freq, apb_tim_freq, apb_psc_bits) = match rcc_cfg.apb_psc {
            Prescaler::Div2 => (sys_freq / 2, sys_freq, 0b100),
            Prescaler::Div4 => (sys_freq / 4, sys_freq / 2, 0b101),
            Prescaler::Div8 => (sys_freq / 8, sys_freq / 4, 0b110),
            Prescaler::Div16 => (sys_freq / 16, sys_freq / 8, 0b111),
            _ => (sys_clk.0, sys_clk.0, 0b000),
        };

        unsafe {
            // Adjust flash wait states
            let flash = &(*FLASH::ptr());
            flash.acr.modify(|_, w| {
                w.latency().bits(if sys_clk.0 <= 24_000_000 {
                    0b000
                } else if sys_clk.0 <= 48_000_000 {
                    0b001
                } else {
                    0b010
                })
            })
        }

        self.cfgr.modify(|_, w| unsafe {
            w.hpre()
                .bits(ahb_psc_bits)
                .ppre()
                .bits(apb_psc_bits)
                .sw()
                .bits(sw_bits)
        });

        while self.cfgr.read().sws().bits() != sw_bits {}

        Rcc {
            rb: self.rb,
            clocks: Clocks {
                pll_clk,
                sys_clk,
                core_clk: (ahb_freq / 8).hz(),
                ahb_clk: ahb_freq.hz(),
                apb_clk: apb_freq.hz(),
                apb_tim_clk: apb_tim_freq.hz(),
            },
        }
    }

    pub fn set_reset_mode(&mut self, mode: ResetMode) {
        unsafe {
            let flash = &(*FLASH::ptr());

            // Unlock flash
            flash.keyr.write(|w| w.keyr().bits(0x4567_0123));
            flash.keyr.write(|w| w.keyr().bits(0xcdef_89ab));

            // Unlock flash OTP
            flash.optkeyr.write(|w| w.optkeyr().bits(0x0819_2a3b));
            flash.optkeyr.write(|w| w.optkeyr().bits(0x4c5d_6e7f));
            flash.cr.modify(|_, w| w.optlock().clear_bit());

            flash.optr.modify(|_, w| w.nrst_mode().bits(mode as u8));
            flash.cr.modify(|_, w| w.optstrt().set_bit());
        }
    }

    fn config_pll(&self, pll_cfg: PllConfig) -> PLLClocks {
        assert!(pll_cfg.m > 0 && pll_cfg.m <= 8);
        assert!(pll_cfg.r > 1 && pll_cfg.r <= 8);

        // Disable PLL
        self.cr.modify(|_, w| w.pllon().clear_bit());
        while self.cr.read().pllrdy().bit_is_set() {}

        let (freq, pll_sw_bits) = match pll_cfg.mux {
            PLLSrc::HSI => {
                self.enable_hsi();
                (HSI_FREQ, 0b10)
            }
            PLLSrc::HSE(freq) => {
                self.enable_hse(false);
                (freq.0, 0b11)
            }
            PLLSrc::HSE_BYPASS(freq) => {
                self.enable_hse(true);
                (freq.0, 0b11)
            }
        };

        let pll_freq = freq / (pll_cfg.m as u32) * (pll_cfg.n as u32);
        let r = (pll_freq / (pll_cfg.r as u32)).hz();
        let q = match pll_cfg.q {
            Some(div) if div > 1 && div <= 8 => {
                self.pllsyscfgr
                    .write(move |w| unsafe { w.pllq().bits(div - 1) });
                let req = pll_freq / div as u32;
                Some(req.hz())
            }
            _ => None,
        };

        let p = match pll_cfg.p {
            Some(div) if div > 1 && div <= 8 => {
                self.pllsyscfgr
                    .write(move |w| unsafe { w.pllp().bits(div - 1) });
                let req = pll_freq / div as u32;
                Some(req.hz())
            }
            _ => None,
        };

        self.pllsyscfgr.write(move |w| unsafe {
            w.pllsrc()
                .bits(pll_sw_bits)
                .pllm()
                .bits(pll_cfg.m - 1)
                .plln()
                .bits(pll_cfg.n)
                .pllr()
                .bits(pll_cfg.r - 1)
                .pllren()
                .set_bit()
        });

        // Enable PLL
        self.cr.modify(|_, w| w.pllon().set_bit());
        while self.cr.read().pllrdy().bit_is_clear() {}

        PLLClocks { r, q, p }
    }

    pub(crate) fn enable_hsi(&self) {
        self.cr.modify(|_, w| w.hsion().set_bit());
        while self.cr.read().hsirdy().bit_is_clear() {}
    }

    pub(crate) fn enable_hse(&self, bypass: bool) {
        self.cr
            .modify(|_, w| w.hseon().set_bit().hsebyp().bit(bypass));
        while self.cr.read().hserdy().bit_is_clear() {}
    }

    pub(crate) fn enable_lse(&self, bypass: bool) {
        self.bdcr
            .modify(|_, w| w.lseon().set_bit().lsebyp().bit(bypass));
        while self.bdcr.read().lserdy().bit_is_clear() {}
    }

    pub(crate) fn enable_lsi(&self) {
        self.csr.modify(|_, w| w.lsion().set_bit());
        while self.csr.read().lsirdy().bit_is_clear() {}
    }

    pub(crate) fn unlock_rtc(&self) {
        self.apbenr1.modify(|_, w| w.pwren().set_bit());
        let pwr = unsafe { &(*PWR::ptr()) };
        pwr.cr1.modify(|_, w| w.dbp().set_bit());
        while pwr.cr1.read().dbp().bit_is_clear() {}
    }

    pub(crate) fn enable_rtc(&self, src: RTCSrc) {
        match src {
            RTCSrc::LSI => self.enable_lsi(),
            RTCSrc::HSE => self.enable_hse(false),
            RTCSrc::LSE => self.enable_lse(false),
        }
        self.apbenr1
            .modify(|_, w| w.rtcapben().set_bit().pwren().set_bit());
        self.apbsmenr1.modify(|_, w| w.rtcapbsmen().set_bit());
        self.unlock_rtc();
        self.bdcr.modify(|_, w| w.bdrst().set_bit());
        self.bdcr.modify(|_, w| unsafe {
            w.rtcsel()
                .bits(src as u8)
                .rtcen()
                .set_bit()
                .bdrst()
                .clear_bit()
        });
    }
}

/// Extension trait that constrains the `RCC` peripheral
pub trait RccExt {
    /// Constrains the `RCC` peripheral so it plays nicely with the other abstractions
    fn constrain(self) -> Rcc;
    /// Constrains the `RCC` peripheral and apply clock configuration
    fn freeze(self, rcc_cfg: Config) -> Rcc;
}

impl RccExt for RCC {
    fn constrain(self) -> Rcc {
        Rcc {
            rb: self,
            clocks: Clocks::default(),
        }
    }

    fn freeze(self, rcc_cfg: Config) -> Rcc {
        self.constrain().freeze(rcc_cfg)
    }
}

/// Bus associated to peripheral
pub trait RccBus: crate::Sealed {
    /// Bus type;
    type Bus;
}

/// Enable/disable peripheral
pub trait Enable: RccBus {
    /// Enables peripheral
    fn enable(rcc: &mut Rcc);

    /// Disables peripheral
    fn disable(rcc: &mut Rcc);

    /// Check if peripheral enabled
    fn is_enabled() -> bool;

    /// Check if peripheral disabled
    fn is_disabled() -> bool;

    /// # Safety
    ///
    /// Enables peripheral. Takes access to RCC internally
    unsafe fn enable_unchecked();

    /// # Safety
    ///
    /// Disables peripheral. Takes access to RCC internally
    unsafe fn disable_unchecked();
}

/// Enable/disable peripheral in Sleep mode
pub trait SMEnable: RccBus {
    /// Enables peripheral
    fn sleep_mode_enable(rcc: &mut Rcc);

    /// Disables peripheral
    fn sleep_mode_disable(rcc: &mut Rcc);

    /// Check if peripheral enabled
    fn is_sleep_mode_enabled() -> bool;

    /// Check if peripheral disabled
    fn is_sleep_mode_disabled() -> bool;

    /// # Safety
    ///
    /// Enables peripheral. Takes access to RCC internally
    unsafe fn sleep_mode_enable_unchecked();

    /// # Safety
    ///
    /// Disables peripheral. Takes access to RCC internally
    unsafe fn sleep_mode_disable_unchecked();
}

/// Reset peripheral
pub trait Reset: RccBus {
    /// Resets peripheral
    fn reset(rcc: &mut Rcc);

    /// # Safety
    ///
    /// Resets peripheral. Takes access to RCC internally
    unsafe fn reset_unchecked();
}

use crate::stm32::rcc::RegisterBlock as RccRB;

macro_rules! bus_struct {
    ($($busX:ident => ($EN:ident, $en:ident, $SMEN:ident, $smen:ident, $RST:ident, $rst:ident, $doc:literal),)+) => {
        $(
            #[doc = $doc]
            pub struct $busX {
                _0: (),
            }

            impl $busX {
                #[inline(always)]
                fn enr(rcc: &RccRB) -> &rcc::$EN {
                    &rcc.$en
                }
                #[inline(always)]
                fn smenr(rcc: &RccRB) -> &rcc::$SMEN {
                    &rcc.$smen
                }
                #[inline(always)]
                fn rstr(rcc: &RccRB) -> &rcc::$RST {
                    &rcc.$rst
                }
            }
        )+
    };
}

bus_struct! {
    AHB => (AHBENR, ahbenr, AHBSMENR, ahbsmenr, AHBRSTR, ahbrstr, "AMBA High-performance Bus (AHB) registers"),
    APB1 => (APBENR1, apbenr1, APBSMENR1, apbsmenr1, APBRSTR1, apbrstr1, "Advanced Peripheral Bus 1 (APB1) registers"),
    APB2 => (APBENR2, apbenr2, APBSMENR2, apbsmenr2, APBRSTR2, apbrstr2, "Advanced Peripheral Bus 2 (APB2) registers"),
    IOP => (IOPENR, iopenr, IOPSMENR, iopsmenr, IOPRSTR, ioprstr, "Input-Output Peripheral Bus (IOP) registers"),
}
