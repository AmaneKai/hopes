use chrono::{Datelike, Duration, NaiveDate, Weekday};

/// Days ahead that still count as "soon".
pub const SOON_DAYS: i64 = 2;

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum DueUrgency {
    Overdue,
    Today,
    Soon,
    Later,
    /// The window has not opened yet.
    Upcoming,
}

pub fn urgency(due: NaiveDate, today: NaiveDate) -> DueUrgency {
    match (due - today).num_days() {
        d if d < 0 => DueUrgency::Overdue,
        0 => DueUrgency::Today,
        d if d <= SOON_DAYS => DueUrgency::Soon,
        _ => DueUrgency::Later,
    }
}

/// When a task has to be done: by `end`, and from `start` if the window opens later.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct Window {
    pub start: Option<NaiveDate>,
    pub end: NaiveDate,
}

impl Window {
    pub fn urgency(&self, today: NaiveDate) -> DueUrgency {
        match self.start {
            Some(start) if start > today => DueUrgency::Upcoming,
            _ => urgency(self.end, today),
        }
    }

    /// Compact label for lists: `from Sat` before the window opens, `3d left` inside a
    /// multi-day window, otherwise the same as [`relative_label`] for the end date.
    pub fn label(&self, today: NaiveDate) -> String {
        match self.start {
            Some(start) if start > today => format!("from {}", relative_label(start, today)),
            Some(_) if self.end > today => format!("{}d left", (self.end - today).num_days()),
            _ => relative_label(self.end, today),
        }
    }

    /// Full dates for detail views: `Fri, Oct 23 2026` or `Oct 23 – Sun, Oct 25 2026`.
    pub fn full_label(&self) -> String {
        match self.start {
            Some(start) => format!(
                "{} \u{2013} {}",
                start.format("%b %-d"),
                full_label(self.end)
            ),
            None => full_label(self.end),
        }
    }

    /// Shifts the window to end on `end`, keeping its length.
    pub fn moved_to(&self, end: NaiveDate) -> Self {
        Self {
            start: self.start.map(|s| end - (self.end - s)),
            end,
        }
    }
}

/// Compact label for lists: `today`, `tomorrow`, `3d`, `2d late`, `Fri`, `Oct 25`.
pub fn relative_label(due: NaiveDate, today: NaiveDate) -> String {
    let days = (due - today).num_days();
    match days {
        0 => "today".into(),
        1 => "tomorrow".into(),
        -1 => "yesterday".into(),
        d if d < 0 => format!("{}d late", -d),
        2..=6 => due.format("%a").to_string(),
        _ if due.year() == today.year() => due.format("%b %-d").to_string(),
        _ => due.format("%b %-d %Y").to_string(),
    }
}

/// Full date for detail views: `Fri, Oct 24 2026`.
pub fn full_label(due: NaiveDate) -> String {
    due.format("%a, %b %-d %Y").to_string()
}

/// The typed text is not a recognisable date.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct InvalidDate;

/// Parses free-form input relative to `today`. Empty input is `Ok(None)` (no due date).
///
/// Accepts `today`, `tomorrow`, `eow`, `eom`, weekdays (`fri`, `next mon`), offsets
/// (`+3d`, `2w`, `in 3 days`, `1m`), ISO dates (`2026-10-25`), and `oct 25` / `25 oct` / `10/25`.
pub fn parse(input: &str, today: NaiveDate) -> Result<Option<NaiveDate>, InvalidDate> {
    let text = input.trim().to_lowercase();
    if text.is_empty() || matches!(text.as_str(), "none" | "clear" | "-") {
        return Ok(None);
    }
    parse_date(&text, today).map(Some).ok_or(InvalidDate)
}

/// Like [`parse`], but also accepts a range: `fri..sun`, `oct 25 - oct 30`, `mon to fri`.
/// The end is read relative to the start, so `mon..fri` means that Monday through that Friday.
pub fn parse_window(input: &str, today: NaiveDate) -> Result<Option<Window>, InvalidDate> {
    let text = input.trim().to_lowercase();
    let Some((a, b)) = split_range(&text) else {
        return Ok(parse(&text, today)?.map(|end| Window { start: None, end }));
    };
    let start = parse_date(a.trim(), today).ok_or(InvalidDate)?;
    let b = b.trim();
    // A bare weekday may land on the start day itself (`fri..fri`), unlike a standalone `fri`.
    let end_base = if weekday_of(b).is_some() {
        start - Duration::days(1)
    } else {
        start
    };
    let end = parse_date(b, end_base).ok_or(InvalidDate)?;
    if end < start {
        return Err(InvalidDate);
    }
    Ok(Some(Window {
        start: (start < end).then_some(start),
        end,
    }))
}

/// Splits on `..`, ` to `, or a spaced dash; bare dashes belong to ISO dates like `10-25`.
fn split_range(text: &str) -> Option<(&str, &str)> {
    [" to ", " - ", " \u{2013} ", ".."]
        .iter()
        .find_map(|sep| text.split_once(sep))
}

