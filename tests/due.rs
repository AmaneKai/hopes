use chrono::NaiveDate;
use hopes::domain::{
    AppState, Item, NavSection, SortMode, Status,
    due::{DueUrgency, InvalidDate, parse, relative_label, urgency},
};
use hopes::systems::sync::SyncStatus;

/// A Wednesday.
fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 10, 21).unwrap()
}

fn day(y: i32, m: u32, d: u32) -> Option<NaiveDate> {
    NaiveDate::from_ymd_opt(y, m, d)
}

fn p(input: &str) -> Result<Option<NaiveDate>, InvalidDate> {
    parse(input, today())
}

#[test]
fn empty_input_means_no_due_date() {
    assert_eq!(p(""), Ok(None));
    assert_eq!(p("   "), Ok(None));
    assert_eq!(p("none"), Ok(None));
}

#[test]
fn keywords_and_offsets() {
    assert_eq!(p("today"), Ok(Some(today())));
    assert_eq!(p("Tomorrow"), Ok(day(2026, 10, 22)));
    assert_eq!(p("+3d"), Ok(day(2026, 10, 24)));
    assert_eq!(p("2w"), Ok(day(2026, 11, 4)));
    assert_eq!(p("in 3 days"), Ok(day(2026, 10, 24)));
    assert_eq!(p("1m"), Ok(day(2026, 11, 21)));
    assert_eq!(p("eow"), Ok(day(2026, 10, 23)));
    assert_eq!(p("eom"), Ok(day(2026, 10, 31)));
}

#[test]
fn weekdays_are_strictly_in_the_future() {
    assert_eq!(p("fri"), Ok(day(2026, 10, 23)));
    assert_eq!(p("wed"), Ok(day(2026, 10, 28)));
    assert_eq!(p("mon"), Ok(day(2026, 10, 26)));
    assert_eq!(p("next mon"), Ok(day(2026, 10, 26)));
    assert_eq!(p("next fri"), Ok(day(2026, 10, 30)));
}

#[test]
fn calendar_dates() {
    assert_eq!(p("2026-12-01"), Ok(day(2026, 12, 1)));
    assert_eq!(p("oct 25"), Ok(day(2026, 10, 25)));
    assert_eq!(p("25 Oct"), Ok(day(2026, 10, 25)));
    assert_eq!(p("10/25"), Ok(day(2026, 10, 25)));
    assert_eq!(p("dec 3rd"), Ok(day(2026, 12, 3)));
    assert_eq!(p("oct 25 2027"), Ok(day(2027, 10, 25)));
}

#[test]
fn a_past_month_day_rolls_to_next_year() {
    assert_eq!(p("jan 5"), Ok(day(2027, 1, 5)));
    assert_eq!(p("oct 21"), Ok(Some(today())));
}

#[test]
fn garbage_and_impossible_dates_are_rejected() {
    assert_eq!(p("someday"), Err(InvalidDate));
    assert_eq!(p("feb 30"), Err(InvalidDate));
    assert_eq!(p("3x"), Err(InvalidDate));
    assert_eq!(p("2026-13-01"), Err(InvalidDate));
}

#[test]
fn labels_and_urgency() {
    let t = today();
    assert_eq!(relative_label(t, t), "today");
    assert_eq!(relative_label(day(2026, 10, 22).unwrap(), t), "tomorrow");
    assert_eq!(relative_label(day(2026, 10, 24).unwrap(), t), "Sat");
    assert_eq!(relative_label(day(2026, 10, 18).unwrap(), t), "3d late");
    assert_eq!(relative_label(day(2026, 12, 1).unwrap(), t), "Dec 1");
    assert_eq!(urgency(day(2026, 10, 20).unwrap(), t), DueUrgency::Overdue);
    assert_eq!(urgency(t, t), DueUrgency::Today);
    assert_eq!(urgency(day(2026, 10, 23).unwrap(), t), DueUrgency::Soon);
    assert_eq!(urgency(day(2026, 10, 30).unwrap(), t), DueUrgency::Later);
}

fn with_due(title: &str, due: Option<NaiveDate>) -> Item {
    let mut item = Item::new(title.into(), vec![]);
    item.due = due;
    item
}

#[test]
fn due_sort_puts_undated_last_and_overdue_view_skips_completed() {
    let mut done = with_due("done", day(2020, 1, 1));
    done.status = Status::Complete;
    let mut state = AppState::new(
        vec![
            with_due("none", None),
            with_due("late", day(2030, 1, 2)),
            with_due("overdue", day(2020, 1, 1)),
            done,
            with_due("soon", day(2030, 1, 1)),
        ],
        SyncStatus::Disabled,
    );

    state.sort_mode = SortMode::Due;
    state.refresh_cache();
    let order: Vec<_> = state
        .filtered_indices()
        .iter()
        .map(|&i| state.items[i].title.as_str())
        .collect();
    assert_eq!(order, ["overdue", "done", "soon", "late", "none"]);

    state.nav_section = NavSection::Overdue;
    state.refresh_cache();
    let overdue: Vec<_> = state
        .filtered_indices()
        .iter()
        .map(|&i| state.items[i].title.as_str())
        .collect();
    assert_eq!(overdue, ["overdue"]);
    assert_eq!(state.stats().overdue, 1);
}

#[test]
fn due_survives_a_json_round_trip_and_old_data_still_loads() {
    let item = with_due("a", day(2026, 10, 25));
    let json = serde_json::to_string(&item).unwrap();
    assert!(json.contains("2026-10-25"));
    assert_eq!(serde_json::from_str::<Item>(&json).unwrap(), item);

    let old = r#"{"title":"x","priority":"Medium","status":"Todo"}"#;
    assert_eq!(serde_json::from_str::<Item>(old).unwrap().due, None);
}
