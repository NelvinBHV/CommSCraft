//! Times and durations for the interface (no time zone database: the application passes the
//! local UTC offset it read at start).

/// A civil date from days since 1970-01-01 (proleptic Gregorian; H. Hinnant's algorithm).
pub fn civil(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (
        y,
        u32::try_from(m).unwrap_or(1),
        u32::try_from(d).unwrap_or(1),
    )
}

fn local(at: u64, offset: i64) -> i64 {
    i64::try_from(at).unwrap_or(i64::MAX).saturating_add(offset)
}

/// The local day number of a time.
pub fn day(at: u64, offset: i64) -> i64 {
    local(at, offset).div_euclid(86_400)
}

/// "09:42".
pub fn clock(at: u64, offset: i64) -> String {
    let s = local(at, offset).rem_euclid(86_400);
    format!("{:02}:{:02}", s / 3600, (s / 60) % 60)
}

/// "16.11.2026".
pub fn date(at: u64, offset: i64) -> String {
    let (y, m, d) = civil(day(at, offset));
    format!("{d:02}.{m:02}.{y}")
}

/// "512 B", "12 KB", "3.4 MB".
pub fn bytes(n: i64) -> String {
    let n = n.max(0) as f64;
    if n < 1024.0 {
        format!("{n} B")
    } else if n < 1024.0 * 1024.0 {
        format!("{:.0} KB", n / 1024.0)
    } else {
        format!("{:.1} MB", n / (1024.0 * 1024.0))
    }
}

/// "03:48" or "1:02:05".
pub fn duration(seconds: u64) -> String {
    let (h, m, s) = (seconds / 3600, (seconds / 60) % 60, seconds % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m:02}:{s:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_and_clock() {
        // 2026-11-16 08:42:00 UTC
        let t = 1_779_000_000 - 1_779_000_000 % 86_400 + 8 * 3600 + 42 * 60;
        assert_eq!(clock(t, 0), "08:42");
        assert_eq!(clock(t, 3600), "09:42");
        assert_eq!(civil(0), (1970, 1, 1));
        assert_eq!(civil(20_773), (2026, 11, 16));
        assert_eq!(date(20_773 * 86_400, 0), "16.11.2026");
        // Crossing midnight with the offset moves the day.
        assert_eq!(day(20_773 * 86_400 - 1800, 3600), 20_773);
    }

    #[test]
    fn durations() {
        assert_eq!(duration(228), "03:48");
        assert_eq!(duration(3725), "1:02:05");
        assert_eq!(duration(0), "00:00");
    }
}
