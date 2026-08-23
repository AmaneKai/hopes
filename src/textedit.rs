//! A small vim-flavored line/paragraph editor operating on a `String` plus a
//! char-index cursor. Word motions treat text as two classes (whitespace vs.
//! non-whitespace) rather than vim's full word/punctuation/whitespace split —
//! deliberately simplified for short task-form fields rather than code.
//! "Line" here means a run of characters between `\n` boundaries (or buffer
//! start/end); single-line fields (title, tags, filter, command) just have
//! exactly one line, so all of this degrades correctly for them too.

fn to_chars(s: &str) -> Vec<char> {
    s.chars().collect()
}

fn from_chars(chars: &[char]) -> String {
    chars.iter().collect()
}

fn is_word(c: char) -> bool {
    !c.is_whitespace()
}

fn line_bounds(chars: &[char], cursor: usize) -> (usize, usize) {
    let cursor = cursor.min(chars.len());
    let start = chars[..cursor]
        .iter()
        .rposition(|&c| c == '\n')
        .map(|i| i + 1)
        .unwrap_or(0);
    let end = chars[cursor..]
        .iter()
        .position(|&c| c == '\n')
        .map(|i| cursor + i)
        .unwrap_or(chars.len());
    (start, end)
}

#[inline(always)]
fn last_pos_of(start: usize, end: usize) -> usize {
    if end > start { end - 1 } else { start }
}

pub fn char_len(s: &str) -> usize {
    s.chars().count()
}

/// Clamps a cursor to a valid "resting on a character" position for Normal
/// mode (never past the last char of its line, matching vim).
pub fn clamp_normal(s: &str, cursor: usize) -> usize {
    let chars = to_chars(s);
    let n = chars.len();
    if n == 0 {
        return 0;
    }
    let cursor = cursor.min(n - 1);
    let (start, end) = line_bounds(&chars, cursor);
    cursor.clamp(start, last_pos_of(start, end))
}

pub fn move_left(s: &str, cursor: usize) -> usize {
    let chars = to_chars(s);
    let n = chars.len();
    if n == 0 {
        return 0;
    }
    let cursor = cursor.min(n - 1);
    let (start, _) = line_bounds(&chars, cursor);
    cursor.saturating_sub(1).max(start)
}

pub fn move_right(s: &str, cursor: usize) -> usize {
    let chars = to_chars(s);
    let n = chars.len();
    if n == 0 {
        return 0;
    }
    let cursor = cursor.min(n - 1);
    let (start, end) = line_bounds(&chars, cursor);
    (cursor + 1).min(last_pos_of(start, end))
}

pub fn move_up(s: &str, cursor: usize) -> usize {
    let chars = to_chars(s);
    let n = chars.len();
    if n == 0 {
        return 0;
    }
    let cursor = cursor.min(n - 1);
    let (start, _) = line_bounds(&chars, cursor);
    if start == 0 {
        return cursor;
    }
    let col = cursor - start;
    let prev_end = start - 1;
    let prev_start = chars[..prev_end]
        .iter()
        .rposition(|&c| c == '\n')
        .map(|i| i + 1)
        .unwrap_or(0);
    (prev_start + col).min(last_pos_of(prev_start, prev_end))
}

pub fn move_down(s: &str, cursor: usize) -> usize {
    let chars = to_chars(s);
    let n = chars.len();
    if n == 0 {
        return 0;
    }
    let cursor = cursor.min(n - 1);
    let (start, end) = line_bounds(&chars, cursor);
    if end >= n {
        return cursor;
    }
    let col = cursor - start;
    let next_start = end + 1;
    let next_end = chars[next_start..]
        .iter()
        .position(|&c| c == '\n')
        .map(|i| next_start + i)
        .unwrap_or(n);
    (next_start + col).min(last_pos_of(next_start, next_end))
}

// -- Caret (Insert-mode) movement -------------------------------------------
//
// The functions above implement vim Normal-mode semantics, where the cursor
// always rests *on* a character (max position is `len - 1`). Insert mode
// needs a plain text-cursor instead — a caret that can sit *after* the last
// character of a line (max position is `len`), same as any ordinary text
// box. Reusing the Normal-mode functions for Insert-mode arrow keys was a
// real bug: a cursor at end-of-buffer got silently dragged back one
// character before every move even started.

