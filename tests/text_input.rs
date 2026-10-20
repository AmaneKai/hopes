use crossterm::event::KeyCode;
use hopes::ui::text_input::TextInput;

fn typed(input: &mut TextInput, buf: &mut String, text: &str) {
    for c in text.chars() {
        input.handle_key(buf, KeyCode::Char(c), false, false);
    }
}

#[test]
fn insert_mode_types_at_cursor_not_just_at_the_end() {
    let mut input = TextInput::default();
    let mut buf = String::new();
    input.focus(&buf);

    typed(&mut input, &mut buf, "helloworld");
    assert_eq!(buf, "helloworld");
    assert_eq!(input.cursor, 10);

    for _ in 0..5 {
        input.handle_key(&mut buf, KeyCode::Left, false, false);
    }
    input.handle_key(&mut buf, KeyCode::Char(' '), false, false);
    assert_eq!(buf, "hello world");
    assert_eq!(input.cursor, 6);
}

#[test]
fn esc_leaves_insert_before_reporting_close() {
    let mut input = TextInput::default();
    let mut buf = "draft".to_string();
    input.focus(&buf);

    assert!(input.handle_esc(&mut buf));
    assert!(!input.insert);
    assert_eq!(buf, "draft");

    assert!(!input.handle_esc(&mut buf));
}

#[test]
fn dd_in_normal_mode_clears_a_single_line_field() {
    let mut input = TextInput {
        insert: false,
        cursor: 3,
        ..TextInput::default()
    };
    let mut buf = "delete me".to_string();

    input.handle_key(&mut buf, KeyCode::Char('d'), false, false);
    assert!(input.pending_d);
    input.handle_key(&mut buf, KeyCode::Char('d'), false, false);
    assert!(!input.pending_d);
    assert_eq!(buf, "");
}

#[test]
fn focus_resets_to_insert_at_the_end() {
    let mut input = TextInput {
        insert: false,
        cursor: 0,
        ..TextInput::default()
    };
    input.focus("abc");
    assert!(input.insert);
    assert_eq!(input.cursor, 3);
}

#[test]
fn visual_mode_delete_removes_selection_and_exits_visual() {
    let mut input = TextInput {
        insert: false,
        cursor: 0,
        ..TextInput::default()
    };
    let mut buf = "hello world".to_string();

    input.handle_key(&mut buf, KeyCode::Char('v'), false, false);
    assert!(input.visual_anchor.is_some());

    for _ in 0..4 {
        input.handle_key(&mut buf, KeyCode::Char('l'), false, false);
    }
    input.handle_key(&mut buf, KeyCode::Char('d'), false, false);

    assert_eq!(buf, " world");
    assert!(input.visual_anchor.is_none());
    assert_eq!(input.clipboard, "hello");
}

#[test]
fn visual_mode_yank_then_normal_mode_paste() {
    let mut input = TextInput {
        insert: false,
        cursor: 6,
        ..TextInput::default()
    };
    let mut buf = "hello world".to_string();

    input.handle_key(&mut buf, KeyCode::Char('v'), false, false);
    for _ in 0..4 {
        input.handle_key(&mut buf, KeyCode::Char('l'), false, false);
    }
    input.handle_key(&mut buf, KeyCode::Char('y'), false, false);
    assert_eq!(input.clipboard, "world");
    assert!(input.visual_anchor.is_none());
    assert_eq!(buf, "hello world");

    input.cursor = 4;
    input.handle_key(&mut buf, KeyCode::Char('p'), false, false);
    assert_eq!(buf, "helloworld world");
}

#[test]
fn visual_mode_change_deletes_and_enters_insert() {
    let mut input = TextInput {
        insert: false,
        cursor: 1,
        ..TextInput::default()
    };
    let mut buf = "abcdef".to_string();

    input.handle_key(&mut buf, KeyCode::Char('v'), false, false);
    input.handle_key(&mut buf, KeyCode::Char('l'), false, false);
    input.handle_key(&mut buf, KeyCode::Char('c'), false, false);

    assert_eq!(buf, "adef");
    assert!(input.insert);
    assert!(input.visual_anchor.is_none());
}

#[test]
fn esc_exits_visual_mode_without_editing_before_closing() {
    let mut input = TextInput {
        insert: false,
        cursor: 0,
        ..TextInput::default()
    };
    let mut buf = "keep me".to_string();
    input.handle_key(&mut buf, KeyCode::Char('v'), false, false);

    assert!(input.handle_esc(&mut buf));
    assert!(input.visual_anchor.is_none());
    assert_eq!(buf, "keep me");

    assert!(!input.handle_esc(&mut buf));
}
