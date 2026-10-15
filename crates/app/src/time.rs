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

#[cfg(not(unix))]
pub fn local(unix: i64) -> Local {
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
    fn days_start_at_midnight() {
        let t = now();
        let s = day_start(t);
        assert!(s <= t && t - s < 86_400);
        let l = local(s);
        assert_eq!((l.hour, l.minute), (0, 0));
    }
}
