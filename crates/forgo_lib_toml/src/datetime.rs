//! [`Datetime`]: TOML's offset date-times, local date-times, local dates
//! and local times (RFC 3339).

use std::fmt;

/// A TOML date and/or time.
///
/// | Kind | `date` | `time` | `offset` | Example |
/// |---|---|---|---|---|
/// | offset date-time | yes | yes | yes | `1979-05-27T07:32:00Z` |
/// | local date-time | yes | yes | no | `1979-05-27T07:32:00` |
/// | local date | yes | no | no | `1979-05-27` |
/// | local time | no | yes | no | `07:32:00` |
///
/// ```
/// use forgo_lib_toml::{Datetime, Document, Offset};
///
/// let doc = Document::parse("t = 1979-05-27T07:32:00.5-07:00")?;
/// let t = doc.root().get("t").unwrap().as_datetime().unwrap();
/// assert_eq!(t.date.unwrap().year, 1979);
/// assert_eq!(t.time.unwrap().nanosecond, 500_000_000);
/// assert_eq!(t.offset, Some(Offset::Minutes(-420)));
/// assert_eq!(t.to_string(), "1979-05-27T07:32:00.5-07:00");
/// # Ok::<(), forgo_lib_toml::Error>(())
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Datetime {
    /// The date, if any.
    pub date: Option<Date>,
    /// The time, if any.
    pub time: Option<Time>,
    /// The offset from UTC (only with a date and a time).
    pub offset: Option<Offset>,
}

/// A calendar date.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Date {
    /// 0 to 9999.
    pub year: u16,
    /// 1 to 12.
    pub month: u8,
    /// 1 to 31 (as the month allows).
    pub day: u8,
}

/// A time of day.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Time {
    /// 0 to 23.
    pub hour: u8,
    /// 0 to 59.
    pub minute: u8,
    /// 0 to 60 (60: a leap second).
    pub second: u8,
    /// 0 to 999,999,999. Digits past nanoseconds are dropped.
    pub nanosecond: u32,
}

/// An offset from UTC.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Offset {
    /// `Z`: UTC.
    Z,
    /// `±HH:MM`, in minutes east of UTC.
    Minutes(i16),
}

impl Datetime {
    /// The kind, as toml-test names it: `datetime`, `datetime-local`,
    /// `date-local` or `time-local`.
    pub fn kind_name(&self) -> &'static str {
        match (
            self.date.is_some(),
            self.time.is_some(),
            self.offset.is_some(),
        ) {
            (true, true, true) => "datetime",
            (true, true, false) => "datetime-local",
            (true, false, _) => "date-local",
            _ => "time-local",
        }
    }
}

pub(crate) fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            let leap =
                (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400);
            if leap { 29 } else { 28 }
        }
        _ => 0,
    }
}

fn digits(b: &[u8], i: usize, n: usize) -> Option<u32> {
    let s = b.get(i..i + n)?;
    let mut v = 0u32;
    for &c in s {
        if !c.is_ascii_digit() {
            return None;
        }
        v = v * 10 + u32::from(c - b'0');
    }
    Some(v)
}

