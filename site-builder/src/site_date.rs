//! The calendar day a build describes. Updates are split into Upcoming,
//! Ongoing, and earlier plans by this day (see `Update::phase_on`), so it is
//! the build's only input that does not come from the repository.
//!
//! Event dates are days in Japan, so the day is taken in Asia/Tokyo, never in
//! the machine's local time zone or in UTC. Japan has observed UTC+09:00
//! without daylight saving time since 1952, so a fixed offset gives the same
//! day as the time zone database for every date the site can show.

use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, ensure};
use chrono::{DateTime, FixedOffset, NaiveDate};

/// Overrides the clock with a day, `YYYY-MM-DD`, already in Asia/Tokyo. Tests
/// and the verification script set it so that repeated builds describe the
/// same day.
pub const OVERRIDE_VARIABLE: &str = "SITE_BUILD_DATE";

const TOKYO_OFFSET_SECONDS: i32 = 9 * 60 * 60;

/// The Asia/Tokyo calendar day at `unix_seconds`.
pub fn tokyo_date(unix_seconds: i64) -> Option<NaiveDate> {
    let offset = FixedOffset::east_opt(TOKYO_OFFSET_SECONDS)?;
    Some(
        DateTime::from_timestamp(unix_seconds, 0)?
            .with_timezone(&offset)
            .date_naive(),
    )
}

/// The build's day: the override when one is given, otherwise the Asia/Tokyo
/// day at `now`.
pub fn resolve(override_value: Option<&str>, now: SystemTime) -> Result<NaiveDate> {
    if let Some(value) = override_value {
        ensure!(
            value.len() == 10,
            "{OVERRIDE_VARIABLE} must be a date, YYYY-MM-DD; got {value:?}"
        );
        return NaiveDate::parse_from_str(value, "%Y-%m-%d").with_context(|| {
            format!("{OVERRIDE_VARIABLE} must be a date, YYYY-MM-DD; got {value:?}")
        });
    }
    let seconds = now
        .duration_since(UNIX_EPOCH)
        .context("the system clock is before 1970")?
        .as_secs();
    i64::try_from(seconds)
        .ok()
        .and_then(tokyo_date)
        .context("the system clock is out of range")
}

/// The build's day from the environment and the system clock.
pub fn current() -> Result<NaiveDate> {
    let value = std::env::var(OVERRIDE_VARIABLE).ok();
    resolve(value.as_deref(), SystemTime::now())
}

#[cfg(test)]
mod tests {
    use super::{resolve, tokyo_date};
    use chrono::{NaiveDate, NaiveDateTime};
    use std::time::{Duration, UNIX_EPOCH};

    fn utc_seconds(text: &str) -> i64 {
        NaiveDateTime::parse_from_str(text, "%Y-%m-%d %H:%M:%S")
            .unwrap()
            .and_utc()
            .timestamp()
    }

    fn day(month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, month, day).unwrap()
    }

    #[test]
    fn the_tokyo_day_begins_at_15_00_utc() {
        // 23:59:59 JST on 15 October is still 15 October in UTC.
        assert_eq!(
            tokyo_date(utc_seconds("2026-10-15 14:59:59")),
            Some(day(10, 15))
        );
        // From 15:00 UTC the UTC date still says 15 October, but in Tokyo it
        // is already the first day of the event.
        assert_eq!(
            tokyo_date(utc_seconds("2026-10-15 15:00:00")),
            Some(day(10, 16))
        );
        assert_eq!(
            tokyo_date(utc_seconds("2026-10-15 23:30:00")),
            Some(day(10, 16))
        );
        // 08:59 JST on 19 October is 18 October in UTC.
        assert_eq!(
            tokyo_date(utc_seconds("2026-10-18 23:59:00")),
            Some(day(10, 19))
        );
        // Year boundary.
        assert_eq!(
            tokyo_date(utc_seconds("2026-12-31 15:00:00")),
            NaiveDate::from_ymd_opt(2027, 1, 1)
        );
    }

    #[test]
    fn the_override_wins_over_the_clock_and_must_be_a_real_date() {
        let now = UNIX_EPOCH + Duration::from_secs(utc_seconds("2026-10-15 15:00:00") as u64);
        assert_eq!(resolve(None, now).unwrap(), day(10, 16));
        assert_eq!(resolve(Some("2026-10-19"), now).unwrap(), day(10, 19));
        for invalid in [
            "",
            "2026-10-32",
            "2026-02-29",
            "2026-1-19",
            "19 Oct 2026",
            "2026-10-19\n",
        ] {
            assert!(resolve(Some(invalid), now).is_err(), "{invalid:?}");
        }
    }
}
