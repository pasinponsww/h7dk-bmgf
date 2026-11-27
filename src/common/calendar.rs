//! Dates: how many days the two of you have been together, and whether
//! today is your anniversary.
//!
//! The board's real-time clock gives today's date. The story's optional
//! `[together] since = "YYYY-MM-DD"` says when it all began. The firmware
//! is built with the PC's clock (`H7_NOW`, set by `cargo proj mbedded ...`)
//! so the real-time clock can be set when it has lost the time.

include!(concat!(env!("OUT_DIR"), "/clock.rs"));

/// A day in the (proleptic Gregorian) calendar.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Date {
    /// The year, e.g. 2025.
    pub year: u16,
    /// 1 to 12.
    pub month: u8,
    /// 1 to 31.
    pub day: u8,
}

/// A moment, to the second.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DateTime {
    /// The day.
    pub date: Date,
    /// 0 to 23.
    pub hour: u8,
    /// 0 to 59.
    pub minute: u8,
    /// 0 to 59.
    pub second: u8,
}

impl Date {
    /// A date, if it is a real one.
    pub const fn new(year: u16, month: u8, day: u8) -> Option<Self> {
        if month < 1 || month > 12 || day < 1 || day > days_in_month(year, month) {
            return None;
        }
        Some(Self { year, month, day })
    }

    /// Days since 1 January 1970 (negative before it), by Howard Hinnant's
    /// `days_from_civil`.
    pub const fn days_since_epoch(self) -> i32 {
        let m = self.month as i32;
        let d = self.day as i32;
        let y = self.year as i32 - if m <= 2 { 1 } else { 0 };
        let era = (if y >= 0 { y } else { y - 399 }) / 400;
        let yoe = y - era * 400;
        let mp = (m + 9) % 12;
        let doy = (153 * mp + 2) / 5 + d - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146_097 + doe - 719_468
    }
}

const fn is_leap(year: u16) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

const fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        2 if is_leap(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Which day of being together `today` is, counting the first day as day 1.
/// `None` before it began.
pub fn day_together(since: Date, today: Date) -> Option<u32> {
    let days = today.days_since_epoch() - since.days_since_epoch();
    u32::try_from(days).ok().map(|d| d + 1)
}

/// How many years it has been if `today` is an anniversary of `since` (a
/// 29 February start counts on 28 February in other years). `None` on any
/// other day, and on the first day itself.
pub fn anniversary(since: Date, today: Date) -> Option<u16> {
    let years = today.year.checked_sub(since.year).filter(|&y| y > 0)?;
    let same_day = today.month == since.month && today.day == since.day;
    let leap_start = since.month == 2 && since.day == 29;
    let stand_in = leap_start && !is_leap(today.year) && today.month == 2 && today.day == 28;
    (same_day || stand_in).then_some(years)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: u16, m: u8, day: u8) -> Date {
        Date::new(y, m, day).unwrap()
    }

    #[test]
    fn only_real_dates_are_dates() {
        assert!(Date::new(2024, 2, 29).is_some(), "a leap day");
        assert!(Date::new(2025, 2, 29).is_none());
        assert!(Date::new(1900, 2, 29).is_none(), "not a leap year");
        assert!(Date::new(2000, 2, 29).is_some(), "but this one is");
        assert!(Date::new(2025, 4, 31).is_none());
        assert!(Date::new(2025, 13, 1).is_none());
        assert!(Date::new(2025, 1, 0).is_none());
    }

    #[test]
    fn days_are_counted_from_1970() {
        assert_eq!(d(1970, 1, 1).days_since_epoch(), 0);
        assert_eq!(d(1970, 1, 2).days_since_epoch(), 1);
        assert_eq!(d(2000, 3, 1).days_since_epoch(), 11_017);
        assert_eq!(d(2026, 11, 26).days_since_epoch(), 20_783);
        assert_eq!(d(1969, 12, 31).days_since_epoch(), -1);
    }

    #[test]
    fn the_first_day_is_day_one() {
        let since = d(2024, 2, 14);
        assert_eq!(day_together(since, since), Some(1));
        assert_eq!(day_together(since, d(2024, 2, 15)), Some(2));
        assert_eq!(
            day_together(since, d(2025, 2, 14)),
            Some(367),
            "2024 was a leap year"
        );
        assert_eq!(day_together(since, d(2024, 2, 13)), None, "not yet");
    }

    #[test]
    fn anniversaries_fall_on_the_same_day_each_year() {
        let since = d(2024, 2, 14);
        assert_eq!(anniversary(since, d(2025, 2, 14)), Some(1));
        assert_eq!(anniversary(since, d(2030, 2, 14)), Some(6));
        assert_eq!(anniversary(since, d(2024, 2, 14)), None, "the first day");
        assert_eq!(anniversary(since, d(2025, 2, 15)), None);
        let leap = d(2024, 2, 29);
        assert_eq!(anniversary(leap, d(2025, 2, 28)), Some(1));
        assert_eq!(anniversary(leap, d(2028, 2, 29)), Some(4));
        assert_eq!(
            anniversary(leap, d(2028, 2, 28)),
            None,
            "it has its own day then"
        );
    }

    #[test]
    fn moments_compare_in_time_order() {
        let a = DateTime {
            date: d(2026, 1, 1),
            hour: 23,
            minute: 59,
            second: 59,
        };
        let b = DateTime {
            date: d(2026, 1, 2),
            hour: 0,
            minute: 0,
            second: 0,
        };
        assert!(a < b);
    }
}