fn parse_date(text: &str, today: NaiveDate) -> Option<NaiveDate> {
    match text {
        "today" | "tod" | "now" => return Some(today),
        "tomorrow" | "tom" | "tmr" | "tmrw" => return Some(today + Duration::days(1)),
        "yesterday" => return Some(today - Duration::days(1)),
        "eow" => return Some(end_of_week(today)),
        "eom" => return Some(end_of_month(today)),
        _ => {}
    }

    if let Ok(date) = NaiveDate::parse_from_str(text, "%Y-%m-%d") {
        return Some(date);
    }

    let words: Vec<&str> = text
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|w| !w.is_empty())
        .collect();

    let (next, rest) = match words.as_slice() {
        ["next" | "this", rest @ ..] => (text.starts_with("next"), rest),
        all => (false, all),
    };

    if let [word] = rest
        && let Some(weekday) = weekday_of(word)
    {
        return Some(upcoming(today, weekday, next));
    }

    parse_offset(text, today)
        .or_else(|| parse_month_day(&words, today))
        .or_else(|| parse_numeric_month_day(text, today))
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

/// Next occurrence of `weekday` strictly after today; `next` instead picks that weekday
/// in the following Monday-based week.
fn upcoming(today: NaiveDate, weekday: Weekday, next: bool) -> NaiveDate {
    let from = today.weekday().num_days_from_monday() as i64;
    let to = weekday.num_days_from_monday() as i64;
    let ahead = if next {
        7 - from + to
    } else {
        match (to - from).rem_euclid(7) {
            0 => 7,
            d => d,
        }
    };
    today + Duration::days(ahead)
}

fn end_of_week(today: NaiveDate) -> NaiveDate {
    let ahead = (Weekday::Fri.num_days_from_monday() as i64
        - today.weekday().num_days_from_monday() as i64)
        .max(0);
    today + Duration::days(ahead)
}

fn end_of_month(today: NaiveDate) -> NaiveDate {
    let (year, month) = if today.month() == 12 {
        (today.year() + 1, 1)
    } else {
        (today.year(), today.month() + 1)
    };
    NaiveDate::from_ymd_opt(year, month, 1)
        .map(|first| first - Duration::days(1))
        .unwrap_or(today)
}

/// `+3d`, `3d`, `2w`, `1m`, `in 3 days`, `in 2 weeks`.
fn parse_offset(text: &str, today: NaiveDate) -> Option<NaiveDate> {
    let body = text.strip_prefix("in ").unwrap_or(text);
    let body = body.strip_prefix('+').unwrap_or(body).trim();
    let split = body.find(|c: char| !c.is_ascii_digit())?;
    let (digits, unit) = body.split_at(split);
    let n: i64 = digits.parse().ok()?;
    match unit.trim() {
        "d" | "day" | "days" => Some(today + Duration::days(n)),
        "w" | "wk" | "week" | "weeks" => Some(today + Duration::weeks(n)),
        "m" | "mo" | "month" | "months" => add_months(today, u32::try_from(n).ok()?),
        _ => None,
    }
}

fn add_months(date: NaiveDate, months: u32) -> Option<NaiveDate> {
    date.checked_add_months(chrono::Months::new(months))
}

fn month_of(word: &str) -> Option<u32> {
    const NAMES: [&str; 12] = [
        "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
    ];
    if word.len() < 3 {
        return None;
    }
    let idx = NAMES.iter().position(|m| word.starts_with(m))?;
    Some(idx as u32 + 1)
}

fn day_number(word: &str) -> Option<u32> {
    let digits = word.trim_end_matches(|c: char| c.is_ascii_alphabetic());
    if !word[digits.len()..].is_empty()
        && !matches!(&word[digits.len()..], "st" | "nd" | "rd" | "th")
    {
        return None;
    }
    digits.parse().ok()
}

/// `oct 25`, `25 oct`, `oct 25 2027`.
fn parse_month_day(words: &[&str], today: NaiveDate) -> Option<NaiveDate> {
    let (month, day, year) = match words {
        [a, b] | [a, b, _] if month_of(a).is_some() => (month_of(a)?, day_number(b)?, words.get(2)),
        [a, b] | [a, b, _] if month_of(b).is_some() => (month_of(b)?, day_number(a)?, words.get(2)),
        _ => return None,
    };
    resolve_month_day(month, day, year.and_then(|y| y.parse().ok()), today)
}

/// `10/25`, `10-25`, `10/25/2027`.
fn parse_numeric_month_day(text: &str, today: NaiveDate) -> Option<NaiveDate> {
    let parts: Vec<u32> = text
        .split(['/', '-'])
        .map(|p| p.trim().parse().ok())
        .collect::<Option<_>>()?;
    match parts.as_slice() {
        [m, d] => resolve_month_day(*m, *d, None, today),
        [m, d, y] => resolve_month_day(*m, *d, Some(*y as i32), today),
        _ => None,
    }
}

/// Without an explicit year, a date already past rolls into next year.
fn resolve_month_day(
    month: u32,
    day: u32,
    year: Option<i32>,
    today: NaiveDate,
) -> Option<NaiveDate> {
    if let Some(year) = year {
        return NaiveDate::from_ymd_opt(year, month, day);
    }
    let this_year = NaiveDate::from_ymd_opt(today.year(), month, day)?;
    if this_year >= today {
        Some(this_year)
    } else {
        NaiveDate::from_ymd_opt(today.year() + 1, month, day)
    }
}