pub fn caret_left(s: &str, cursor: usize) -> usize {
    let chars = to_chars(s);
    let cursor = cursor.min(chars.len());
    let (start, _) = line_bounds(&chars, cursor);
    cursor.saturating_sub(1).max(start)
}

pub fn caret_right(s: &str, cursor: usize) -> usize {
    let chars = to_chars(s);
    let cursor = cursor.min(chars.len());
    let (_, end) = line_bounds(&chars, cursor);
    (cursor + 1).min(end)
}

pub fn caret_up(s: &str, cursor: usize) -> usize {
    let chars = to_chars(s);
    let n = chars.len();
    let cursor = cursor.min(n);
    let (start, _) = line_bounds(&chars, cursor);
    if start == 0 {
        return cursor;
    }
    let col = cursor - start;
    let prev_end = start - 1;
    let prev_start = chars[..prev_end]
        .iter()
        .rposition(|&c| c == '\n')
        .map(|i| i + 1)
        .unwrap_or(0);
    (prev_start + col).min(prev_end)
}

pub fn caret_down(s: &str, cursor: usize) -> usize {
    let chars = to_chars(s);
    let n = chars.len();
    let cursor = cursor.min(n);
    let (start, end) = line_bounds(&chars, cursor);
    if end >= n {
        return cursor;
    }
    let col = cursor - start;
    let next_start = end + 1;
    let next_end = chars[next_start..]
        .iter()
        .position(|&c| c == '\n')
        .map(|i| next_start + i)
        .unwrap_or(n);
    (next_start + col).min(next_end)
}

pub fn line_start(s: &str, cursor: usize) -> usize {
    let chars = to_chars(s);
    let n = chars.len();
    if n == 0 {
        return 0;
    }
    line_bounds(&chars, cursor.min(n - 1)).0
}

pub fn line_end(s: &str, cursor: usize) -> usize {
    let chars = to_chars(s);
    let n = chars.len();
    if n == 0 {
        return 0;
    }
    let (start, end) = line_bounds(&chars, cursor.min(n - 1));
    last_pos_of(start, end)
}

pub fn word_forward(s: &str, cursor: usize) -> usize {
    let chars = to_chars(s);
    let n = chars.len();
    if n == 0 {
        return 0;
    }
    let mut i = cursor.min(n - 1);
    if is_word(chars[i]) {
        while i < n && is_word(chars[i]) {
            i += 1;
        }
    }
    while i < n && !is_word(chars[i]) {
        i += 1;
    }
    if i >= n { n - 1 } else { i }
}

pub fn word_end(s: &str, cursor: usize) -> usize {
    let chars = to_chars(s);
    let n = chars.len();
    if n == 0 {
        return 0;
    }
    let mut i = cursor.min(n - 1) + 1;
    while i < n && !is_word(chars[i]) {
        i += 1;
    }
    while i + 1 < n && is_word(chars[i + 1]) {
        i += 1;
    }
    i.min(n - 1)
}

pub fn word_back(s: &str, cursor: usize) -> usize {
    let chars = to_chars(s);
    let n = chars.len();
    if n == 0 {
        return 0;
    }
    let mut i = cursor.min(n - 1);
    if i == 0 {
        return 0;
    }
    i -= 1;
    while i > 0 && !is_word(chars[i]) {
        i -= 1;
    }
    while i > 0 && is_word(chars[i - 1]) {
        i -= 1;
    }
    i
}

pub fn insert_char(s: &mut String, cursor: usize, c: char) -> usize {
    let mut chars = to_chars(s);
    let idx = cursor.min(chars.len());
    chars.insert(idx, c);
    *s = from_chars(&chars);
    idx + 1
}

pub fn backspace(s: &mut String, cursor: usize) -> usize {
    if cursor == 0 {
        return 0;
    }
    let mut chars = to_chars(s);
    let idx = cursor.min(chars.len());
    if idx == 0 {
        return 0;
    }
    chars.remove(idx - 1);
    *s = from_chars(&chars);
    idx - 1
}

/// vim `x`
pub fn delete_char(s: &mut String, cursor: usize) -> usize {
    let mut chars = to_chars(s);
    let n = chars.len();
    if n == 0 {
        return 0;
    }
    let cursor = cursor.min(n - 1);
    let (start, end) = line_bounds(&chars, cursor);
    if end > start {
        chars.remove(cursor);
        *s = from_chars(&chars);
    }
    clamp_normal(s, cursor)
}

