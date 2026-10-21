use super::due::InvalidDate;
use chrono::{Datelike, Duration, Months, NaiveDate, Weekday};
use serde::{Deserialize, Serialize};
use std::fmt;

const MAX_INTERVAL: u32 = 999;
const MON_FRI: u8 = 0b001_1111;
const SAT_SUN: u8 = 0b110_0000;

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum Unit {
    Day,
    Week,
    Month,
    Year,
}

/// How a task comes back after it is completed.
///
/// Serialized as its display text (`weekly`, `every 2w`, `Mon, Wed`) so data files stay readable.
#[derive(PartialEq, Eq, Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum Repeat {
    /// Every `n` days / weeks / months / years.
    Every(u32, Unit),
    /// On the given weekdays, bit 0 = Monday.
    On(u8),
}

impl Repeat {
    /// Parses `daily`, `weekly`, `every 2 weeks`, `3d`, `every fri`, `mon wed fri`, `weekdays`.
    /// Empty input is `Ok(None)` (does not repeat).
    pub fn parse(input: &str) -> Result<Option<Self>, InvalidDate> {
        let text = input.trim().to_lowercase();
        if text.is_empty() || matches!(text.as_str(), "none" | "never" | "off" | "-") {
            return Ok(None);
        }
        parse_repeat(&text).map(Some).ok_or(InvalidDate)
    }

    /// First occurrence on or after `from`; where a new repeating task starts.
    pub fn first_from(&self, from: NaiveDate) -> NaiveDate {
        match self {
            Self::Every(..) => from,
            Self::On(mask) => next_weekday(*mask, from),
        }
    }

    /// The occurrence after completing the one due on `base`: the first one past `base` that is
    /// not already behind `today`, so missed cycles are skipped rather than replayed.
    pub fn next_after(&self, base: NaiveDate, today: NaiveDate) -> NaiveDate {
        let earliest = (base + Duration::days(1)).max(today);
        match *self {
            Self::On(mask) => next_weekday(mask, earliest),
            Self::Every(n, Unit::Day) => step_days(base, n as i64, earliest),
            Self::Every(n, Unit::Week) => step_days(base, n as i64 * 7, earliest),
            Self::Every(n, Unit::Month) => step_months(base, n, earliest),
            Self::Every(n, Unit::Year) => step_months(base, n * 12, earliest),
        }
    }
}

fn step_days(base: NaiveDate, step: i64, earliest: NaiveDate) -> NaiveDate {
    let gap = (earliest - base).num_days();
    base + Duration::days(((gap + step - 1) / step).max(1) * step)
}

/// Steps from `base` itself each time so a 31st clamps to month end without drifting.
fn step_months(base: NaiveDate, step: u32, earliest: NaiveDate) -> NaiveDate {
    (1..)
        .map_while(|k| base.checked_add_months(Months::new(step * k)))
        .find(|d| *d >= earliest)
        .unwrap_or(earliest)
}

fn next_weekday(mask: u8, from: NaiveDate) -> NaiveDate {
    (0..7)
        .map(|i| from + Duration::days(i))
        .find(|d| mask & (1 << d.weekday().num_days_from_monday()) != 0)
        .unwrap_or(from)
}

impl fmt::Display for Repeat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Every(1, Unit::Day) => f.write_str("daily"),
            Self::Every(1, Unit::Week) => f.write_str("weekly"),
            Self::Every(1, Unit::Month) => f.write_str("monthly"),
            Self::Every(1, Unit::Year) => f.write_str("yearly"),
            Self::Every(n, unit) => {
                let c = match unit {
                    Unit::Day => 'd',
                    Unit::Week => 'w',
                    Unit::Month => 'm',
                    Unit::Year => 'y',
                };
                write!(f, "every {n}{c}")
            }
            Self::On(MON_FRI) => f.write_str("weekdays"),
            Self::On(SAT_SUN) => f.write_str("weekends"),
            Self::On(mask) => {
                let days: Vec<String> = ALL_DAYS
                    .iter()
                    .filter(|d| mask & bit(**d) != 0)
                    .map(|d| d.to_string())
                    .collect();
                if let [only] = days.as_slice() {
                    write!(f, "every {only}")
                } else {
                    f.write_str(&days.join(", "))
                }
            }
        }
    }
}

impl From<Repeat> for String {
    fn from(repeat: Repeat) -> Self {
        repeat.to_string()
    }
}

impl TryFrom<String> for Repeat {
    type Error = String;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        match Repeat::parse(&text) {
            Ok(Some(repeat)) => Ok(repeat),
            _ => Err(format!("invalid repeat rule: {text:?}")),
        }
    }
}

const ALL_DAYS: [Weekday; 7] = [
    Weekday::Mon,
    Weekday::Tue,
    Weekday::Wed,
    Weekday::Thu,
    Weekday::Fri,
    Weekday::Sat,
    Weekday::Sun,
];

fn bit(day: Weekday) -> u8 {
    1 << day.num_days_from_monday()
}

fn parse_repeat(text: &str) -> Option<Repeat> {
    let body = text.strip_prefix("every").map_or(text, str::trim);
    match body {
        "day" | "daily" => return Some(Repeat::Every(1, Unit::Day)),
        "week" | "weekly" => return Some(Repeat::Every(1, Unit::Week)),
        "month" | "monthly" => return Some(Repeat::Every(1, Unit::Month)),
        "year" | "yearly" | "annually" => return Some(Repeat::Every(1, Unit::Year)),
        "weekday" | "weekdays" => return Some(Repeat::On(MON_FRI)),
        "weekend" | "weekends" => return Some(Repeat::On(SAT_SUN)),
        _ => {}
    }

    if let Some(repeat) = parse_interval(body) {
        return Some(repeat);
    }

    let mut mask = 0u8;
    let words = body
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|w| !w.is_empty());
    for word in words {
        mask |= bit(weekday_of(word)?);
    }
    (mask != 0).then_some(Repeat::On(mask))
}

/// `2w`, `3 days`, `1 month`.
fn parse_interval(body: &str) -> Option<Repeat> {
    let split = body.find(|c: char| !c.is_ascii_digit())?;
    let (digits, unit) = body.split_at(split);
    let n: u32 = digits.parse().ok()?;
    if n == 0 || n > MAX_INTERVAL {
        return None;
    }
    let unit = match unit.trim() {
        "d" | "day" | "days" => Unit::Day,
        "w" | "wk" | "week" | "weeks" => Unit::Week,
        "m" | "mo" | "month" | "months" => Unit::Month,
        "y" | "yr" | "year" | "years" => Unit::Year,
        _ => return None,
    };
    Some(Repeat::Every(n, unit))
}

fn weekday_of(word: &str) -> Option<Weekday> {
    Some(match word {
        "mon" | "monday" => Weekday::Mon,
        "tue" | "tues" | "tuesday" => Weekday::Tue,
        "wed" | "weds" | "wednesday" => Weekday::Wed,
        "thu" | "thur" | "thurs" | "thursday" => Weekday::Thu,
        "fri" | "friday" => Weekday::Fri,
        "sat" | "saturday" => Weekday::Sat,
        "sun" | "sunday" => Weekday::Sun,
        _ => return None,
    })
}
