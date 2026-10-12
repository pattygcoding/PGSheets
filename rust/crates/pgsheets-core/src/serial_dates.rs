//! The 1900 serial date system: the count of days a spreadsheet stores a date as, with the time of
//! day held in the fraction.
//!
//! Excel keeps 1900-02-29 as serial 60 even though 1900 was not a leap year, so that files written by
//! Lotus 1-2-3 still line up. No such day exists in the Gregorian calendar: serial 60 therefore reads
//! as 1900-03-01, the same date as serial 61. That one ambiguous serial is the only place this codec
//! is not exact; every real date in the system converts both ways.
//!
//! The calendar arithmetic is Hinnant's `days_from_civil` / `civil_from_days`, which is the proleptic
//! Gregorian calendar — the same one .NET's `DateTime` uses — in about twenty lines.

/// Milliseconds in a day, the resolution .NET's `DateTime.AddDays` rounds a serial to.
const MILLIS_PER_DAY: i64 = 86_400_000;

/// A decoded date and time of day.
///
/// Milliseconds are the resolution because that is what .NET's `AddDays` produces: it rounds the
/// fraction of the serial to the nearest millisecond before building the date.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CivilDateTime {
    /// Calendar year, 1 to 9999.
    pub year: i32,

    /// Month, 1 to 12.
    pub month: u32,

    /// Day of the month, 1 to 31.
    pub day: u32,

    /// Hour, 0 to 23.
    pub hour: u32,

    /// Minute, 0 to 59.
    pub minute: u32,

    /// Second, 0 to 59.
    pub second: u32,

    /// Millisecond, 0 to 999.
    pub millisecond: u32,

    /// Day of the week, 0 for Sunday, matching .NET's `DayOfWeek`.
    pub weekday: u32,
}

impl CivilDateTime {
    /// A date at midnight, with the day of the week filled in.
    pub fn date(year: i32, month: u32, day: u32) -> Self {
        Self {
            year,
            month,
            day,
            hour: 0,
            minute: 0,
            second: 0,
            millisecond: 0,
            weekday: weekday_from_days(days_from_civil(year, month, day)),
        }
    }

    /// The same date, at a time of day.
    pub fn at(mut self, hour: u32, minute: u32, second: u32, millisecond: u32) -> Self {
        self.hour = hour;
        self.minute = minute;
        self.second = second;
        self.millisecond = millisecond;
        self
    }

    /// The fraction of a day this time of day represents.
    fn time_of_day_fraction(self) -> f64 {
        let millis = i64::from(self.hour) * 3_600_000
            + i64::from(self.minute) * 60_000
            + i64::from(self.second) * 1_000
            + i64::from(self.millisecond);

        millis as f64 / MILLIS_PER_DAY as f64
    }
}

/// The date a serial number stands for.
///
/// `None` for a serial that is negative, is not finite, or would land outside the calendar — all of
/// which a spreadsheet reports as `#NUM!`.
pub fn try_to_datetime(serial: f64) -> Option<CivilDateTime> {
    if !serial.is_finite() || serial < 0.0 {
        return None;
    }

    // Counting from a day later below the phantom day is what makes 1 = 1900-01-01 and 59 =
    // 1900-02-28. Serial 60 is the day Excel calls 1900-02-29, one day after the last real date
    // counted that way; with no such date to return it lands on 1900-03-01, which is also what
    // serial 61 is. That single ambiguous day is the price of the phantom leap year.
    let base_days = if serial.floor() < 61.0 {
        days_from_civil(1899, 12, 31)
    } else {
        days_from_civil(1899, 12, 30)
    };

    // .NET adds `(long)(value * millisPerDay + 0.5)` milliseconds, so the rounding happens before the
    // day is split off — a serial just under a whole day can carry into the next one.
    let total_millis = (serial * MILLIS_PER_DAY as f64 + 0.5).floor();

    if !total_millis.is_finite() || total_millis < 0.0 || total_millis > i64::MAX as f64 {
        return None;
    }

    let total_millis = total_millis as i64;
    let day_index = base_days + total_millis.div_euclid(MILLIS_PER_DAY);
    let time_millis = total_millis.rem_euclid(MILLIS_PER_DAY);

    let (year, month, day) = civil_from_days(day_index);

    // Past 9999-12-31 there is no date to return. .NET's `AddDays` throws here; refusing is the safe
    // half of the same behaviour.
    if year > 9999 {
        return None;
    }

    Some(CivilDateTime {
        year,
        month,
        day,
        hour: (time_millis / 3_600_000) as u32,
        minute: ((time_millis / 60_000) % 60) as u32,
        second: ((time_millis / 1_000) % 60) as u32,
        millisecond: (time_millis % 1_000) as u32,
        weekday: weekday_from_days(day_index),
    })
}

/// The serial number for a date and time of day, in the 1900 date system.
pub fn to_serial(date: CivilDateTime) -> f64 {
    // "Before the phantom day" is a property of the whole instant, not just of its date.
    let before_phantom = (date.year, date.month, date.day) < (1900, 3, 1);
    let base_days = if before_phantom {
        days_from_civil(1899, 12, 31)
    } else {
        days_from_civil(1899, 12, 30)
    };

    let days = days_from_civil(date.year, date.month, date.day) - base_days;
    days as f64 + date.time_of_day_fraction()
}

/// The English month name, which is what the invariant culture uses in a number format.
pub fn month_name(month: u32) -> &'static str {
    const NAMES: [&str; 12] = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];

    NAMES.get(month.saturating_sub(1) as usize).copied().unwrap_or("")
}

/// The English day name, Sunday first.
pub fn day_name(weekday: u32) -> &'static str {
    const NAMES: [&str; 7] = [
        "Sunday",
        "Monday",
        "Tuesday",
        "Wednesday",
        "Thursday",
        "Friday",
        "Saturday",
    ];

    NAMES.get(weekday as usize % 7).copied().unwrap_or("")
}

/// Days since 1970-01-01 for a proleptic Gregorian date.
pub(crate) fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let year = i64::from(year) - i64::from(month <= 2);
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let month = i64::from(month);
    let day_of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + i64::from(day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;

    era * 146_097 + day_of_era - 719_468
}

/// The proleptic Gregorian date for days since 1970-01-01.
pub(crate) fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted - era * 146_097;
    let year_of_era = (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * month_index + 2) / 5 + 1) as u32;
    let month = (month_index + if month_index < 10 { 3 } else { -9 }) as u32;

    ((year + i64::from(month <= 2)) as i32, month, day)
}

/// Day of the week for days since 1970-01-01, with 0 for Sunday: 1970-01-01 was a Thursday.
fn weekday_from_days(days: i64) -> u32 {
    (days + 4).rem_euclid(7) as u32
}