/// vim `D`
pub fn delete_to_line_end(s: &mut String, cursor: usize) -> usize {
    let mut chars = to_chars(s);
    let n = chars.len();
    let cursor = cursor.min(n);
    let (start, end) = line_bounds(&chars, cursor);
    if end > cursor {
        chars.drain(cursor..end);
        *s = from_chars(&chars);
    }
    clamp_normal(s, cursor.max(start).min(char_len(s)))
}

/// vim `dd`
pub fn delete_line(s: &mut String, cursor: usize) -> usize {
    let mut chars = to_chars(s);
    let n = chars.len();
    if n == 0 {
        return 0;
    }
    let cursor = cursor.min(n - 1);
    let (start, end) = line_bounds(&chars, cursor);
    let (remove_start, remove_end) = if end < n {
        (start, end + 1)
    } else if start > 0 {
        (start - 1, end)
    } else {
        (start, end)
    };
    chars.drain(remove_start..remove_end);
    *s = from_chars(&chars);
    clamp_normal(s, remove_start)
}

/// vim `i`
pub fn enter_insert_before(s: &str, cursor: usize) -> usize {
    clamp_normal(s, cursor)
}

/// vim `a`
pub fn enter_insert_after(s: &str, cursor: usize) -> usize {
    let chars = to_chars(s);
    let n = chars.len();
    if n == 0 {
        return 0;
    }
    let cursor = cursor.min(n - 1);
    let (start, end) = line_bounds(&chars, cursor);
    (cursor + 1).min(end).max(start)
}

/// vim `I`
pub fn enter_insert_line_start(s: &str, cursor: usize) -> usize {
    line_start(s, cursor)
}

/// vim `A`
pub fn enter_insert_line_end(s: &str, cursor: usize) -> usize {
    let chars = to_chars(s);
    let n = chars.len();
    if n == 0 {
        return 0;
    }
    line_bounds(&chars, cursor.min(n - 1)).1
}

/// vim `o`. When `allow_newline` is false (single-line fields), falls back to
/// jumping to end-of-line in insert mode instead of inserting a literal `\n`.
pub fn open_below(s: &mut String, cursor: usize, allow_newline: bool) -> usize {
    if !allow_newline {
        return enter_insert_line_end(s, cursor);
    }
    let mut chars = to_chars(s);
    let n = chars.len();
    let (_, end) = line_bounds(&chars, cursor.min(n.saturating_sub(1)));
    chars.insert(end, '\n');
    *s = from_chars(&chars);
    end + 1
}

/// vim `O`. See `open_below` for `allow_newline`.
pub fn open_above(s: &mut String, cursor: usize, allow_newline: bool) -> usize {
    if !allow_newline {
        return enter_insert_line_start(s, cursor);
    }
    let mut chars = to_chars(s);
    let n = chars.len();
    let (start, _) = line_bounds(&chars, cursor.min(n.saturating_sub(1)));
    chars.insert(start, '\n');
    *s = from_chars(&chars);
    start
}

// -- Visual-mode selection ----------------------------------------------
//
// Char-wise only (no vim linewise `V` / blockwise `<C-v>`). A selection is
// just an (anchor, cursor) pair the caller already tracks; these helpers
// take the pair and normalize it into an inclusive [lo, hi] range.

#[inline(always)]
pub fn selection_range(anchor: usize, cursor: usize) -> (usize, usize) {
    (anchor.min(cursor), anchor.max(cursor))
}

/// vim `y` over a selection
pub fn yank_range(s: &str, lo: usize, hi: usize) -> String {
    let chars = to_chars(s);
    let n = chars.len();
    if n == 0 || lo >= n {
        return String::new();
    }
    let hi = hi.min(n - 1);
    chars[lo..=hi].iter().collect()
}

/// vim `d`/`x`/`c` over a selection
pub fn delete_range(s: &mut String, lo: usize, hi: usize) -> usize {
    let mut chars = to_chars(s);
    let n = chars.len();
    if n == 0 || lo >= n {
        return clamp_normal(s, lo);
    }
    let hi = hi.min(n - 1);
    chars.drain(lo..=hi);
    *s = from_chars(&chars);
    clamp_normal(s, lo)
}

