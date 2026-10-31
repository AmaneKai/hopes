fn to_chars(s: &str) -> Vec<char> {
    s.chars().collect()
}

fn from_chars(chars: &[char]) -> String {
    chars.iter().collect()
}

fn is_word(c: char) -> bool {
    !c.is_whitespace()
}

fn line_start_before(chars: &[char], end: usize) -> usize {
    for i in (0..end).rev() {
        if chars[i] == '\n' {
            return i + 1;
        }
    }
    0
}

fn line_end_from(chars: &[char], from: usize) -> usize {
    for (i, &c) in chars.iter().enumerate().skip(from) {
        if c == '\n' {
            return i;
        }
    }
    chars.len()
}

fn line_bounds(chars: &[char], cursor: usize) -> (usize, usize) {
    let cursor = cursor.min(chars.len());
    let start = line_start_before(chars, cursor);
    let end = line_end_from(chars, cursor);
    (start, end)
}

#[inline(always)]
fn last_pos_of(start: usize, end: usize) -> usize {
    if end > start { end - 1 } else { start }
}

pub fn char_len(s: &str) -> usize {
    s.chars().count()
}

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
    let prev_start = line_start_before(&chars, prev_end);
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
    let next_end = line_end_from(&chars, next_start);
    (next_start + col).min(last_pos_of(next_start, next_end))
}

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
    let prev_start = line_start_before(&chars, prev_end);
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
    let next_end = line_end_from(&chars, next_start);
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

pub fn enter_insert_before(s: &str, cursor: usize) -> usize {
    clamp_normal(s, cursor)
}

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

pub fn enter_insert_line_start(s: &str, cursor: usize) -> usize {
    line_start(s, cursor)
}

pub fn enter_insert_line_end(s: &str, cursor: usize) -> usize {
    let chars = to_chars(s);
    let n = chars.len();
    if n == 0 {
        return 0;
    }
    line_bounds(&chars, cursor.min(n - 1)).1
}

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

#[inline(always)]
pub fn selection_range(anchor: usize, cursor: usize) -> (usize, usize) {
    (anchor.min(cursor), anchor.max(cursor))
}

pub fn yank_range(s: &str, lo: usize, hi: usize) -> String {
    let chars = to_chars(s);
    let n = chars.len();
    if n == 0 || lo >= n {
        return String::new();
    }
    let hi = hi.min(n - 1);
    chars[lo..=hi].iter().collect()
}

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
