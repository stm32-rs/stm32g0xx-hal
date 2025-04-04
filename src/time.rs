pub use fugit::{
    ExtU32, HertzU32 as Hertz, HoursDurationU32 as Hour, MicrosDurationU32 as MicroSecond,
    MinutesDurationU32 as Minute, RateExtU32, SecsDurationU32 as Second,
};

/// Baudrate
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, Eq, PartialEq, PartialOrd, Clone, Copy)]
pub struct Bps(pub u32);

/// A measurement of a monotonically nondecreasing clock
pub type Instant = fugit::TimerInstantU32<1_000_000>;

/// WeekDay (1-7)
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WeekDay(pub u32);

/// Date (1-31)
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MonthDay(pub u32);

/// Week (1-52)
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Week(pub u32);

/// Month (1-12)
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Month(pub u32);

/// Year
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Year(pub u32);

#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Time {
    pub hours: u32,
    pub minutes: u32,
    pub seconds: u32,
    pub daylight_savings: bool,
}

impl Time {
    pub fn new(hours: Hour, minutes: Minute, seconds: Second, daylight_savings: bool) -> Self {
        Self {
            hours: hours.ticks(),
            minutes: minutes.ticks(),
            seconds: seconds.ticks(),
            daylight_savings,
        }
    }
}

#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

pub fn duration(hz: Hertz, cycles: u32) -> MicroSecond {
    let cycles = cycles as u64;
    let clk = hz.raw() as u64;
    let us = cycles.saturating_mul(1_000_000_u64) / clk;
    MicroSecond::from_ticks(us as u32)
}

pub fn cycles(ms: MicroSecond, clk: Hertz) -> u32 {
    assert!(ms.ticks() > 0);
    let clk = clk.raw() as u64;
    let period = ms.ticks() as u64;
    let cycles = clk.saturating_mul(period) / 1_000_000_u64;
    cycles as u32
}
