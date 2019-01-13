use cast::u32;

use crate::stm32::RCC;
use crate::time::{Hertz, U32Ext};

/// HSI speed
pub const HSI_FREQ: u32 = 16_000_000;

/// Prescaler
#[derive(Clone, Copy)]
pub enum Prescaler {
    NotDivided,
    Div2,
    Div4,
    Div8,
    Div16,
    Div32,
    Div64,
    Div128,
    Div256,
    Div512,
}

/// System clock mux source
pub enum SysClockSrc {
    LSI,
    PLL,
    HSI(Prescaler),
    HSE(Hertz),
    HSE_BYPASS(Hertz),
    LSE(Hertz),
    LSE_BYPASS(Hertz),
}

/// PLL clock input source
#[derive(Clone, Copy)]
pub enum PLLSrc {
    HSI,
    HSE(Hertz),
    HSE_BYPASS(Hertz),
}

/// PLL divider
pub type PLLDiv = u8;

/// PLL multiplier
pub type PLLMul = u8;

/// PLL config
#[derive(Clone, Copy)]
pub struct PllConfig {
    mux: PLLSrc,
    m: PLLDiv,
    n: PLLMul,
    r: PLLDiv,
    q: Option<PLLDiv>,
    p: Option<PLLDiv>,
}

impl Default for PllConfig {
    fn default() -> PllConfig {
        PllConfig {
            mux: PLLSrc::HSI,
            m: 1,
            n: 8,
            r: 2,
            q: None,
            p: None,
        }
    }
}

/// Clocks configutation
pub struct RccConfig {
    sys_mux: SysClockSrc,
    pll_cfg: PllConfig,
    ahb_psc: Prescaler,
    apb_psc: Prescaler,
}

impl RccConfig {
    pub fn new(mux: SysClockSrc) -> Self {
        RccConfig::default().clock_src(mux)
    }

    pub fn clock_src(mut self, mux: SysClockSrc) -> Self {
        self.sys_mux = mux;
        self
    }

    pub fn pll_cfg(mut self, cfg: PllConfig) -> Self {
        self.pll_cfg = cfg;
        self
    }

    pub fn ahb_psc(mut self, psc: Prescaler) -> Self {
        self.ahb_psc = psc;
        self
    }

    pub fn apb_psc(mut self, psc: Prescaler) -> Self {
        self.apb_psc = psc;
        self
    }
}

impl Default for RccConfig {
    fn default() -> RccConfig {
        RccConfig {
            sys_mux: SysClockSrc::HSI(Prescaler::NotDivided),
            pll_cfg: PllConfig::default(),
            ahb_psc: Prescaler::NotDivided,
            apb_psc: Prescaler::NotDivided,
        }
    }
}

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

/// Constrained RCC peripheral
pub struct Rcc {
    /// Clock configuration
    pub clocks: Clocks,
    pub(crate) rb: RCC,
}

impl Rcc {
    /// Apply clock configuration
    pub fn freeze(self, rcc_cfg: RccConfig) -> Self {
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
                self.rb.cr.write(|w| unsafe { w.hsidiv().bits(div_bits) });
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

        self.rb.cfgr.modify(|_, w| unsafe {
            w.hpre()
                .bits(ahb_psc_bits)
                .ppre()
                .bits(apb_psc_bits)
                .sw()
                .bits(sw_bits)
        });

        while self.rb.cfgr.read().sws().bits() != sw_bits {}

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

    fn config_pll(&self, pll_cfg: PllConfig) -> PLLClocks {
        assert!(pll_cfg.m > 0 && pll_cfg.m <= 8);
        assert!(pll_cfg.r > 1 && pll_cfg.r <= 8);

        // Disable PLL
        self.rb.cr.write(|w| w.pllon().clear_bit());
        while self.rb.cr.read().pllrdy().bit_is_set() {}

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

        let pll_freq = freq / u32(pll_cfg.m) * u32(pll_cfg.n);
        let r = (pll_freq / u32(pll_cfg.r)).hz();
        let q = match pll_cfg.q {
            Some(div) if div > 1 && div <= 8 => {
                self.rb
                    .pllsyscfgr
                    .write(move |w| unsafe { w.pllq().bits(div - 1) });
                let req = freq / u32(div);
                Some(req.hz())
            }
            _ => None,
        };

        let p = match pll_cfg.p {
            Some(div) if div > 1 && div <= 8 => {
                self.rb
                    .pllsyscfgr
                    .write(move |w| unsafe { w.pllp().bits(div - 1) });
                let req = freq / u32(div);
                Some(req.hz())
            }
            _ => None,
        };

        self.rb.pllsyscfgr.write(move |w| unsafe {
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
        self.rb.cr.write(|w| w.pllon().set_bit());
        while self.rb.cr.read().pllrdy().bit_is_clear() {}

        PLLClocks { r, q, p }
    }

    fn enable_hsi(&self) {
        self.rb.cr.write(|w| w.hsion().set_bit());
        while self.rb.cr.read().hsirdy().bit_is_clear() {}
    }

    fn enable_hse(&self, bypass: bool) {
        self.rb
            .cr
            .write(|w| w.hseon().set_bit().hsebyp().bit(bypass));
        while self.rb.cr.read().hserdy().bit_is_clear() {}
    }

    fn enable_lse(&self, bypass: bool) {
        self.rb
            .bdcr
            .write(|w| w.lseon().set_bit().lsebyp().bit(bypass));
        while self.rb.bdcr.read().lserdy().bit_is_clear() {}
    }

    fn enable_lsi(&self) {
        self.rb.csr.write(|w| w.lsion().set_bit());
        while self.rb.csr.read().lsirdy().bit_is_clear() {}
    }
}

/// Extension trait that constrains the `RCC` peripheral
pub trait RccExt {
    /// Constrains the `RCC` peripheral so it plays nicely with the other abstractions
    fn constrain(self) -> Rcc;
    /// Constrains the `RCC` peripheral and apply clock configuration
    fn freeze(self, rcc_cfg: RccConfig) -> Rcc;
}

impl RccExt for RCC {
    fn constrain(self) -> Rcc {
        Rcc {
            rb: self,
            clocks: Clocks::default(),
        }
    }

    fn freeze(self, rcc_cfg: RccConfig) -> Rcc {
        self.constrain().freeze(rcc_cfg)
    }
}
