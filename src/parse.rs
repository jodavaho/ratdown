// Port of termdown's parse_timedelta / parse_timestr. Instead of dateutil,
// absolute times accept the documented forms: 'HH:MM[:SS]', 'YYYY-MM-DD',
// 'YYYY-MM-DD HH:MM[:SS]', each optionally suffixed with ' UTC'.
use chrono::{Duration, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Timelike, Utc};

// Ordered components as in termdown's TIMEDELTA_REGEX: \d+y \d+d \d+h \d+m \d+s,
// each optional, separated by at most one space. Trailing garbage is ignored
// (the regex is a prefix match); no components matched yields 0.
pub fn parse_timedelta(deltastr: &str) -> i64 {
    let mut rest = deltastr;
    let mut total = 0i64;
    for (unit, secs) in [
        ('y', 31557600i64),
        ('d', 86400),
        ('h', 3600),
        ('m', 60),
        ('s', 1),
    ] {
        let digits_len = rest.chars().take_while(|c| c.is_ascii_digit()).count();
        if digits_len == 0 {
            continue;
        }
        let after = &rest[digits_len..];
        if after.starts_with(unit) {
            total += rest[..digits_len].parse::<i64>().unwrap_or(0) * secs;
            rest = &after[1..];
            if let Some(stripped) = rest.strip_prefix(' ') {
                rest = stripped;
            }
        }
    }
    total
}

fn parse_datetime(s: &str) -> Option<NaiveDateTime> {
    let s = s.trim();
    let (s, utc) = match s
        .strip_suffix(" UTC")
        .or_else(|| s.strip_suffix(" utc"))
        .or_else(|| s.strip_suffix("Z"))
    {
        Some(prefix) => (prefix.trim_end(), true),
        None => (s, false),
    };
    let today = Local::now().date_naive();
    let naive = NaiveTime::parse_from_str(s, "%H:%M:%S")
        .map(|t| today.and_time(t))
        .ok()
        .or_else(|| {
            NaiveTime::parse_from_str(s, "%H:%M")
                .map(|t| today.and_time(t))
                .ok()
        })
        .or_else(|| NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S").ok())
        .or_else(|| NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M").ok())
        .or_else(|| NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S").ok())
        .or_else(|| {
            NaiveDate::parse_from_str(s, "%Y-%m-%d")
                .map(|d| d.and_hms_opt(0, 0, 0).unwrap())
                .ok()
        })?;
    if utc {
        Some(
            Utc.from_utc_datetime(&naive)
                .with_timezone(&Local)
                .naive_local(),
        )
    } else {
        Some(naive)
    }
}

pub fn parse_timestr(timestr: &str) -> Result<(NaiveDateTime, NaiveDateTime), String> {
    let delta = parse_timedelta(timestr);
    let mut sync_start = Local::now().naive_local();
    let target = if delta != 0 {
        sync_start + Duration::seconds(delta)
    } else if !timestr.is_empty() && timestr.chars().all(|c| c.is_ascii_digit()) {
        let secs: i64 = timestr
            .parse()
            .map_err(|_| format!("Unable to parse '{}'", timestr))?;
        sync_start + Duration::seconds(secs)
    } else {
        // Countdowns to absolute times draw frames at full seconds.
        sync_start = sync_start.with_nanosecond(0).unwrap();
        parse_datetime(timestr).ok_or_else(|| format!("Unable to parse '{}'", timestr))?
    };
    Ok((sync_start, target))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timedelta_units() {
        assert_eq!(parse_timedelta("10"), 0);
        assert_eq!(parse_timedelta("10s"), 10);
        assert_eq!(parse_timedelta("1h 5m 30s"), 3930);
        assert_eq!(parse_timedelta("1h5m30s"), 3930);
        assert_eq!(parse_timedelta("2d"), 172800);
        assert_eq!(parse_timedelta("1y"), 31557600);
        assert_eq!(parse_timedelta("12:00"), 0);
        assert_eq!(parse_timedelta("2020-01-01"), 0);
    }

    #[test]
    fn timestr_pure_seconds() {
        let (start, target) = parse_timestr("90").unwrap();
        let diff = (target - start).num_seconds();
        assert_eq!(diff, 90);
    }

    #[test]
    fn timestr_duration() {
        let (start, target) = parse_timestr("1m 30s").unwrap();
        assert_eq!((target - start).num_seconds(), 90);
    }

    #[test]
    fn timestr_absolute_date() {
        let (start, target) = parse_timestr("2099-01-02").unwrap();
        assert_eq!(start.nanosecond(), 0);
        assert_eq!(
            target,
            NaiveDate::from_ymd_opt(2099, 1, 2)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap()
        );
    }

    #[test]
    fn timestr_clock_time_is_today() {
        let (_, target) = parse_timestr("12:34").unwrap();
        assert_eq!(target.date(), Local::now().date_naive());
        assert_eq!(target.time(), NaiveTime::from_hms_opt(12, 34, 0).unwrap());
    }

    #[test]
    fn timestr_datetime_with_utc() {
        let (_, target) = parse_timestr("2099-01-02 14:00 UTC").unwrap();
        let expected = Utc
            .from_utc_datetime(
                &NaiveDate::from_ymd_opt(2099, 1, 2)
                    .unwrap()
                    .and_hms_opt(14, 0, 0)
                    .unwrap(),
            )
            .with_timezone(&Local)
            .naive_local();
        assert_eq!(target, expected);
    }

    #[test]
    fn timestr_garbage_errors() {
        assert_eq!(
            parse_timestr("nonsense").unwrap_err(),
            "Unable to parse 'nonsense'"
        );
    }
}
