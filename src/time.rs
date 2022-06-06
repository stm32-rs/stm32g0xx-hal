use core::ops::{Add, Div};

/// A measurement of a monotonically nondecreasing clock
#[derive(Debug, PartialEq, PartialOrd, Clone, Copy)]
pub struct Instant(pub u32);

/// Baudrate
#[derive(Debug, PartialEq, PartialOrd, Clone, Copy)]
pub struct Bps(pub u32);

/// Hertz
#[derive(Debug, PartialEq, PartialOrd, Clone, Copy)]
pub struct Hertz(pub u32);

/// Microseconds
#[derive(Debug, PartialEq, PartialOrd, Clone, Copy)]
pub struct MicroSecond(pub u32);

/// Seconds
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Second(pub u32);

/// Minutes
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Minute(pub u32);

/// Hours
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hour(pub u32);

/// WeekDay (1-7)
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WeekDay(pub u32);

/// Date (1-31)
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MonthDay(pub u32);

/// Week (1-52)
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Week(pub u32);

/// Month (1-12)
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Month(pub u32);

/// Year
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Year(pub u32);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Time {
    pub hours: u32,
    pub minutes: u32,
    pub seconds: u32,
    pub daylight_savings: bool,
}

impl Time {
    pub fn new(hours: Hour, minutes: Minute, seconds: Second, daylight_savings: bool) -> Self {
        Self {
            hours: hours.0,
            minutes: minutes.0,
            seconds: seconds.0,
            daylight_savings,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Date {
    pub day: u32,
    pub month: u32,
    pub year: u32,
}

impl Date {
    pub fn new(year: Year, month: Month, day: MonthDay) -> Self {
        Self {
            day: day.0,
            month: month.0,
            year: year.0,
        }
    }
}

pub trait U32Ext {
    /// Wrap in `Bps`
    fn bps(self) -> Bps;

    /// Wrap in `Hertz`
    fn hz(self) -> Hertz;

    /// Wrap in `Hertz`
    fn khz(self) -> Hertz;

    /// Wrap in `Hertz`
    fn mhz(self) -> Hertz;

    /// Wrap in `MicroSecond`
    fn us(self) -> MicroSecond;

    /// Wrap in `MicroSecond`
    fn ms(self) -> MicroSecond;

    /// Seconds
    fn seconds(self) -> Second;

    /// Minutes
    fn minutes(self) -> Minute;

    /// Hours
    fn hours(self) -> Hour;

    /// Day in month
    fn day(self) -> MonthDay;

    /// Month
    fn month(self) -> Month;

    /// Year
    fn year(self) -> Year;
}

impl U32Ext for u32 {
    fn bps(self) -> Bps {
        assert!(self > 0);
        Bps(self)
    }

    fn hz(self) -> Hertz {
        assert!(self > 0);
        Hertz(self)
    }

    fn khz(self) -> Hertz {
        Hertz(self.saturating_mul(1_000))
    }

    fn mhz(self) -> Hertz {
        Hertz(self.saturating_mul(1_000_000))
    }

    fn ms(self) -> MicroSecond {
        MicroSecond(self.saturating_mul(1_000))
    }

    fn us(self) -> MicroSecond {
        MicroSecond(self)
    }

    fn seconds(self) -> Second {
        Second(self)
    }

    fn minutes(self) -> Minute {
        Minute(self)
    }

    fn hours(self) -> Hour {
        Hour(self)
    }

    fn day(self) -> MonthDay {
        MonthDay(self)
    }

    fn month(self) -> Month {
        Month(self)
    }

    fn year(self) -> Year {
        Year(self)
    }
}

impl Hertz {
    pub fn duration(self, cycles: u32) -> MicroSecond {
        let cycles = cycles as u64;
        let clk = self.0 as u64;
        let us = cycles.saturating_mul(1_000_000_u64) / clk;
        MicroSecond(us as u32)
    }
}

impl Add for Hertz {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        Self(self.0 + other.0)
    }
}

impl Div for Hertz {
    type Output = u32;

    fn div(self, other: Self) -> Self::Output {
        self.0 / other.0
    }
}

impl MicroSecond {
    pub fn cycles(self, clk: Hertz) -> u32 {
        assert!(self.0 > 0);
        let clk = clk.0 as u64;
        let period = self.0 as u64;
        let cycles = clk.saturating_mul(period) / 1_000_000_u64;
        cycles as u32
    }
}

impl Add for MicroSecond {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        Self(self.0 + other.0)
    }
}

impl From<Second> for MicroSecond {
    fn from(period: Second) -> MicroSecond {
        MicroSecond(period.0 * 1_000_000)
    }
}

impl From<Hertz> for MicroSecond {
    fn from(freq: Hertz) -> MicroSecond {
        assert!(freq.0 <= 1_000_000);
        MicroSecond(1_000_000 / freq.0)
    }
}

impl From<MicroSecond> for Hertz {
    fn from(period: MicroSecond) -> Hertz {
        assert!(period.0 > 0 && period.0 <= 1_000_000);
        Hertz(1_000_000 / period.0)
    }
}
