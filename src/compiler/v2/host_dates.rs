//! Versioned millisecond-resolution Gregorian/UTC/fixed-offset date contract.
use super::host_v3::HostError;
const DAY: i64 = 86_400_000;
fn invalid() -> HostError {
    HostError {
        code: "date",
        message: "invalid or unsupported Gregorian date/offset".into(),
    }
}
fn leap(y: i64) -> bool {
    y % 4 == 0 && (y % 100 != 0 || y % 400 == 0)
}
fn days(y: i64, m: i64, d: i64) -> Result<i64, HostError> {
    let max = match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if leap(y) {
                29
            } else {
                28
            }
        }
        _ => return Err(invalid()),
    };
    if !(1..=9999).contains(&y) || d < 1 || d > max {
        return Err(invalid());
    }
    let y = y - i64::from(m <= 2);
    let era = y.div_euclid(400);
    let yo = y - era * 400;
    let mp = m + if m > 2 { -3 } else { 9 };
    Ok(era * 146097 + yo * 365 + yo / 4 - yo / 100 + (153 * mp + 2) / 5 + d - 1 - 719468)
}
fn civil(day: i64) -> (i64, i64, i64) {
    let z = day + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yo = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let mut y = yo + era * 400;
    let doy = doe - (365 * yo + yo / 4 - yo / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    y += i64::from(m <= 2);
    (y, m, d)
}
fn valid_epoch(ms: i64) -> Result<(), HostError> {
    if ms < days(1, 1, 1)? * DAY || ms > days(9999, 12, 31)? * DAY + DAY - 1 {
        return Err(invalid());
    }
    Ok(())
}
fn offset(minutes: i64) -> Result<(), HostError> {
    if !(-1439..=1439).contains(&minutes) {
        Err(invalid())
    } else {
        Ok(())
    }
}
fn number(bytes: &[u8]) -> Result<i64, HostError> {
    if bytes.is_empty() || !bytes.iter().all(u8::is_ascii_digit) {
        return Err(invalid());
    }
    bytes.iter().try_fold(0i64, |n, b| {
        n.checked_mul(10)
            .and_then(|n| n.checked_add(i64::from(*b - b'0')))
            .ok_or_else(invalid)
    })
}
/// Parse ISO 8601 with seconds, optional 1..3 fractional digits and explicit Z/fixed offset.
/// Millisecond precision is exact; leap seconds and named/local zones are unsupported.
pub fn parse_date(text: &str) -> Result<i64, HostError> {
    let b = text.as_bytes();
    if b.len() < 20
        || b.len() > 29
        || b[4] != b'-'
        || b[7] != b'-'
        || b[10] != b'T'
        || b[13] != b':'
        || b[16] != b':'
    {
        return Err(invalid());
    }
    let (y, m, d) = (number(&b[..4])?, number(&b[5..7])?, number(&b[8..10])?);
    let (h, min, s) = (
        number(&b[11..13])?,
        number(&b[14..16])?,
        number(&b[17..19])?,
    );
    if h > 23 || min > 59 || s > 59 {
        return Err(invalid());
    }
    let mut at = 19;
    let mut millis = 0;
    if b.get(at) == Some(&b'.') {
        at += 1;
        let start = at;
        while b.get(at).is_some_and(u8::is_ascii_digit) {
            at += 1;
        }
        let n = at - start;
        if !(1..=3).contains(&n) {
            return Err(invalid());
        }
        millis = number(&b[start..at])? * 10i64.pow((3 - n) as u32);
    }
    let zone = match b.get(at) {
        Some(b'Z') if at + 1 == b.len() => 0,
        Some(b'+' | b'-') if at + 6 == b.len() && b[at + 3] == b':' => {
            let hh = number(&b[at + 1..at + 3])?;
            let mm = number(&b[at + 4..at + 6])?;
            if hh > 23 || mm > 59 {
                return Err(invalid());
            }
            (hh * 60 + mm) * if b[at] == b'-' { -1 } else { 1 }
        }
        _ => return Err(invalid()),
    };
    let ms =
        days(y, m, d)? * DAY + h * 3_600_000 + min * 60_000 + s * 1000 + millis - zone * 60_000;
    valid_epoch(ms)?;
    Ok(ms)
}
/// Format an in-range UTC millisecond instant using an explicit fixed minute offset.
pub fn format_date(ms: i64, minutes: i64) -> Result<String, HostError> {
    valid_epoch(ms)?;
    offset(minutes)?;
    let local = ms.checked_add(minutes * 60_000).ok_or_else(invalid)?;
    valid_epoch(local)?;
    let (y, m, d) = civil(local.div_euclid(DAY));
    let rest = local.rem_euclid(DAY);
    let base = format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}.{:03}",
        rest / 3_600_000,
        (rest / 60_000) % 60,
        (rest / 1000) % 60,
        rest % 1000
    );
    if minutes == 0 {
        Ok(base + "Z")
    } else {
        Ok(format!(
            "{base}{}{:02}:{:02}",
            if minutes < 0 { '-' } else { '+' },
            minutes.abs() / 60,
            minutes.abs() % 60
        ))
    }
}
/// Checked instant arithmetic within the supported Gregorian year range.
pub fn add_date(ms: i64, delta: i64) -> Result<i64, HostError> {
    valid_epoch(ms)?;
    let out = ms.checked_add(delta).ok_or_else(invalid)?;
    valid_epoch(out)?;
    Ok(out)
}
