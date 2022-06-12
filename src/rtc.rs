//! Real Time Clock
use crate::rcc::{RTCSrc, Rcc};
use crate::stm32::RTC;
use crate::time::*;

pub struct Rtc {
    rb: RTC,
}

impl Rtc {
    pub fn new(rtc: RTC, src: RTCSrc, rcc: &mut Rcc) -> Self {
        let mut rtc = Rtc { rb: rtc };
        rcc.enable_rtc(src);
        rtc.modify(|rb| {
            rb.cr.modify(|_, w| w.fmt().clear_bit());
        });
        rtc
    }

    fn modify<F>(&mut self, mut closure: F)
    where
        F: FnMut(&mut RTC),
    {
        // Disable write protection
        self.rb.wpr.write(|w| unsafe { w.bits(0xCA) });
        self.rb.wpr.write(|w| unsafe { w.bits(0x53) });
        // Enter init mode
        let isr = self.rb.icsr.read();
        if isr.initf().bit_is_clear() {
            self.rb.icsr.write(|w| w.init().set_bit());
            self.rb.icsr.write(|w| unsafe { w.bits(0xFFFF_FFFF) });
            while self.rb.icsr.read().initf().bit_is_clear() {}
        }
        // Invoke closure
        closure(&mut self.rb);
        // Exit init mode
        self.rb.icsr.write(|w| w.init().clear_bit());
        // Enable_write_protection
        self.rb.wpr.write(|w| unsafe { w.bits(0xFF) });
    }

    pub fn set_date(&mut self, date: &Date) {
        let (yt, yu) = bcd2_encode(date.year - 1970);
        let (mt, mu) = bcd2_encode(date.month);
        let (dt, du) = bcd2_encode(date.day);

        self.modify(|rb| {
            rb.dr.write(|w| unsafe {
                w.dt()
                    .bits(dt)
                    .du()
                    .bits(du)
                    .mt()
                    .bit(mt > 0)
                    .mu()
                    .bits(mu)
                    .yt()
                    .bits(yt)
                    .yu()
                    .bits(yu)
                    .wdu()
                    .bits(date.day as u8)
            });
        });
    }

    pub fn set_time(&mut self, time: &Time) {
        let (ht, hu) = bcd2_encode(time.hours);
        let (mnt, mnu) = bcd2_encode(time.minutes);
        let (st, su) = bcd2_encode(time.seconds);
        self.modify(|rb| {
            rb.tr.write(|w| unsafe {
                w.ht()
                    .bits(ht)
                    .hu()
                    .bits(hu)
                    .mnt()
                    .bits(mnt)
                    .mnu()
                    .bits(mnu)
                    .st()
                    .bits(st)
                    .su()
                    .bits(su)
                    .pm()
                    .clear_bit()
            });
            rb.cr.modify(|_, w| w.fmt().bit(time.daylight_savings));
        });
    }

    pub fn get_time(&self) -> Time {
        let timer = self.rb.tr.read();
        Time::new(
            bcd2_decode(timer.ht().bits(), timer.hu().bits()).hours(),
            bcd2_decode(timer.mnt().bits(), timer.mnu().bits()).minutes(),
            bcd2_decode(timer.st().bits(), timer.su().bits()).secs(),
            self.rb.cr.read().fmt().bit(),
        )
    }

    pub fn get_date(&self) -> Date {
        let date = self.rb.dr.read();
        Date::new(
            (bcd2_decode(date.yt().bits(), date.yu().bits()) + 1970).year(),
            bcd2_decode(date.mt().bit() as u8, date.mu().bits()).month(),
            bcd2_decode(date.dt().bits(), date.du().bits()).day(),
        )
    }

    pub fn get_week_day(&self) -> u8 {
        self.rb.dr.read().wdu().bits()
    }
}

pub trait RtcExt {
    fn constrain(self, rcc: &mut Rcc) -> Rtc;
}

impl RtcExt for RTC {
    fn constrain(self, rcc: &mut Rcc) -> Rtc {
        Rtc::new(self, RTCSrc::LSI, rcc)
    }
}

fn bcd2_encode(word: u32) -> (u8, u8) {
    let mut value = word as u8;
    let mut bcd_high: u8 = 0;
    while value >= 10 {
        bcd_high += 1;
        value -= 10;
    }
    let bcd_low = ((bcd_high << 4) | value) as u8;
    (bcd_high, bcd_low)
}

fn bcd2_decode(fst: u8, snd: u8) -> u32 {
    let value = snd | fst << 4;
    let value = (value & 0x0F) + ((value & 0xF0) >> 4) * 10;
    value as u32
}
