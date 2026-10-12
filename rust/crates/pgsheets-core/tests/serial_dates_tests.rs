//! The serial-number date system.
//!
//! A wrong answer here would move every date in every file, so the round trip is checked day by day
//! across two centuries rather than at a handful of points.

use pgsheets_core::serial_dates::{CivilDateTime, to_serial, try_to_datetime};

#[test]
fn a_serial_becomes_the_date_it_stands_for() {
    for (serial, expected) in [
        (45292.0, CivilDateTime::date(2024, 1, 1)),
        (45356.0, CivilDateTime::date(2024, 3, 5)),
        (43831.0, CivilDateTime::date(2020, 1, 1)),
        (1.0, CivilDateTime::date(1900, 1, 1)),
        (59.0, CivilDateTime::date(1900, 2, 28)),
        (61.0, CivilDateTime::date(1900, 3, 1)),
    ] {
        assert_eq!(try_to_datetime(serial), Some(expected), "serial {serial}");
    }
}

#[test]
fn a_date_becomes_the_serial_it_stands_for() {
    for (date, expected) in [
        (CivilDateTime::date(2024, 1, 1), 45292.0),
        (CivilDateTime::date(2024, 3, 5), 45356.0),
        (CivilDateTime::date(2020, 1, 1), 43831.0),
        (CivilDateTime::date(1900, 1, 1), 1.0),
        (CivilDateTime::date(1900, 2, 28), 59.0),
        (CivilDateTime::date(1900, 3, 1), 61.0),
    ] {
        assert_eq!(to_serial(date), expected, "{date:?}");
    }
}

#[test]
fn the_weekday_is_what_a_calendar_says() {
    // 2024-03-05 was a Tuesday, which is 2 when Sunday is 0.
    assert_eq!(try_to_datetime(45356.0).unwrap().weekday, 2);

    // 2024-01-01 was a Monday.
    assert_eq!(try_to_datetime(45292.0).unwrap().weekday, 1);

    // 1900-01-01 was a Monday, and .NET agrees despite the phantom leap day that follows.
    assert_eq!(try_to_datetime(1.0).unwrap().weekday, 1);
}

#[test]
fn the_fraction_holds_the_time_of_day() {
    let midday = CivilDateTime::date(2024, 1, 1).at(12, 0, 0, 0);

    assert_eq!(try_to_datetime(45292.5), Some(midday));
    assert_eq!(to_serial(midday), 45292.5);
}

#[test]
fn every_day_from_1900_to_2100_survives_a_round_trip() {
    let mut days = 0;
    let mut first_mismatch = None;

    // Serial 61 is 1900-03-01; 2100-01-01 is serial 73050. Below 61 the phantom leap day makes the
    // mapping ambiguous, which the next test covers.
    for step in 61..73_050 {
        days += 1;
        let serial = step as f64;

        let Some(date) = try_to_datetime(serial) else {
            first_mismatch.get_or_insert(serial);
            continue;
        };

        if to_serial(date) != serial {
            first_mismatch.get_or_insert(serial);
        }
    }

    assert!(days > 70_000, "only {days} days were checked");
    assert_eq!(
        first_mismatch, None,
        "the first mismatch was at {first_mismatch:?}"
    );
}

#[test]
fn the_phantom_leap_day_lands_on_the_first_of_march() {
    // Excel counts 1900-02-29 as serial 60. No such day exists in the Gregorian calendar, so it reads
    // as 1900-03-01 — the same date as serial 61. That one serial is the only inexact mapping.
    assert_eq!(try_to_datetime(60.0), Some(CivilDateTime::date(1900, 3, 1)));

    // The day before it is unaffected, which is what matters for real dates.
    assert_eq!(try_to_datetime(59.0), Some(CivilDateTime::date(1900, 2, 28)));
}

#[test]
fn a_serial_that_is_not_a_date_is_refused() {
    for serial in [-1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 4_000_000.0] {
        assert!(
            try_to_datetime(serial).is_none(),
            "serial {serial} should have been refused"
        );
    }
}

#[test]
fn millisecond_precision_is_the_resolution() {
    // .NET's DateTime.AddDays rounds to the millisecond, which is what this codec reproduces.
    let date = try_to_datetime(45356.25).unwrap();

    assert_eq!(
        (date.hour, date.minute, date.second, date.millisecond),
        (6, 0, 0, 0)
    );

    let half_second = try_to_datetime(45356.0 + 0.5 / 86_400.0).unwrap();
    assert_eq!(half_second.millisecond, 500);
}
