use hopes::utils::textedit::*;

#[test]
fn hl_stay_within_line() {
    let s = "abc\ndef";
    assert_eq!(move_left("abc\ndef", 0), 0);
    assert_eq!(move_right(s, 2), 2);
    assert_eq!(move_right(s, 3.min(char_len(s))), 2);
}

#[test]
fn caret_left_right_allow_resting_past_the_last_char() {
    let s = "helloworld";
    assert_eq!(caret_left(s, 10), 9);
    assert_eq!(caret_right(s, 10), 10);
    assert_eq!(caret_left(s, 0), 0);

    let mut cursor = 10;
    for _ in 0..5 {
        cursor = caret_left(s, cursor);
    }
    assert_eq!(cursor, 5);
}

#[test]
fn caret_up_down_allow_resting_past_the_last_char_of_each_line() {
    let s = "ab\ncd";
    assert_eq!(caret_down(s, 2), 5);
    assert_eq!(caret_up(s, 5), 2);
}

#[test]
fn zero_and_dollar() {
    let s = "hello world\nsecond";
    assert_eq!(line_start(s, 8), 0);
    assert_eq!(line_end(s, 2), 10);
    assert_eq!(line_start(s, 14), 12);
    assert_eq!(line_end(s, 14), 17);
}

#[test]
fn word_motions() {
    let s = "foo   bar baz";
    assert_eq!(word_forward(s, 0), 6);
    assert_eq!(word_forward(s, 6), 10);
    assert_eq!(word_end(s, 0), 2);
    assert_eq!(word_end(s, 2), 8);
    assert_eq!(word_back(s, 10), 6);
    assert_eq!(word_back(s, 6), 0);
}

#[test]
fn up_down_clamp_into_short_lines() {
    let s = "abcdef\nxy\nqrstuv";
    let down = move_down(s, 4);
    assert_eq!(down, 8);
    let down2 = move_down(s, down);
    assert_eq!(down2, 11);
    let up = move_up(s, down2);
    assert_eq!(up, 8);
}

#[test]
fn x_deletes_under_cursor_and_clamps() {
    let mut s = "abc".to_string();
    let cursor = delete_char(&mut s, 2);
    assert_eq!(s, "ab");
    assert_eq!(cursor, 1);
}

#[test]
fn dd_removes_whole_line_including_newline() {
    let mut s = "one\ntwo\nthree".to_string();
    let cursor = delete_line(&mut s, 5);
    assert_eq!(s, "one\nthree");
    assert_eq!(cursor, 4);
}

#[test]
fn dd_on_only_line_clears_buffer() {
    let mut s = "solo".to_string();
    let cursor = delete_line(&mut s, 2);
    assert_eq!(s, "");
    assert_eq!(cursor, 0);
}

#[test]
fn capital_d_deletes_to_line_end_only() {
    let mut s = "keep this\nand this".to_string();
    let cursor = delete_to_line_end(&mut s, 4);
    assert_eq!(s, "keep\nand this");
    assert_eq!(cursor, 3);
}

#[test]
fn insert_and_backspace_are_cursor_aware() {
    let mut s = "helloworld".to_string();
    let cursor = insert_char(&mut s, 5, ' ');
    assert_eq!(s, "hello world");
    assert_eq!(cursor, 6);
    let cursor = backspace(&mut s, cursor);
    assert_eq!(s, "helloworld");
    assert_eq!(cursor, 5);
}

#[test]
fn open_below_without_newline_support_just_appends() {
    let mut s = "title".to_string();
    let cursor = open_below(&mut s, 0, false);
    assert_eq!(s, "title");
    assert_eq!(cursor, 5);
}

#[test]
fn open_below_with_newline_support_splits_line() {
    let mut s = "one".to_string();
    let cursor = open_below(&mut s, 1, true);
    assert_eq!(s, "one\n");
    assert_eq!(cursor, 4);
}

#[test]
fn insert_positions_ia_ie_a_shift_correctly() {
    let s = "hey\nyou";
    assert_eq!(enter_insert_before(s, 5), 5);
    assert_eq!(enter_insert_after(s, 1), 2);
    assert_eq!(enter_insert_line_start(s, 5), 4);
    assert_eq!(enter_insert_line_end(s, 5), 7);
}

#[test]
fn empty_buffer_is_safe() {
    let s = "";
    assert_eq!(clamp_normal(s, 0), 0);
    assert_eq!(move_left(s, 0), 0);
    assert_eq!(move_right(s, 0), 0);
    assert_eq!(word_forward(s, 0), 0);
    assert_eq!(line_end(s, 0), 0);
    let mut owned = String::new();
    assert_eq!(delete_char(&mut owned, 0), 0);
    assert_eq!(delete_line(&mut owned, 0), 0);
}

#[test]
fn visual_selection_is_inclusive_regardless_of_direction() {
    let s = "hello world";
    assert_eq!(selection_range(2, 6), (2, 6));
    assert_eq!(selection_range(6, 2), (2, 6));
    assert_eq!(yank_range(s, 0, 4), "hello");
    assert_eq!(yank_range(s, 6, 10), "world");
}

#[test]
fn delete_range_removes_inclusive_span_and_lands_cursor_at_lo() {
    let mut s = "hello world".to_string();
    let cursor = delete_range(&mut s, 0, 4);
    assert_eq!(s, " world");
    assert_eq!(cursor, 0);
}

#[test]
fn yank_then_paste_after_and_before() {
    let mut s = "hello world".to_string();
    let clip = yank_range(&s, 6, 10);
    assert_eq!(clip, "world");

    let cursor = paste_after(&mut s, 4, &clip);
    assert_eq!(s, "helloworld world");
    assert_eq!(cursor, 9);

    let mut s2 = "hi".to_string();
    let cursor2 = paste_before(&mut s2, 0, "XY");
    assert_eq!(s2, "XYhi");
    assert_eq!(cursor2, 1);
}

#[test]
fn paste_of_empty_clipboard_is_a_no_op() {
    let mut s = "abc".to_string();
    let cursor = paste_after(&mut s, 1, "");
    assert_eq!(s, "abc");
    assert_eq!(cursor, 1);
}