/// Why a date-time didn't parse.
pub(crate) enum DtError {
    /// Not a date-time at all (the caller may try something else).
    Syntax(usize),
    /// A date-time with an impossible value.
    Range(usize, &'static str),
}

/// A date at `i` (`YYYY-MM-DD`), and the index after it.
fn date(b: &[u8], i: usize) -> Result<(Date, usize), DtError> {
    let year = digits(b, i, 4).ok_or(DtError::Syntax(i))?;
    if b.get(i + 4) != Some(&b'-') {
        return Err(DtError::Syntax(i + 4));
    }
    let month = digits(b, i + 5, 2).ok_or(DtError::Syntax(i + 5))?;
    if b.get(i + 7) != Some(&b'-') {
        return Err(DtError::Syntax(i + 7));
    }
    let day = digits(b, i + 8, 2).ok_or(DtError::Syntax(i + 8))?;
    if b.get(i + 10).is_some_and(u8::is_ascii_digit) {
        return Err(DtError::Syntax(i + 10));
    }
    if !(1..=12).contains(&month) {
        return Err(DtError::Range(i + 5, "the month must be 01 to 12"));
    }
    let (year, month) = (year as u16, month as u8);
    if day == 0 || day > u32::from(days_in_month(year, month)) {
        return Err(DtError::Range(i + 8, "no such day in that month"));
    }
    Ok((
        Date {
            year,
            month,
            day: day as u8,
        },
        i + 10,
    ))
}

/// A time at `i` (`HH:MM[:SS[.fraction]]`), and the index after it.
/// Seconds are required unless `optional_seconds` (TOML 1.1).
fn time(b: &[u8], i: usize, optional_seconds: bool) -> Result<(Time, usize), DtError> {
    let hour = digits(b, i, 2).ok_or(DtError::Syntax(i))?;
    if b.get(i + 2) != Some(&b':') {
        return Err(DtError::Syntax(i + 2));
    }
    let minute = digits(b, i + 3, 2).ok_or(DtError::Syntax(i + 3))?;
    let mut j = i + 5;
    let mut second = 0;
    let mut nanosecond = 0;
    if b.get(j) == Some(&b':') {
        second = digits(b, j + 1, 2).ok_or(DtError::Syntax(j + 1))?;
        j += 3;
        if b.get(j) == Some(&b'.') {
            let start = j + 1;
            let mut k = start;
            while b.get(k).is_some_and(u8::is_ascii_digit) {
                if k - start < 9 {
                    nanosecond = nanosecond * 10 + u32::from(b[k] - b'0');
                }
                k += 1;
            }
            if k == start {
                return Err(DtError::Syntax(k));
            }
            for _ in (k - start)..9 {
                nanosecond *= 10;
            }
            j = k;
        }
    } else if !optional_seconds {
        return Err(DtError::Syntax(j));
    }
    if b.get(j).is_some_and(u8::is_ascii_digit) {
        return Err(DtError::Syntax(j));
    }
    if hour > 23 {
        return Err(DtError::Range(i, "the hour must be 00 to 23"));
    }
    if minute > 59 {
        return Err(DtError::Range(i + 3, "the minute must be 00 to 59"));
    }
    if second > 60 {
        return Err(DtError::Range(i + 6, "the second must be 00 to 60"));
    }
    Ok((
        Time {
            hour: hour as u8,
            minute: minute as u8,
            second: second as u8,
            nanosecond,
        },
        j,
    ))
}

fn offset(b: &[u8], i: usize) -> Result<(Option<Offset>, usize), DtError> {
    match b.get(i) {
        Some(b'Z' | b'z') => Ok((Some(Offset::Z), i + 1)),
        Some(&sign @ (b'+' | b'-')) => {
            let h = digits(b, i + 1, 2).ok_or(DtError::Syntax(i + 1))?;
            if b.get(i + 3) != Some(&b':') {
                return Err(DtError::Syntax(i + 3));
            }
            let m = digits(b, i + 4, 2).ok_or(DtError::Syntax(i + 4))?;
            if h > 23 {
                return Err(DtError::Range(i + 1, "the offset's hour must be 00 to 23"));
            }
            if m > 59 {
                return Err(DtError::Range(
                    i + 4,
                    "the offset's minute must be 00 to 59",
                ));
            }
            let minutes = (h * 60 + m) as i16;
            Ok((
                Some(Offset::Minutes(if sign == b'-' {
                    -minutes
                } else {
                    minutes
                })),
                i + 6,
            ))
        }
        _ => Ok((None, i)),
    }
}

/// A date-time at `i`: the value and the index after it.
pub(crate) fn parse(
    b: &[u8],
    i: usize,
    optional_seconds: bool,
) -> Result<(Datetime, usize), DtError> {
    // A local time: `HH:MM...`.
    if b.get(i + 2) == Some(&b':') {
        let (t, j) = time(b, i, optional_seconds)?;
        return Ok((
            Datetime {
                date: None,
                time: Some(t),
                offset: None,
            },
            j,
        ));
    }
    let (d, j) = date(b, i)?;
    // A time follows `T`, `t`, or a space and a digit.
    let sep = match b.get(j) {
        Some(b'T' | b't') => true,
        Some(b' ') => b.get(j + 1).is_some_and(u8::is_ascii_digit),
        _ => false,
    };
    if !sep {
        return Ok((
            Datetime {
                date: Some(d),
                time: None,
                offset: None,
            },
            j,
        ));
    }
    let (t, k) = time(b, j + 1, optional_seconds)?;
    let (o, k) = offset(b, k)?;
    Ok((
        Datetime {
            date: Some(d),
            time: Some(t),
            offset: o,
        },
        k,
    ))
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

impl fmt::Display for Time {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02}:{:02}:{:02}", self.hour, self.minute, self.second)?;
        if self.nanosecond != 0 {
            let s = format!("{:09}", self.nanosecond);
            write!(f, ".{}", s.trim_end_matches('0'))?;
        }
        Ok(())
    }
}

impl fmt::Display for Offset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Offset::Z => f.write_str("Z"),
            Offset::Minutes(m) => {
                let sign = if m < 0 { '-' } else { '+' };
                let m = m.unsigned_abs();
                write!(f, "{sign}{:02}:{:02}", m / 60, m % 60)
            }
        }
    }
}

impl fmt::Display for Datetime {
    /// RFC 3339: `1979-05-27T07:32:00Z`, `1979-05-27T07:32:00`,
    /// `1979-05-27`, `07:32:00` (seconds always written).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(d) = &self.date {
            write!(f, "{d}")?;
            if self.time.is_some() {
                f.write_str("T")?;
            }
        }
        if let Some(t) = &self.time {
            write!(f, "{t}")?;
        }
        if let Some(o) = &self.offset {
            write!(f, "{o}")?;
        }
        Ok(())
    }
}
