use chrono::NaiveDate;
use hopes::domain::{Item, Repeat};

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

fn rule(text: &str) -> Repeat {
    Repeat::parse(text).unwrap().unwrap()
}

#[test]
fn parses_intervals_weekdays_and_nothing() {
    assert_eq!(Repeat::parse(""), Ok(None));
    assert_eq!(Repeat::parse("never"), Ok(None));
    assert_eq!(rule("daily").to_string(), "daily");
    assert_eq!(rule("every week").to_string(), "weekly");
    assert_eq!(rule("2 weeks").to_string(), "every 2w");
    assert_eq!(rule("every 3d").to_string(), "every 3d");
    assert_eq!(rule("annually").to_string(), "yearly");
    assert_eq!(rule("every fri").to_string(), "every Fri");
    assert_eq!(rule("fri mon, wed").to_string(), "Mon, Wed, Fri");
    assert_eq!(rule("mon tue wed thu fri").to_string(), "weekdays");
    assert_eq!(rule("weekends").to_string(), "weekends");
    assert!(Repeat::parse("sometimes").is_err());
    assert!(Repeat::parse("0d").is_err());
    assert!(Repeat::parse("every fri blah").is_err());
}

#[test]
fn display_text_parses_back_to_the_same_rule() {
    for text in [
        "daily",
        "every 2w",
        "monthly",
        "every Fri",
        "Mon, Wed",
        "weekdays",
    ] {
        let r = rule(text);
        assert_eq!(rule(&r.to_string()), r, "{text}");
    }
}

#[test]
fn next_occurrence_follows_the_old_due_date_not_today() {
    let weekly = rule("weekly");
    // Fri Oct 23 done on time, early, and on the day itself.
    assert_eq!(
        weekly.next_after(d(2026, 10, 23), d(2026, 10, 23)),
        d(2026, 10, 30)
    );
    assert_eq!(
        weekly.next_after(d(2026, 10, 23), d(2026, 10, 20)),
        d(2026, 10, 30)
    );
    // Done late: skip the missed cycle but stay on the same weekday.
    assert_eq!(
        weekly.next_after(d(2026, 10, 23), d(2026, 11, 5)),
        d(2026, 11, 6)
    );
    assert_eq!(
        rule("daily").next_after(d(2026, 10, 23), d(2026, 10, 25)),
        d(2026, 10, 25)
    );
    assert_eq!(
        rule("every 3d").next_after(d(2026, 10, 1), d(2026, 10, 1)),
        d(2026, 10, 4)
    );
}

#[test]
fn weekday_sets_pick_the_next_listed_day() {
    let r = rule("mon wed fri");
    assert_eq!(
        r.next_after(d(2026, 10, 23), d(2026, 10, 23)),
        d(2026, 10, 26)
    );
    assert_eq!(
        r.next_after(d(2026, 10, 26), d(2026, 10, 26)),
        d(2026, 10, 28)
    );
    assert_eq!(r.first_from(d(2026, 10, 24)), d(2026, 10, 26));
    assert_eq!(r.first_from(d(2026, 10, 23)), d(2026, 10, 23));
}

#[test]
fn months_clamp_without_drifting() {
    let monthly = rule("monthly");
    assert_eq!(
        monthly.next_after(d(2026, 1, 31), d(2026, 1, 31)),
        d(2026, 2, 28)
    );
    assert_eq!(
        monthly.next_after(d(2026, 1, 31), d(2026, 3, 1)),
        d(2026, 3, 31)
    );
    assert_eq!(
        rule("yearly").next_after(d(2024, 2, 29), d(2024, 2, 29)),
        d(2025, 2, 28)
    );
}

#[test]
fn stored_as_readable_text_and_optional_for_old_data() {
    let mut item = Item::new("show".into(), vec![]);
    item.repeat = Repeat::parse("every fri").unwrap();
    let json = serde_json::to_string(&item).unwrap();
    assert!(json.contains(r#""repeat":"every Fri""#), "{json}");
    assert_eq!(serde_json::from_str::<Item>(&json).unwrap(), item);

    let old = r#"{"title":"x","priority":"Medium","status":"Todo"}"#;
    assert_eq!(serde_json::from_str::<Item>(old).unwrap().repeat, None);
}
