use crate::stm32::RCC;
use crate::time::{Hertz, U32Ext};

/// Extension trait that constrains the `RCC` peripheral
pub trait RccExt {
  /// Constrains the `RCC` peripheral so it plays nicely with the other abstractions
  fn constrain(self) -> Rcc;
}

/// Constrained RCC peripheral
pub struct Rcc {
  pub cfgr: Config,
}

/// System clock mux source
pub enum ClockSrc {
  LSI,
  HSI,
  HSE(Hertz),
  LSE(Hertz),
  PLL(PLLS, PLLMul, PLLDiv),
}

/// PLL divider
#[derive(Clone, Copy)]
pub enum PLLDiv {
  Div2 = 1,
  Div3 = 2,
  Div4 = 3,
}

/// PLL multiplier
#[derive(Clone, Copy)]
pub enum PLLMul {
  Mul3 = 0,
  Mul4 = 1,
  Mul6 = 2,
  Mul8 = 3,
  Mul12 = 4,
  Mul16 = 5,
  Mul24 = 6,
  Mul32 = 7,
  Mul48 = 8,
}

/// AHB prescaler
#[derive(Clone, Copy)]
pub enum AHBPsc {
  NotDivided = 0,
  Div2 = 0b1000,
  Div4 = 0b1001,
  Div8 = 0b1010,
  Div16 = 0b1011,
  Div64 = 0b1100,
  Div128 = 0b1101,
  Div256 = 0b1110,
  Div512 = 0b1111,
}

/// APB prescaler
#[derive(Clone, Copy)]
pub enum APBPsc {
  NotDivided = 0,
  Div2 = 0b100,
  Div4 = 0b101,
  Div8 = 0b110,
  Div16 = 0b111,
}

/// PLL clock input source
#[derive(Clone, Copy)]
pub enum PLLS {
  HSI,
  HSE(Hertz),
}

/// HSI speed
pub const HSI_FREQ: u32 = 15_998_976;

/// Clocks configutation
pub struct Config {
  mux: ClockSrc,
  ahb_psc: AHBPsc,
  apb1_psc: APBPsc,
  apb2_psc: APBPsc,
}

impl Default for Config {
  fn default() -> Config {
    Config {
      mux: ClockSrc::HSI,
      ahb_psc: AHBPsc::NotDivided,
      apb1_psc: APBPsc::NotDivided,
      apb2_psc: APBPsc::NotDivided,
    }
  }
}

impl Config {
  pub fn clock_src(mut self, mux: ClockSrc) -> Self {
    self.mux = mux;
    self
  }

  pub fn ahb_psc(mut self, psc: AHBPsc) -> Self {
    self.ahb_psc = psc;
    self
  }

  pub fn apb1_psc(mut self, psc: APBPsc) -> Self {
    self.apb1_psc = psc;
    self
  }

  pub fn apb2_psc(mut self, psc: APBPsc) -> Self {
    self.apb2_psc = psc;
    self
  }

