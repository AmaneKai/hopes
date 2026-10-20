use crate::utils::textedit;
use crossterm::event::KeyCode;

pub struct TextInput {
    pub cursor: usize,
    pub insert: bool,
    pub pending_d: bool,
    pub visual_anchor: Option<usize>,
    pub clipboard: String,
}

impl Default for TextInput {
    fn default() -> Self {
        Self {
            cursor: 0,
            insert: true,
            pending_d: false,
            visual_anchor: None,
            clipboard: String::new(),
        }
    }
}

impl TextInput {
    pub fn focus(&mut self, buf: &str) {
        self.insert = true;
        self.pending_d = false;
        self.visual_anchor = None;
        self.cursor = textedit::char_len(buf);
    }

    #[inline(always)]
    fn edit(&mut self, buf: &mut String, f: impl FnOnce(&mut String, usize) -> usize) {
        self.cursor = f(buf, self.cursor);
    }

    pub fn handle_esc(&mut self, buf: &mut String) -> bool {
        if self.insert {
            self.insert = false;
            self.pending_d = false;
            self.edit(buf, |buf, cursor| textedit::clamp_normal(buf, cursor));
            return true;
        }
        self.visual_anchor.take().is_some()
    }

    pub fn handle_key(&mut self, buf: &mut String, code: KeyCode, ctrl: bool, allow_newline: bool) {
        if self.insert {
            self.handle_insert_key(buf, code, ctrl, allow_newline);
            return;
        }

        if self.pending_d {
            self.pending_d = false;
            if code == KeyCode::Char('d') {
                self.edit(buf, textedit::delete_line);
                return;
            }
        }

        if let Some(anchor) = self.visual_anchor {
            self.handle_visual_key(buf, code, anchor, allow_newline);
            return;
        }

        match code {
            KeyCode::Char('d') => self.pending_d = true,
            KeyCode::Char('v') => self.visual_anchor = Some(self.cursor),
            KeyCode::Char('p') => {
                let clip = self.clipboard.clone();
                self.edit(buf, move |buf, cursor| {
                    textedit::paste_after(buf, cursor, &clip)
                });
            }
            KeyCode::Char('P') => {
                let clip = self.clipboard.clone();
                self.edit(buf, move |buf, cursor| {
                    textedit::paste_before(buf, cursor, &clip)
                });
            }
            KeyCode::Char('h') | KeyCode::Left => {
                self.edit(buf, |buf, cursor| textedit::move_left(buf, cursor));
            }
            KeyCode::Char('l') | KeyCode::Right => {
                self.edit(buf, |buf, cursor| textedit::move_right(buf, cursor));
            }
            KeyCode::Char('j') | KeyCode::Down if allow_newline => {
                self.edit(buf, |buf, cursor| textedit::move_down(buf, cursor));
            }
            KeyCode::Char('k') | KeyCode::Up if allow_newline => {
                self.edit(buf, |buf, cursor| textedit::move_up(buf, cursor));
            }
            KeyCode::Char('0') => {
                self.edit(buf, |buf, cursor| textedit::line_start(buf, cursor));
            }
            KeyCode::Char('$') => {
                self.edit(buf, |buf, cursor| textedit::line_end(buf, cursor));
            }
            KeyCode::Char('w') => {
                self.edit(buf, |buf, cursor| textedit::word_forward(buf, cursor));
            }
            KeyCode::Char('e') => {
                self.edit(buf, |buf, cursor| textedit::word_end(buf, cursor));
            }
            KeyCode::Char('b') => {
                self.edit(buf, |buf, cursor| textedit::word_back(buf, cursor));
            }
            KeyCode::Char('x') => self.edit(buf, textedit::delete_char),
            KeyCode::Char('D') => self.edit(buf, textedit::delete_to_line_end),
            KeyCode::Char('i') => {
                self.edit(buf, |buf, cursor| {
                    textedit::enter_insert_before(buf, cursor)
                });
                self.insert = true;
            }
            KeyCode::Char('a') => {
                self.edit(buf, |buf, cursor| textedit::enter_insert_after(buf, cursor));
                self.insert = true;
            }
            KeyCode::Char('I') => {
                self.edit(buf, |buf, cursor| {
                    textedit::enter_insert_line_start(buf, cursor)
                });
                self.insert = true;
            }
            KeyCode::Char('A') => {
                self.edit(buf, |buf, cursor| {
                    textedit::enter_insert_line_end(buf, cursor)
                });
                self.insert = true;
            }
            KeyCode::Char('o') => {
                self.edit(buf, |buf, cursor| {
                    textedit::open_below(buf, cursor, allow_newline)
                });
                self.insert = true;
            }
            KeyCode::Char('O') => {
                self.edit(buf, |buf, cursor| {
                    textedit::open_above(buf, cursor, allow_newline)
                });
                self.insert = true;
            }
            _ => {}
        }
    }

