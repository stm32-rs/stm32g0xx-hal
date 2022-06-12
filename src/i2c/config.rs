use crate::i2c::SlaveAddressMask;
use crate::time::Hertz;
use core::cmp;

pub struct Config {
    pub speed: Option<Hertz>,
    pub timing: Option<u32>,
    pub analog_filter: bool,
    pub digital_filter: u8,
    pub slave_address_1: u16,
    pub address_11bits: bool,
    pub slave_address_2: u8,
    pub slave_address_mask: SlaveAddressMask,
}

impl Config {
    pub fn new(speed: Hertz) -> Self {
        Config {
            speed: Some(speed),
            timing: None,
            analog_filter: true,
            digital_filter: 0,
            slave_address_1: 0,
            address_11bits: false,
            slave_address_2: 0,
            slave_address_mask: SlaveAddressMask::MaskNone,
        }
    }

    pub fn with_timing(timing: u32) -> Self {
        Config {
            timing: Some(timing),
            speed: None,
            analog_filter: true,
            digital_filter: 0,
            slave_address_1: 0,
            address_11bits: false,
            slave_address_2: 0,
            slave_address_mask: SlaveAddressMask::MaskNone,
        }
    }

    pub fn disable_analog_filter(mut self) -> Self {
        self.analog_filter = false;
        self
    }

    pub fn enable_digital_filter(mut self, cycles: u8) -> Self {
        assert!(cycles <= 16);
        self.digital_filter = cycles;
        self
    }

    pub fn timing_bits(&self, i2c_clk: Hertz) -> u32 {
        if let Some(bits) = self.timing {
            return bits;
        }
        let speed = self.speed.unwrap();
        let (psc, scll, sclh, sdadel, scldel) = if speed.raw() <= 100_000 {
            let psc = 3;
            let scll = cmp::min((((i2c_clk.raw() >> 1) / (psc + 1)) / speed.raw()) - 1, 255);
            let sclh = scll - 4;
            let sdadel = 2;
            let scldel = 4;
            (psc, scll, sclh, sdadel, scldel)
        } else {
            let psc = 1;
            let scll = cmp::min((((i2c_clk.raw() >> 1) / (psc + 1)) / speed.raw()) - 1, 255);
            let sclh = scll - 6;
            let sdadel = 1;
            let scldel = 3;
            (psc, scll, sclh, sdadel, scldel)
        };
        psc << 28 | scldel << 20 | sdadel << 16 | sclh << 8 | scll
    }
    /// Slave address 1 as 7 bit address, in range 0 .. 127
    pub fn slave_address(&mut self, own_address: u8) {
        //assert!(own_address < (2 ^ 7));
        self.slave_address_1 = own_address as u16;
        self.address_11bits = false;
    }
    /// Slave address 1 as 11 bit address in range 0 .. 2047
    pub fn slave_address_11bits(&mut self, own_address: u16) {
        //assert!(own_address < (2 ^ 11));
        self.slave_address_1 = own_address;
        self.address_11bits = true;
    }
    /// Slave address 2 as 7 bit address in range 0 .. 127.
    /// The mask makes all slaves within the mask addressable
    pub fn slave_address_2(&mut self, own_address: u8, mask: SlaveAddressMask) {
        //assert!(own_address < (2 ^ 7));
        self.slave_address_2 = own_address;
        self.slave_address_mask = mask;
    }
}

impl From<Hertz> for Config {
    fn from(speed: Hertz) -> Self {
        Config::new(speed)
    }
}
