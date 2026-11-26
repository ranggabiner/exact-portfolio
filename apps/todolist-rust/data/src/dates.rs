//! Civil-calendar arithmetic. The data module reads no clock: every instant
//! arrives as epoch milliseconds plus the zone's offset in minutes east of UTC.

pub const DAY_MS: f64 = 86_400_000.0;

/// Days since 1970-01-01 of a proleptic Gregorian date (H. Hinnant's algorithm).
pub fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let mp = i64::from((m + 9) % 12);
    let doy = (153 * mp + 2) / 5 + i64::from(d) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

pub fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

/// 0 = Monday … 6 = Sunday.
pub fn weekday(days: i64) -> u32 {
    (days + 3).rem_euclid(7) as u32
}

pub fn month_length(y: i64, m: u32) -> u32 {
    let next = if m == 12 {
        days_from_civil(y + 1, 1, 1)
    } else {
        days_from_civil(y, m + 1, 1)
    };
    (next - days_from_civil(y, m, 1)) as u32
}

pub fn key(days: i64) -> String {
    let (y, m, d) = civil_from_days(days);
    format!("{y:04}-{m:02}-{d:02}")
}

pub fn parse_key(key: &str) -> Option<i64> {
    let mut parts = key.split('-');
    let y: i64 = parts.next()?.parse().ok()?;
    let m: u32 = parts.next()?.parse().ok()?;
    let d: u32 = parts.next()?.parse().ok()?;
    if parts.next().is_some() || !(1..=12).contains(&m) || d == 0 || d > month_length(y, m) {
        return None;
    }
    Some(days_from_civil(y, m, d))
}

/// `"HH:MM"` as minutes after midnight.
pub fn parse_time(time: &str) -> Option<u32> {
    let (h, m) = time.split_once(':')?;
    let h: u32 = h.parse().ok()?;
    let m: u32 = m.parse().ok()?;
    (h < 24 && m < 60).then_some(h * 60 + m)
}

pub fn time_key(minutes: u32) -> String {
    format!("{:02}:{:02}", (minutes / 60) % 24, minutes % 60)
}

/// The person's own day and minute of an instant.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Local {
    pub day: i64,
    pub minute: u32,
}

pub fn local(now: f64, offset: f64) -> Local {
    let ms = now + offset * 60_000.0;
    let day = (ms / DAY_MS).floor();
    let minute = ((ms - day * DAY_MS) / 60_000.0).floor().clamp(0.0, 1439.0);
    Local {
        day: day as i64,
        minute: minute as u32,
    }
}

/// The instant of a local day and minute.
pub fn instant(day: i64, minute: u32, offset: f64) -> f64 {
    day as f64 * DAY_MS + f64::from(minute) * 60_000.0 - offset * 60_000.0
}

/// The next month's same day, clamped to the month's end.
pub fn add_month(days: i64) -> i64 {
    let (y, m, d) = civil_from_days(days);
    let (y, m) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
    days_from_civil(y, m, d.min(month_length(y, m)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_names_weekdays() {
        let day = parse_key("2026-11-27").unwrap();
        assert_eq!(key(day), "2026-11-27");
        assert_eq!(weekday(day), 4, "a Friday");
        assert_eq!(weekday(0), 3, "1970-01-01 was a Thursday");
    }

    #[test]
    fn refuses_impossible_dates_and_times() {
        assert!(parse_key("2026-02-30").is_none());
        assert!(parse_key("2026-13-01").is_none());
        assert!(parse_key("tomorrow").is_none());
        assert!(parse_time("24:00").is_none());
        assert_eq!(parse_time("07:05"), Some(425));
    }

    #[test]
    fn months_clamp_to_their_end() {
        let jan31 = parse_key("2027-01-31").unwrap();
        assert_eq!(key(add_month(jan31)), "2027-02-28");
    }

    #[test]
    fn local_days_follow_the_offset() {
        let midnight_utc = parse_key("2026-11-27").unwrap() as f64 * DAY_MS;
        let jakarta = local(midnight_utc, 420.0);
        assert_eq!(
            (key(jakarta.day).as_str(), jakarta.minute),
            ("2026-11-27", 420)
        );
        let new_york = local(midnight_utc, -300.0);
        assert_eq!(key(new_york.day), "2026-11-26");
        assert_eq!(instant(jakarta.day, jakarta.minute, 420.0), midnight_utc);
    }
}