/// vim `p`
pub fn paste_after(s: &mut String, cursor: usize, text: &str) -> usize {
    if text.is_empty() {
        return cursor;
    }
    let mut chars = to_chars(s);
    let insert_at = (cursor + 1).min(chars.len());
    let pasted: Vec<char> = text.chars().collect();
    let pasted_len = pasted.len();
    chars.splice(insert_at..insert_at, pasted);
    *s = from_chars(&chars);
    clamp_normal(s, insert_at + pasted_len.saturating_sub(1))
}

/// vim `P`
pub fn paste_before(s: &mut String, cursor: usize, text: &str) -> usize {
    if text.is_empty() {
        return cursor;
    }
    let mut chars = to_chars(s);
    let insert_at = cursor.min(chars.len());
    let pasted: Vec<char> = text.chars().collect();
    let pasted_len = pasted.len();
    chars.splice(insert_at..insert_at, pasted);
    *s = from_chars(&chars);
    clamp_normal(s, insert_at + pasted_len.saturating_sub(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hl_stay_within_line() {
        let s = "abc\ndef";
        assert_eq!(move_left("abc\ndef", 0), 0);
        assert_eq!(move_right(s, 2), 2); // 'c' is last char of line 1
        assert_eq!(move_right(s, 3.min(char_len(s))), 2);
    }

    #[test]
    fn caret_left_right_allow_resting_past_the_last_char() {
        // This is exactly the Insert-mode bug this module used to have:
        // reusing Normal-mode move_left/move_right (which clamp to len-1)
        // silently ate a character every time you arrowed left from the end
        // of a line you'd just finished typing.
        let s = "helloworld"; // len 10, valid caret positions are 0..=10
        assert_eq!(caret_left(s, 10), 9);
        assert_eq!(caret_right(s, 10), 10); // already at end, no-op
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
        assert_eq!(caret_down(s, 2), 5); // end of "ab" -> end of "cd" (both len 2)
        assert_eq!(caret_up(s, 5), 2);
    }

    #[test]
    fn zero_and_dollar() {
        let s = "hello world\nsecond";
        assert_eq!(line_start(s, 8), 0);
        assert_eq!(line_end(s, 2), 10); // 'd' of "world"
        assert_eq!(line_start(s, 14), 12);
        assert_eq!(line_end(s, 14), 17); // 'd' of "second"
    }

    #[test]
    fn word_motions() {
        let s = "foo   bar baz";
        assert_eq!(word_forward(s, 0), 6); // start of "bar"
        assert_eq!(word_forward(s, 6), 10); // start of "baz"
        assert_eq!(word_end(s, 0), 2); // end of "foo"
        assert_eq!(word_end(s, 2), 8); // 'o' -> end of "bar"
        assert_eq!(word_back(s, 10), 6); // start of "bar"
        assert_eq!(word_back(s, 6), 0); // start of "foo"
    }

    #[test]
    fn up_down_clamp_into_short_lines() {
        // No persistent "desired column" memory (unlike real vim) — once a
        // move clamps into a short line, subsequent moves carry the clamped
        // column forward rather than the original one.
        let s = "abcdef\nxy\nqrstuv";
        let down = move_down(s, 4); // col 4 on "abcdef" -> clamped into "xy"
        assert_eq!(down, 8); // 'y', last char of "xy"
        let down2 = move_down(s, down); // col 1 carried into "qrstuv"
        assert_eq!(down2, 11); // 'r'
        let up = move_up(s, down2); // col 1 carried back into "xy"
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
        let cursor = delete_line(&mut s, 5); // inside "two"
        assert_eq!(s, "one\nthree");
        assert_eq!(cursor, 4); // start of "three"
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
        let cursor = delete_to_line_end(&mut s, 4); // after "keep"
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
        // anchor before cursor
        assert_eq!(selection_range(2, 6), (2, 6));
        // anchor after cursor (selected backward) normalizes the same way
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
        let clip = yank_range(&s, 6, 10); // "world"
        assert_eq!(clip, "world");

        let cursor = paste_after(&mut s, 4, &clip); // after the 'o' of hello
        assert_eq!(s, "helloworld world");
        assert_eq!(cursor, 9); // resting on last pasted char

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
}
