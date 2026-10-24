use super::due::{InvalidDate, weekday_of};
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

#[derive(PartialEq, Eq, Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum Repeat {
    Every(u32, Unit),
    On(u8),
}

impl Repeat {
    pub fn parse(input: &str) -> Result<Option<Self>, InvalidDate> {
        let text = input.trim().to_lowercase();
        if text.is_empty() || matches!(text.as_str(), "none" | "never" | "off" | "-") {
            return Ok(None);
        }
        parse_repeat(&text).map(Some).ok_or(InvalidDate)
    }

    pub fn first_from(&self, from: NaiveDate) -> NaiveDate {
        match self {
            Self::Every(..) => from,
            Self::On(mask) => next_weekday(*mask, from),
        }
    }

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
    let steps = ((gap + step - 1) / step).max(1);
    base + Duration::days(steps * step)
}

fn step_months(base: NaiveDate, step: u32, earliest: NaiveDate) -> NaiveDate {
    for k in 1.. {
        let Some(date) = base.checked_add_months(Months::new(step * k)) else {
            break;
        };
        if date >= earliest {
            return date;
        }
    }
    earliest
}

fn next_weekday(mask: u8, from: NaiveDate) -> NaiveDate {
    for offset in 0..7 {
        let date = from + Duration::days(offset);
        if mask & bit(date.weekday()) != 0 {
            return date;
        }
    }
    from
}

impl fmt::Display for Repeat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Every(1, Unit::Day) => f.write_str("daily"),
            Self::Every(1, Unit::Week) => f.write_str("weekly"),
            Self::Every(1, Unit::Month) => f.write_str("monthly"),
            Self::Every(1, Unit::Year) => f.write_str("yearly"),
            Self::Every(n, unit) => {
                let letter = match unit {
                    Unit::Day => 'd',
                    Unit::Week => 'w',
                    Unit::Month => 'm',
                    Unit::Year => 'y',
                };
                write!(f, "every {n}{letter}")
            }
            Self::On(MON_FRI) => f.write_str("weekdays"),
            Self::On(SAT_SUN) => f.write_str("weekends"),
            Self::On(mask) => {
                let mut days: Vec<String> = Vec::new();
                for day in ALL_DAYS {
                    if mask & bit(day) != 0 {
                        days.push(day.to_string());
                    }
                }
                if days.len() == 1 {
                    write!(f, "every {}", days[0])
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
    let body = match text.strip_prefix("every") {
        Some(rest) => rest.trim(),
        None => text,
    };
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
    for word in body.split(|c: char| c.is_whitespace() || c == ',') {
        if word.is_empty() {
            continue;
        }
        mask |= bit(weekday_of(word)?);
    }
    if mask == 0 {
        return None;
    }
    Some(Repeat::On(mask))
}

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
