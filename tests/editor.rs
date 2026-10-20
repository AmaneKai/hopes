use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use hopes::{
    action::Action,
    domain::{AppState, Item, Priority, Status},
    systems::sync::SyncStatus,
    ui::{Component, KeyResult, components::editor::EditorComponent},
};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new_with_kind(code, KeyModifiers::NONE, KeyEventKind::Press)
}

fn state_with(items: Vec<Item>) -> AppState {
    AppState::new(items, SyncStatus::Disabled)
}

fn opened(state: &AppState, status: Status) -> EditorComponent {
    let mut editor = EditorComponent::default();
    editor.update(&Action::OpenCreateModal { status }, state);
    editor
}

fn type_str(editor: &mut EditorComponent, state: &AppState, text: &str) {
    for c in text.chars() {
        editor.handle_key(key(KeyCode::Char(c)), state);
    }
}

fn is_handled(result: &KeyResult) -> bool {
    matches!(result, KeyResult::Handled)
}

#[test]
fn esc_leaves_insert_before_cancelling_the_modal() {
    let state = state_with(Vec::new());
    let mut editor = opened(&state, Status::Todo);
    type_str(&mut editor, &state, "draft");

    assert!(is_handled(&editor.handle_key(key(KeyCode::Esc), &state)));
    assert!(!editor.text_input().unwrap().insert);

    assert!(matches!(
        editor.handle_key(key(KeyCode::Esc), &state),
        KeyResult::Emit(Action::CloseModal)
    ));
}

#[test]
fn esc_on_the_priority_field_closes_at_once() {
    let state = state_with(Vec::new());
    let mut editor = opened(&state, Status::Todo);
    editor.handle_key(key(KeyCode::Tab), &state);
    editor.handle_key(key(KeyCode::Tab), &state);
    assert!(editor.text_input().is_none());

    assert!(matches!(
        editor.handle_key(key(KeyCode::Esc), &state),
        KeyResult::Emit(Action::CloseModal)
    ));
}

#[test]
fn tabbing_between_fields_resets_to_insert_at_the_end() {
    let state = state_with(Vec::new());
    let mut editor = opened(&state, Status::Todo);
    editor.handle_key(key(KeyCode::Tab), &state);
    type_str(&mut editor, &state, "notes");
    editor.handle_key(key(KeyCode::Esc), &state);
    assert!(!editor.text_input().unwrap().insert);

    editor.handle_key(key(KeyCode::Tab), &state);
    editor.handle_key(key(KeyCode::BackTab), &state);
    let input = editor.text_input().unwrap();
    assert!(input.insert);
    assert_eq!(input.cursor, 5);
}

#[test]
fn saving_needs_a_title_and_carries_the_form_values() {
    let state = state_with(Vec::new());
    let mut editor = opened(&state, Status::InProgress);

    assert!(is_handled(&editor.handle_key(key(KeyCode::Enter), &state)));

    type_str(&mut editor, &state, "  ship it  ");
    editor.handle_key(key(KeyCode::Tab), &state);
    editor.handle_key(key(KeyCode::Tab), &state);
    editor.handle_key(key(KeyCode::Char('1')), &state);
    editor.handle_key(key(KeyCode::Tab), &state);
    type_str(&mut editor, &state, "a, b,, c");

    let KeyResult::Emit(Action::SaveTask(draft)) = editor.handle_key(key(KeyCode::Enter), &state)
    else {
        panic!("expected SaveTask");
    };
    assert_eq!(draft.id, None);
    assert_eq!(draft.title, "ship it");
    assert_eq!(draft.priority, Priority::Urgent);
    assert_eq!(draft.tags, vec!["a", "b", "c"]);
    assert_eq!(draft.status, Status::InProgress);
}

#[test]
fn enter_in_the_notes_field_adds_a_line_instead_of_saving() {
    let state = state_with(Vec::new());
    let mut editor = opened(&state, Status::Todo);
    type_str(&mut editor, &state, "title");
    editor.handle_key(key(KeyCode::Tab), &state);
    type_str(&mut editor, &state, "one");
    assert!(is_handled(&editor.handle_key(key(KeyCode::Enter), &state)));
    type_str(&mut editor, &state, "two");

    let ctrl_s = KeyEvent::new_with_kind(
        KeyCode::Char('s'),
        KeyModifiers::CONTROL,
        KeyEventKind::Press,
    );
    let KeyResult::Emit(Action::SaveTask(draft)) = editor.handle_key(ctrl_s, &state) else {
        panic!("expected SaveTask");
    };
    assert_eq!(draft.description, "one\ntwo");
}

#[test]
fn editing_an_existing_task_loads_its_fields_and_targets_it() {
    let mut task = Item::new(
        "Existing".to_string(),
        vec!["x".to_string(), "y".to_string()],
    );
    task.description = "details".to_string();
    task.priority = Priority::High;
    let state = state_with(vec![task]);

    let mut editor = EditorComponent::default();
    editor.update(&Action::OpenEditModal(0), &state);

    let KeyResult::Emit(Action::SaveTask(draft)) = editor.handle_key(key(KeyCode::Enter), &state)
    else {
        panic!("expected SaveTask");
    };
    assert_eq!(draft.id, Some(0));
    assert_eq!(draft.title, "Existing");
    assert_eq!(draft.description, "details");
    assert_eq!(draft.priority, Priority::High);
    assert_eq!(draft.tags, vec!["x", "y"]);
}

#[test]
fn reopening_the_form_starts_blank() {
    let state = state_with(Vec::new());
    let mut editor = opened(&state, Status::Todo);
    type_str(&mut editor, &state, "leftover");

    editor.update(
        &Action::OpenCreateModal {
            status: Status::Todo,
        },
        &state,
    );
    assert!(is_handled(&editor.handle_key(key(KeyCode::Enter), &state)));
}

#[test]
fn ctrl_c_cancels_from_any_field() {
    let state = state_with(Vec::new());
    let mut editor = opened(&state, Status::Todo);
    type_str(&mut editor, &state, "abc");

    let ctrl_c = KeyEvent::new_with_kind(
        KeyCode::Char('c'),
        KeyModifiers::CONTROL,
        KeyEventKind::Press,
    );
    assert!(matches!(
        editor.handle_key(ctrl_c, &state),
        KeyResult::Emit(Action::CloseModal)
    ));
}
