use hopes::{
    domain::{AppState, Item, NavSection, SortMode, ViewMode},
    systems::{sync::SyncStatus, view_state::ViewState},
};

fn state() -> AppState {
    AppState::new(
        vec![Item::new("a".to_string(), vec!["uni".to_string()])],
        SyncStatus::Disabled,
    )
}

#[test]
fn a_saved_tag_filter_is_restored() {
    let mut saved = state();
    saved.tag_filter = Some("#uni".to_string());
    saved.view_mode = ViewMode::Kanban;
    saved.sort_mode = SortMode::Priority;
    saved.nav_section = NavSection::Todo;

    let json = serde_json::to_string(&ViewState::capture(&saved)).unwrap();
    let mut fresh = state();
    serde_json::from_str::<ViewState>(&json)
        .unwrap()
        .apply(&mut fresh);

    assert_eq!(fresh.tag_filter.as_deref(), Some("#uni"));
    assert_eq!(fresh.view_mode, ViewMode::Kanban);
    assert_eq!(fresh.sort_mode, SortMode::Priority);
    assert_eq!(fresh.nav_section, NavSection::Todo);
}

#[test]
fn a_filter_for_a_vanished_tag_is_dropped() {
    let mut saved = state();
    saved.tag_filter = Some("#gone".to_string());
    let mut fresh = state();
    ViewState::capture(&saved).apply(&mut fresh);
    assert_eq!(fresh.tag_filter, None);
}
