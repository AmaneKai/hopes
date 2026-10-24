use chrono::{Datelike, Duration, NaiveDate, Weekday};

pub const SOON_DAYS: i64 = 2;

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum DueUrgency {
    Overdue,
    Today,
    Soon,
    Later,
    Upcoming,
}

pub fn urgency(due: NaiveDate, today: NaiveDate) -> DueUrgency {
    match (due - today).num_days() {
        ..=-1 => DueUrgency::Overdue,
        0 => DueUrgency::Today,
        1..=SOON_DAYS => DueUrgency::Soon,
        _ => DueUrgency::Later,
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct Window {
    pub start: Option<NaiveDate>,
    pub end: NaiveDate,
}

impl Window {
    pub fn urgency(&self, today: NaiveDate) -> DueUrgency {
        if let Some(start) = self.start
            && start > today
        {
            return DueUrgency::Upcoming;
        }
        urgency(self.end, today)
    }

    pub fn label(&self, today: NaiveDate) -> String {
        if let Some(start) = self.start {
            if start > today {
                return format!("from {}", relative_label(start, today));
            }
            if self.end > today {
                return format!("{}d left", (self.end - today).num_days());
            }
        }
        relative_label(self.end, today)
    }

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

    pub fn moved_to(&self, end: NaiveDate) -> Self {
        Self {
            start: self.start.map(|start| end - (self.end - start)),
            end,
        }
    }
}

pub fn relative_label(due: NaiveDate, today: NaiveDate) -> String {
    let days = (due - today).num_days();
    match days {
        ..=-2 => format!("{}d late", -days),
        -1 => "yesterday".to_string(),
        0 => "today".to_string(),
        1 => "tomorrow".to_string(),
        2..=6 => due.format("%a").to_string(),
        7.. => far_label(due, today),
    }
}

fn far_label(due: NaiveDate, today: NaiveDate) -> String {
    if due.year() == today.year() {
        due.format("%b %-d").to_string()
    } else {
        due.format("%b %-d %Y").to_string()
    }
}

pub fn full_label(due: NaiveDate) -> String {
    due.format("%a, %b %-d %Y").to_string()
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct InvalidDate;

pub fn parse(input: &str, today: NaiveDate) -> Result<Option<NaiveDate>, InvalidDate> {
    let text = input.trim().to_lowercase();
    if text.is_empty() || matches!(text.as_str(), "none" | "clear" | "-") {
        return Ok(None);
    }
    parse_date(&text, today).map(Some).ok_or(InvalidDate)
}

pub fn parse_window(input: &str, today: NaiveDate) -> Result<Option<Window>, InvalidDate> {
    let text = input.trim().to_lowercase();

    let Some((start_text, end_text)) = split_range(&text) else {
        return match parse(&text, today)? {
            Some(end) => Ok(Some(Window { start: None, end })),
            None => Ok(None),
        };
    };

    let start = parse_date(start_text.trim(), today).ok_or(InvalidDate)?;
    let end_text = end_text.trim();

    let end_base = if weekday_of(end_text).is_some() {
        start - Duration::days(1)
    } else {
        start
    };

    let end = parse_date(end_text, end_base).ok_or(InvalidDate)?;

    if end < start {
        return Err(InvalidDate);
    }

    let start = if start < end { Some(start) } else { None };
    Ok(Some(Window { start, end }))
}

fn split_range(text: &str) -> Option<(&str, &str)> {
    for separator in [" to ", " - ", " \u{2013} ", ".."] {
        if let Some(parts) = text.split_once(separator) {
            return Some(parts);
        }
    }
    None
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

    let next = text.starts_with("next");
    let weekday_words = match words.as_slice() {
        ["next" | "this", rest @ ..] => rest,
        all => all,
    };

    let is_modified = weekday_words.len() < words.len();
    if let [word] = weekday_words
        && let Some(weekday) = weekday_of(word)
    {
        return Some(upcoming(today, weekday, is_modified && next));
    }

    if let Some(date) = parse_offset(text, today) {
        return Some(date);
    }

    if let Some(date) = parse_month_day(&words, today) {
        return Some(date);
    }

    parse_numeric_month_day(text, today)
}

pub fn weekday_of(word: &str) -> Option<Weekday> {
    match word {
        "mon" | "monday" => Some(Weekday::Mon),
        "tue" | "tues" | "tuesday" => Some(Weekday::Tue),
        "wed" | "weds" | "wednesday" => Some(Weekday::Wed),
        "thu" | "thur" | "thurs" | "thursday" => Some(Weekday::Thu),
        "fri" | "friday" => Some(Weekday::Fri),
        "sat" | "saturday" => Some(Weekday::Sat),
        "sun" | "sunday" => Some(Weekday::Sun),
        _ => None,
    }
}

fn upcoming(today: NaiveDate, weekday: Weekday, next: bool) -> NaiveDate {
    let from = today.weekday().num_days_from_monday() as i64;
    let to = weekday.num_days_from_monday() as i64;
    let ahead = if next {
        7 - from + to
    } else {
        let days = (to - from).rem_euclid(7);
        if days == 0 { 7 } else { days }
    };

    today + Duration::days(ahead)
}

fn end_of_week(today: NaiveDate) -> NaiveDate {
    let friday = Weekday::Fri.num_days_from_monday() as i64;
    let current = today.weekday().num_days_from_monday() as i64;
    let ahead = (friday - current).max(0);

    today + Duration::days(ahead)
}

fn end_of_month(today: NaiveDate) -> NaiveDate {
    let (year, month) = if today.month() == 12 {
        (today.year() + 1, 1)
    } else {
        (today.year(), today.month() + 1)
    };
    match NaiveDate::from_ymd_opt(year, month, 1) {
        Some(first_of_next) => first_of_next - Duration::days(1),
        None => today,
    }
}

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
    let index = NAMES.iter().position(|name| word.starts_with(name))?;

    Some(index as u32 + 1)
}

fn day_number(word: &str) -> Option<u32> {
    let digits = word.trim_end_matches(|c: char| c.is_ascii_alphabetic());
    let suffix = &word[digits.len()..];

    if !suffix.is_empty() && !matches!(suffix, "st" | "nd" | "rd" | "th") {
        return None;
    }

    digits.parse().ok()
}

fn parse_month_day(words: &[&str], today: NaiveDate) -> Option<NaiveDate> {
    if words.len() != 2 && words.len() != 3 {
        return None;
    }

    let (first, second) = (words[0], words[1]);
    let (month, day) = if let Some(month) = month_of(first) {
        (month, day_number(second)?)
    } else if let Some(month) = month_of(second) {
        (month, day_number(first)?)
    } else {
        return None;
    };

    let year = match words.get(2) {
        Some(text) => text.parse().ok(),
        None => None,
    };

    resolve_month_day(month, day, year, today)
}

fn parse_numeric_month_day(text: &str, today: NaiveDate) -> Option<NaiveDate> {
    let mut parts: Vec<u32> = Vec::new();
    for piece in text.split(['/', '-']) {
        parts.push(piece.trim().parse().ok()?);
    }

    match parts.as_slice() {
        [month, day] => resolve_month_day(*month, *day, None, today),
        [month, day, year] => resolve_month_day(*month, *day, Some(*year as i32), today),
        _ => None,
    }
}

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