    fn handle_insert_key(
        &mut self,
        buf: &mut String,
        code: KeyCode,
        ctrl: bool,
        allow_newline: bool,
    ) {
        match code {
            KeyCode::Char('w') if ctrl => {
                self.edit(buf, |buf, _| {
                    delete_word_back(buf);
                    textedit::char_len(buf)
                });
            }
            KeyCode::Char('u') if ctrl => {
                self.edit(buf, |buf, _| {
                    buf.clear();
                    0
                });
            }
            KeyCode::Enter if allow_newline => {
                self.edit(buf, |buf, cursor| textedit::insert_char(buf, cursor, '\n'));
            }
            KeyCode::Char(c) => {
                self.edit(buf, |buf, cursor| textedit::insert_char(buf, cursor, c));
            }
            KeyCode::Backspace => self.edit(buf, textedit::backspace),
            KeyCode::Left => {
                self.edit(buf, |buf, cursor| textedit::caret_left(buf, cursor));
            }
            KeyCode::Right => {
                self.edit(buf, |buf, cursor| textedit::caret_right(buf, cursor));
            }
            KeyCode::Up if allow_newline => {
                self.edit(buf, |buf, cursor| textedit::caret_up(buf, cursor));
            }
            KeyCode::Down if allow_newline => {
                self.edit(buf, |buf, cursor| textedit::caret_down(buf, cursor));
            }
            _ => {}
        }
    }

    fn handle_visual_key(
        &mut self,
        buf: &mut String,
        code: KeyCode,
        anchor: usize,
        allow_newline: bool,
    ) {
        match code {
            KeyCode::Char('v') => self.visual_anchor = None,
            KeyCode::Char('h') | KeyCode::Left => {
                self.edit(buf, |buf, cursor| textedit::move_left(buf, cursor));
            }
            KeyCode::Char('l') | KeyCode::Right => {
                self.edit(buf, |buf, cursor| textedit::move_right(buf, cursor));
            }
            KeyCode::Char('j') | KeyCode::Down if allow_newline => {
                self.edit(buf, |buf, cursor| textedit::move_down(buf, cursor));
            }
            KeyCode::Char('k') | KeyCode::Up if allow_newline => {
                self.edit(buf, |buf, cursor| textedit::move_up(buf, cursor));
            }
            KeyCode::Char('0') => {
                self.edit(buf, |buf, cursor| textedit::line_start(buf, cursor));
            }
            KeyCode::Char('$') => {
                self.edit(buf, |buf, cursor| textedit::line_end(buf, cursor));
            }
            KeyCode::Char('w') => {
                self.edit(buf, |buf, cursor| textedit::word_forward(buf, cursor));
            }
            KeyCode::Char('e') => {
                self.edit(buf, |buf, cursor| textedit::word_end(buf, cursor));
            }
            KeyCode::Char('b') => {
                self.edit(buf, |buf, cursor| textedit::word_back(buf, cursor));
            }
            KeyCode::Char('y') => {
                let (lo, hi) = textedit::selection_range(anchor, self.cursor);
                self.clipboard = textedit::yank_range(buf, lo, hi);
                self.cursor = lo;
                self.visual_anchor = None;
            }
            KeyCode::Char('d') | KeyCode::Char('x') | KeyCode::Char('c') => {
                let (lo, hi) = textedit::selection_range(anchor, self.cursor);
                self.clipboard = textedit::yank_range(buf, lo, hi);
                self.edit(buf, |buf, _| textedit::delete_range(buf, lo, hi));
                self.visual_anchor = None;
                self.insert = code == KeyCode::Char('c');
            }
            _ => {}
        }
    }
}

fn delete_word_back(buffer: &mut String) {
    while buffer.ends_with(' ') {
        buffer.pop();
    }
    while !buffer.is_empty() && !buffer.ends_with(' ') {
        buffer.pop();
    }
}