  pub fn freeze(self) -> Clocks {
    let rcc = unsafe { &*RCC::ptr() };
    let (sys_clk, sw_bits) = match self.mux {
      _ => unimplemented!(),
      ClockSrc::HSI => {
        // Enable HSI
        rcc.cr.write(|w| w.hsion().set_bit());
        while rcc.cr.read().hsirdy().bit_is_clear() {}

        (HSI_FREQ, 1)
      }
      ClockSrc::HSE(freq) => {
        // Enable HSE
        rcc.cr.write(|w| w.hseon().set_bit());
        while rcc.cr.read().hserdy().bit_is_clear() {}

        (freq.0, 2)
      }
      ClockSrc::PLL(src, mul, div) => {
        let (src_bit, freq) = match src {
          PLLS::HSE(freq) => {
            // Enable HSE
            rcc.cr.write(|w| w.hseon().set_bit());
            while rcc.cr.read().hserdy().bit_is_clear() {}
            (true, freq.0)
          }
          PLLS::HSI => {
            // Enable HSI
            rcc.cr.write(|w| w.hsion().set_bit());
            while rcc.cr.read().hsirdy().bit_is_clear() {}
            (false, 15_998_976)
          }
        };

        // Disable PLL
        rcc.cr.write(|w| w.pllon().clear_bit());
        while rcc.cr.read().pllrdy().bit_is_set() {}

        let mul_bytes = mul as u8;
        let div_bytes = div as u8;

        let freq = match mul {
          PLLMul::Mul3 => freq * 3,
          PLLMul::Mul4 => freq * 4,
          PLLMul::Mul6 => freq * 6,
          PLLMul::Mul8 => freq * 8,
          PLLMul::Mul12 => freq * 12,
          PLLMul::Mul16 => freq * 16,
          PLLMul::Mul24 => freq * 24,
          PLLMul::Mul32 => freq * 32,
          PLLMul::Mul48 => freq * 48,
        };

        let freq = match div {
          PLLDiv::Div2 => freq / 2,
          PLLDiv::Div3 => freq / 3,
          PLLDiv::Div4 => freq / 4,
        };
        assert!(freq <= 24.mhz().0);

        // rcc.cfgr.write(move |w| unsafe {
        //   w.pllmul()
        //     .bits(mul_bytes)
        //     .plldiv()
        //     .bits(div_bytes)
        //     .pllsrc()
        //     .bit(src_bit)
        // });

        // Enable PLL
        rcc.cr.write(|w| w.pllon().set_bit());
        while rcc.cr.read().pllrdy().bit_is_clear() {}

        (freq, 3)
      }
    };

    // rcc.cfgr.modify(|_, w| unsafe {
    //   w.sw()
    //     .bits(sw_bits)
    //     .hpre()
    //     .bits(self.ahb_psc as u8)
    //     .ppre1()
    //     .bits(self.apb1_psc as u8)
    //     .ppre2()
    //     .bits(self.apb2_psc as u8)
    // });

    let ahb_freq = match self.ahb_psc {
      AHBPsc::NotDivided => sys_clk,
      pre => sys_clk / (1 << (pre as u8 - 7)),
    };

    let (apb1_freq, apb1_tim_freq) = match self.apb1_psc {
      APBPsc::NotDivided => (ahb_freq, ahb_freq),
      pre => {
        let freq = ahb_freq / (1 << (pre as u8 - 3));
        (freq, freq * 2)
      }
    };

    let (apb2_freq, apb2_tim_freq) = match self.apb2_psc {
      APBPsc::NotDivided => (ahb_freq, ahb_freq),
      pre => {
        let freq = ahb_freq / (1 << (pre as u8 - 3));
        (freq, freq * 2)
      }
    };

    Clocks {
      sys_clk: sys_clk.hz(),
      ahb_clk: ahb_freq.hz(),
      apb1_clk: apb1_freq.hz(),
      apb2_clk: apb2_freq.hz(),
      apb1_tim_clk: apb1_tim_freq.hz(),
      apb2_tim_clk: apb2_tim_freq.hz(),
    }
  }
}

/// Frozen clock frequencies
///
/// The existence of this value indicates that the clock configuration can no longer be changed
#[derive(Clone, Copy)]
pub struct Clocks {
  sys_clk: Hertz,
  ahb_clk: Hertz,
  apb1_clk: Hertz,
  apb1_tim_clk: Hertz,
  apb2_clk: Hertz,
  apb2_tim_clk: Hertz,
}

impl Clocks {
  /// Returns the system (core) frequency
  pub fn sys_clk(&self) -> Hertz {
    self.sys_clk
  }

  /// Returns the frequency of the AHB
  pub fn ahb_clk(&self) -> Hertz {
    self.ahb_clk
  }

  /// Returns the frequency of the APB1
  pub fn apb1_clk(&self) -> Hertz {
    self.apb1_clk
  }

  /// Returns the frequency of the APB1 timers
  pub fn apb1_tim_clk(&self) -> Hertz {
    self.apb1_tim_clk
  }

  /// Returns the frequency of the APB2
  pub fn apb2_clk(&self) -> Hertz {
    self.apb2_clk
  }

  /// Returns the frequency of the APB2 timers
  pub fn apb2_tim_clk(&self) -> Hertz {
    self.apb2_tim_clk
  }
}

impl RccExt for RCC {
  fn constrain(self) -> Rcc {
    Rcc {
      cfgr: Config::default(),
    }
  }
}
