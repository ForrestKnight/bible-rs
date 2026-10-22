//! Local dates and times for history and lists: "2 h ago", "Wed", "9:41".

use bible_core::study::now;

/// A moment in the local time zone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Local {
    pub year: i32,
    /// 1–12.
    pub month: u8,
    pub day: u8,
    /// 0 = Sunday.
    pub weekday: u8,
    pub hour: u8,
    pub minute: u8,
    /// Seconds east of UTC.
    pub offset: i64,
}

#[cfg(unix)]
pub fn local(unix: i64) -> Local {
    // SAFETY: `localtime_r` writes into the `tm` we own; `time_t` is i64 on supported platforms.
    let tm = unsafe {
        let mut tm: libc::tm = std::mem::zeroed();
        let t = unix as libc::time_t;
        libc::localtime_r(&t, &mut tm);
        tm
    };
    Local {
        year: tm.tm_year + 1900,
        month: (tm.tm_mon + 1) as u8,
        day: tm.tm_mday as u8,
        weekday: tm.tm_wday as u8,
        hour: tm.tm_hour as u8,
        minute: tm.tm_min as u8,
        offset: tm.tm_gmtoff,
    }
}

#[cfg(windows)]
pub fn local(unix: i64) -> Local {
    use windows_sys::Win32::Foundation::{FILETIME, SYSTEMTIME};
    use windows_sys::Win32::System::Time::{
        FileTimeToSystemTime, SystemTimeToFileTime, SystemTimeToTzSpecificLocalTime,
    };
    // FILETIME counts 100 ns ticks from 1601-01-01 (UTC).
    const TICKS_1601_TO_1970: i64 = 116_444_736_000_000_000;
    let ticks = |ft: &FILETIME| (i64::from(ft.dwHighDateTime) << 32) | i64::from(ft.dwLowDateTime);
    let t = (unix.max(0) * 10_000_000 + TICKS_1601_TO_1970) as u64;
    let utc_ft = FILETIME { dwLowDateTime: t as u32, dwHighDateTime: (t >> 32) as u32 };
    let zero = SYSTEMTIME {
        wYear: 0,
        wMonth: 0,
        wDayOfWeek: 0,
        wDay: 0,
        wHour: 0,
        wMinute: 0,
        wSecond: 0,
        wMilliseconds: 0,
    };
    let (mut utc, mut tm) = (zero, zero);
    let mut local_ft = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
    // SAFETY: plain conversions between structs we own; a null time zone means the current one
    // (with its daylight saving rules for that date).
    let ok = unsafe {
        FileTimeToSystemTime(&utc_ft, &mut utc) != 0
            && SystemTimeToTzSpecificLocalTime(std::ptr::null(), &utc, &mut tm) != 0
            && SystemTimeToFileTime(&tm, &mut local_ft) != 0
    };
    if !ok {
        return utc_local(unix);
    }
    Local {
        year: i32::from(tm.wYear),
        month: tm.wMonth as u8,
        day: tm.wDay as u8,
        weekday: tm.wDayOfWeek as u8,
        hour: tm.wHour as u8,
        minute: tm.wMinute as u8,
        offset: (ticks(&local_ft) - ticks(&utc_ft)) / 10_000_000,
    }
}

#[cfg(not(any(unix, windows)))]
pub fn local(unix: i64) -> Local {
    utc_local(unix)
}

/// UTC, for platforms without a time zone database.
#[cfg(not(unix))]
fn utc_local(unix: i64) -> Local {
    let days = unix.div_euclid(86_400);
    let secs = unix.rem_euclid(86_400);
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u8;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u8;
    let year = (yoe + era * 400 + i64::from(month <= 2)) as i32;
    Local {
        year,
        month,
        day,
        weekday: (days + 4).rem_euclid(7) as u8,
        hour: (secs / 3600) as u8,
        minute: (secs / 60 % 60) as u8,
        offset: 0,
    }
}

/// Seconds from 1970-01-01 to the start of a civil date (UTC), for the tests.
#[cfg(test)]
fn civil_seconds(year: i32, month: u8, day: u8) -> i64 {
    // Days from civil (Howard Hinnant's algorithm).
    let y = i64::from(year) - i64::from(month <= 2);
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let m = i64::from(month);
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + i64::from(day) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    (era * 146_097 + doe - 719_468) * 86_400
}

const WEEKDAYS: [&str; 7] = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
const MONTHS: [&str; 12] =
    ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

pub fn weekday(w: u8) -> &'static str {
    WEEKDAYS[w as usize % 7]
}

pub fn weekday_short(w: u8) -> &'static str {
    &weekday(w)[..3]
}

/// Local midnight at the start of the day containing `unix`.
pub fn day_start(unix: i64) -> i64 {
    let l = local(unix);
    unix - i64::from(l.hour) * 3600 - i64::from(l.minute) * 60 - (unix + l.offset).rem_euclid(60)
}

/// "9:41" (24-hour clock, as the design shows).
pub fn clock(unix: i64) -> String {
    let l = local(unix);
    format!("{}:{:02}", l.hour, l.minute)
}

/// "Oct 3".
pub fn short_date(unix: i64) -> String {
    let l = local(unix);
    format!("{} {}", MONTHS[(l.month as usize + 11) % 12], l.day)
}

/// How long ago, briefly: "Just now", "12 min ago", "2 h ago", "Wed", "Oct 3".
pub fn ago(unix: i64) -> String {
    let now = now();
    let secs = (now - unix).max(0);
    if secs < 60 {
        "Just now".into()
    } else if secs < 3600 {
        format!("{} min ago", secs / 60)
    } else if unix >= day_start(now) {
        format!("{} h ago", secs / 3600)
    } else if unix >= day_start(now) - 6 * 86_400 {
        weekday_short(local(unix).weekday).into()
    } else {
        short_date(unix)
    }
}

/// "12 min", "1 h 56 m".
pub fn duration(seconds: u32) -> String {
    let m = (seconds + 30) / 60;
    if m < 60 { format!("{m} min") } else { format!("{} h {} m", m / 60, m % 60) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations() {
        assert_eq!(duration(700), "12 min");
        assert_eq!(duration(116 * 60), "1 h 56 m");
    }

    #[test]
    fn local_time_is_utc_plus_its_offset() {
        let t = now();
        let l = local(t);
        // The same wall-clock minute, read as UTC, is `offset` seconds after `t`.
        let wall = super::civil_seconds(l.year, l.month, l.day)
            + i64::from(l.hour) * 3600
            + i64::from(l.minute) * 60;
        assert_eq!(wall - (t - t.rem_euclid(60)), l.offset);
        assert_eq!(l.offset % 60, 0, "offsets are whole minutes");
        // 1970-01-01 was a Thursday; weekdays count from Sunday.
        assert_eq!(
            i64::from(l.weekday),
            (super::civil_seconds(l.year, l.month, l.day) / 86_400 + 4).rem_euclid(7)
        );
    }

    #[test]
    fn days_start_at_midnight() {
        let t = now();
        let s = day_start(t);
        assert!(s <= t && t - s < 86_400);
        let l = local(s);
        assert_eq!((l.hour, l.minute), (0, 0));
    }
}
