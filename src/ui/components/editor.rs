use crate::{
    action::{Action, TaskDraft},
    domain::{
        AppState, Priority, Status,
        due::{self, DueUrgency, InvalidDate, full_label, urgency},
    },
    ui::{
        Component, KeyResult,
        field::{field_or_placeholder, row_col_of_cursor},
        popup::popup_frame,
        text_input::TextInput,
        theme::Theme,
    },
    utils::textedit,
};
use chrono::{Local, NaiveDate};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

const LABEL_WIDTH: u16 = 10;
const NOTES_ROWS: u16 = 3;
const FORM_HEIGHT: u16 = 16;
/// Inner height below which blank spacers and rules are dropped so every row stays visible.
const ROOMY_INNER: u16 = FORM_HEIGHT - 2;
const NARROW_INNER: u16 = 48;

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum FormField {
    Title,
    Description,
    Priority,
    Due,
    Tags,
}

impl FormField {
    #[inline(always)]
    pub fn next(&self) -> Self {
        match self {
            Self::Title => Self::Description,
            Self::Description => Self::Priority,
            Self::Priority => Self::Due,
            Self::Due => Self::Tags,
            Self::Tags => Self::Title,
        }
    }

    #[inline(always)]
    pub fn prev(&self) -> Self {
        match self {
            Self::Title => Self::Tags,
            Self::Description => Self::Title,
            Self::Priority => Self::Description,
            Self::Due => Self::Priority,
            Self::Tags => Self::Due,
        }
    }
}

pub struct EditorComponent {
    title: String,
    description: String,
    tags: String,
    due: String,
    priority: Priority,
    field: FormField,
    tag_sel: Option<usize>,
    edit_idx: Option<usize>,
    create_status: Status,
    input: TextInput,
}

impl Default for EditorComponent {
    fn default() -> Self {
        Self {
            title: String::new(),
            description: String::new(),
            tags: String::new(),
            due: String::new(),
            priority: Priority::Medium,
            field: FormField::Title,
            tag_sel: None,
            edit_idx: None,
            create_status: Status::Todo,
            input: TextInput::default(),
        }
    }
}

impl EditorComponent {
    pub fn text_input(&self) -> Option<&TextInput> {
        (self.field != FormField::Priority).then_some(&self.input)
    }

    fn open_create(&mut self, status: Status) {
        self.title.clear();
        self.description.clear();
        self.tags.clear();
        self.due.clear();
        self.priority = Priority::Medium;
        self.field = FormField::Title;
        self.edit_idx = None;
        self.create_status = status;
        self.focus_field();
    }

    fn open_edit(&mut self, idx: usize, state: &AppState) {
        let Some(item) = state.items.get(idx) else {
            return;
        };
        self.title = item.title.clone();
        self.description = item.description.clone();
        self.priority = item.priority;
        self.tags = item.tags.join(", ");
        self.due = item.due.map(|d| d.to_string()).unwrap_or_default();
        self.field = FormField::Title;
        self.edit_idx = Some(idx);
        self.focus_field();
    }

    fn input_and_buf(&mut self) -> (&mut TextInput, Option<&mut String>) {
        let buf = match self.field {
            FormField::Title => Some(&mut self.title),
            FormField::Description => Some(&mut self.description),
            FormField::Tags => Some(&mut self.tags),
            FormField::Due => Some(&mut self.due),
            FormField::Priority => None,
        };
        (&mut self.input, buf)
    }

    fn focus_field(&mut self) {
        self.tag_sel = None;
        let (input, buf) = self.input_and_buf();
        input.focus(buf.map(|b| b.as_str()).unwrap_or(""));
    }

    /// Existing tags matching the token being typed, most used first.
    /// Empty unless the caret sits at the end of an insert-mode Tags field.
    fn suggestions(&self, state: &AppState) -> Vec<String> {
        if self.field != FormField::Tags
            || !self.input.insert
            || self.input.cursor != textedit::char_len(&self.tags)
        {
            return Vec::new();
        }
        let (head, token) = split_tags(&self.tags);
        let token = token.to_lowercase();
        let used: Vec<String> = head
            .split(',')
            .map(|t| t.trim().to_lowercase())
            .filter(|t| !t.is_empty())
            .collect();
        let candidates = state.tags().iter().map(|(t, _)| t).filter(|t| {
            let lower = t.to_lowercase();
            lower != token && !used.contains(&lower)
        });
        let (mut hits, rest): (Vec<_>, Vec<_>) =
            candidates.partition(|t| t.to_lowercase().starts_with(&token));
        hits.extend(
            rest.into_iter()
                .filter(|t| t.to_lowercase().contains(&token)),
        );
        hits.into_iter().cloned().collect()
    }

    /// Index Tab or Enter would complete: the picked chip, else the best match for typed text.
    fn tag_target(&self) -> Option<usize> {
        self.tag_sel
            .or_else(|| (!split_tags(&self.tags).1.is_empty()).then_some(0))
    }

    fn accept_tag(&mut self, tag: &str) {
        let mut text = match self.tags.rfind(',') {
            Some(i) => format!("{} ", &self.tags[..=i]),
            None => String::new(),
        };
        text.push_str(tag);
        text.push_str(", ");
        self.input.cursor = textedit::char_len(&text);
        self.tags = text;
        self.tag_sel = None;
    }

    fn handle_priority_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char(' ') | KeyCode::Char('l') | KeyCode::Char('p') | KeyCode::Right => {
                self.priority = self.priority.next();
            }
            KeyCode::Char('h') | KeyCode::Char('P') | KeyCode::Left => {
                self.priority = self.priority.prev();
            }
            KeyCode::Char(c @ '1'..='4') => {
                self.priority = Priority::from_digit(c).unwrap_or(self.priority);
            }
            _ => {}
        }
    }

    /// `Err` when the typed due text is not a date; `Ok(None)` when it is empty.
    fn parsed_due(&self) -> Result<Option<NaiveDate>, InvalidDate> {
        due::parse(&self.due, Local::now().date_naive())
    }

    fn save_action(&self) -> Option<Action> {
        if self.title.trim().is_empty() {
            return None;
        }
        let due = self.parsed_due().ok()?;
        let tags = self
            .tags
            .split(',')
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
            .collect();
        Some(Action::SaveTask(TaskDraft {
            id: self.edit_idx,
            title: self.title.trim().to_string(),
            description: self.description.trim().to_string(),
            priority: self.priority,
            tags,
            due,
            status: self.create_status,
        }))
    }
}

impl Component for EditorComponent {
    fn handle_key(&mut self, key: KeyEvent, state: &AppState) -> KeyResult {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let code = key.code;

        if ctrl && code == KeyCode::Char('c') {
            return KeyResult::Emit(Action::CloseModal);
        }

        if code == KeyCode::Esc {
            let field = self.field;
            let (input, buf) = self.input_and_buf();
            let consumed = match buf {
                Some(buf) if field != FormField::Priority => input.handle_esc(buf),
                _ => false,
            };
            return if consumed {
                KeyResult::Handled
            } else {
                KeyResult::Emit(Action::CloseModal)
            };
        }

        let suggestions = self.suggestions(state);
        if !suggestions.is_empty() {
            let n = suggestions.len();
            match code {
                KeyCode::Down => {
                    self.tag_sel = Some(self.tag_sel.map_or(0, |i| (i + 1) % n));
                    return KeyResult::Handled;
                }
                KeyCode::Up => {
                    self.tag_sel = Some(self.tag_sel.map_or(n - 1, |i| (i + n - 1) % n));
                    return KeyResult::Handled;
                }
                KeyCode::Tab | KeyCode::Enter => {
                    if let Some(tag) = self.tag_target().and_then(|i| suggestions.get(i)) {
                        self.accept_tag(tag);
                        return KeyResult::Handled;
                    }
                }
                _ => {}
            }
        }

        if (code == KeyCode::Enter && self.field != FormField::Description)
            || (ctrl && code == KeyCode::Char('s'))
        {
            return match self.save_action() {
                Some(save) => KeyResult::Emit(save),
                None => KeyResult::Handled,
            };
        }

        if (ctrl && (code == KeyCode::Char('j') || code == KeyCode::Char('n')))
            || code == KeyCode::Tab
            || (code == KeyCode::Down && self.field != FormField::Description)
        {
            self.field = self.field.next();
            self.focus_field();
            return KeyResult::Handled;
        }
        if (ctrl && (code == KeyCode::Char('k') || code == KeyCode::Char('p')))
            || code == KeyCode::BackTab
            || (code == KeyCode::Up && self.field != FormField::Description)
        {
            self.field = self.field.prev();
            self.focus_field();
            return KeyResult::Handled;
        }

        match self.field {
            FormField::Title | FormField::Description | FormField::Due | FormField::Tags => {
                let allow_newline = self.field == FormField::Description;
                self.tag_sel = None;
                let (input, buf) = self.input_and_buf();
                if let Some(buf) = buf {
                    input.handle_key(buf, code, ctrl, allow_newline);
                }
            }
            FormField::Priority => self.handle_priority_key(code),
        }
        KeyResult::Handled
    }

    fn update(&mut self, action: &Action, state: &AppState) {
        match action {
            Action::OpenCreateModal { status } => self.open_create(*status),
            Action::OpenEditModal(idx) => self.open_edit(*idx, state),
            _ => {}
        }
    }

    fn render(&mut self, f: &mut Frame, area: Rect, state: &AppState, _focused: bool) {
        let title = if self.edit_idx.is_some() {
            "Edit task"
        } else {
            "New task"
        };
        let inner = popup_frame(f, area, title, 68, FORM_HEIGHT);

        let roomy = inner.height >= ROOMY_INNER;
        let gap = u16::from(roomy);
        let notes_rows = if roomy { NOTES_ROWS } else { 1 };

        let [
            _,
            title_row,
            title_rule,
            notes_row,
            notes_rule,
            _,
            prio_row,
            due_row,
            tags_row,
            chips_row,
            _,
            hint_row,
        ] = Layout::vertical([
            Constraint::Length(gap),
            Constraint::Length(1),
            Constraint::Length(gap),
            Constraint::Length(notes_rows),
            Constraint::Length(gap),
            Constraint::Length(gap),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .areas(inner);

        let title_focused = self.field == FormField::Title;
        let notes_focused = self.field == FormField::Description;
        let prio_focused = self.field == FormField::Priority;
        let due_focused = self.field == FormField::Due;
        let tags_focused = self.field == FormField::Tags;
        let cursor = self.input.cursor;
        let anchor = self.input.visual_anchor;

        let (g, l, v) = split_row(title_row);
        f.render_widget(gutter(title_focused), g);
        f.render_widget(label("Title", title_focused), l);
        f.render_widget(
            Paragraph::new(field_or_placeholder(
                &self.title,
                cursor,
                title_focused.then_some(anchor).flatten(),
                "Untitled",
            )),
            v,
        );
        underline(f, title_rule, title_focused);
        if title_focused {
            let x = v.x + (cursor as u16).min(v.width.saturating_sub(1));
            f.set_cursor_position((x, v.y));
        }

        let (g, l, v) = split_row(notes_row);
        f.render_widget(gutter(notes_focused), g);
        f.render_widget(label("Notes", notes_focused), l);
        f.render_widget(
            Paragraph::new(field_or_placeholder(
                &self.description,
                cursor,
                notes_focused.then_some(anchor).flatten(),
                "Add notes...",
            )),
            v,
        );
        underline(f, notes_rule, notes_focused);
        if notes_focused {
            let (row, col) = row_col_of_cursor(&self.description, cursor);
            let x = v.x + col.min(v.width.saturating_sub(1));
            let y = v.y + row.min(v.height.saturating_sub(1));
            f.set_cursor_position((x, y));
        }

        let (g, l, v) = split_row(prio_row);
        f.render_widget(gutter(prio_focused), g);
        f.render_widget(label("Priority", prio_focused), l);
        let mut spans = Vec::with_capacity(Priority::ALL.len() * 2);
        for p in Priority::ALL {
            let mut style = Style::default().fg(p.color());
            if p == self.priority {
                style = style.add_modifier(Modifier::BOLD | Modifier::UNDERLINED);
            }
            spans.push(Span::styled(format!("{} {}", p.glyph(), p.label()), style));
            spans.push(Span::raw("   "));
        }
        f.render_widget(Paragraph::new(Line::from(spans)), v);

        let (g, l, v) = split_row(due_row);
        f.render_widget(gutter(due_focused), g);
        f.render_widget(label("Due", due_focused), l);
        let hint_style = Theme::muted();
        let (preview, preview_style) = match self.parsed_due() {
            Ok(Some(date)) => {
                let today = Local::now().date_naive();
                let style = match urgency(date, today) {
                    DueUrgency::Later => hint_style,
                    other => Theme::due(other),
                };
                (format!("\u{2192} {}", full_label(date)), style)
            }
            Ok(None) => (String::new(), hint_style),
            Err(InvalidDate) => (
                "not a date (try fri, +3d, oct 25)".to_string(),
                Style::default().fg(Color::Red),
            ),
        };
        let preview_w = preview.chars().count() as u16;
        let [due_input, due_preview] =
            Layout::horizontal([Constraint::Min(1), Constraint::Length(preview_w + 1)]).areas(v);
        f.render_widget(
            Paragraph::new(field_or_placeholder(
                &self.due,
                cursor,
                due_focused.then_some(anchor).flatten(),
                "tomorrow, fri, +3d, oct 25",
            )),
            due_input,
        );
        f.render_widget(
            Paragraph::new(Span::styled(preview, preview_style))
                .alignment(ratatui::layout::Alignment::Right),
            due_preview,
        );
        if due_focused {
            let x = due_input.x + (cursor as u16).min(due_input.width.saturating_sub(1));
            f.set_cursor_position((x, due_input.y));
        }

        let (g, l, v) = split_row(tags_row);
        f.render_widget(gutter(tags_focused), g);
        f.render_widget(label("Tags", tags_focused), l);
        f.render_widget(
            Paragraph::new(field_or_placeholder(
                &self.tags,
                cursor,
                tags_focused.then_some(anchor).flatten(),
                "design, ui",
            )),
            v,
        );
        if tags_focused {
            let x = v.x + (cursor as u16).min(v.width.saturating_sub(1));
            f.set_cursor_position((x, v.y));
        }

        let suggestions = self.suggestions(state);
        if !suggestions.is_empty() {
            let (_, _, v) = split_row(chips_row);
            let target = self.tag_target();
            let mut spans = Vec::with_capacity(suggestions.len() * 2);
            let mut used = 0usize;
            for (i, tag) in suggestions.iter().enumerate() {
                let w = tag.chars().count() + 4;
                if i > 0 && used + w + 4 > v.width as usize {
                    spans.push(Span::styled(
                        format!("+{}", suggestions.len() - i),
                        Theme::muted(),
                    ));
                    break;
                }
                used += w;
                let style = if Some(i) == target {
                    Theme::tag().add_modifier(Modifier::REVERSED)
                } else {
                    Theme::tag()
                };
                spans.push(Span::styled(format!(" #{tag} "), style));
                spans.push(Span::raw(" "));
            }
            f.render_widget(Paragraph::new(Line::from(spans)), v);
        }

        let picking = !suggestions.is_empty();
        let hint = if inner.width < NARROW_INNER {
            Line::from(vec![
                Span::styled("[^s] ", Theme::shortcut_key()),
                Span::styled("save ", Theme::shortcut_desc()),
                Span::styled("[Tab] ", Theme::shortcut_key()),
                Span::styled(
                    if picking { "fill " } else { "next " },
                    Theme::shortcut_desc(),
                ),
                Span::styled("[Esc]", Theme::shortcut_key()),
            ])
        } else {
            Line::from(vec![
                Span::styled("[Ctrl-s] ", Theme::shortcut_key()),
                Span::styled("save   ", Theme::shortcut_desc()),
                Span::styled("[Tab] ", Theme::shortcut_key()),
                Span::styled(
                    if picking { "complete   " } else { "next   " },
                    Theme::shortcut_desc(),
                ),
                Span::styled(
                    if picking { "[\u{2191}\u{2193}] " } else { "" },
                    Theme::shortcut_key(),
                ),
                Span::styled(if picking { "pick   " } else { "" }, Theme::shortcut_desc()),
                Span::styled("[Esc] ", Theme::shortcut_key()),
                Span::styled("cancel", Theme::shortcut_desc()),
            ])
        };
        f.render_widget(Paragraph::new(hint), hint_row);
    }
}

fn gutter(focused: bool) -> Paragraph<'static> {
    let marker = if focused { "\u{258d} " } else { "  " };
    Paragraph::new(Span::styled(marker, Style::default().fg(Color::Cyan)))
}

fn label(name: &'static str, focused: bool) -> Paragraph<'static> {
    let style = if focused {
        Theme::text_bold()
    } else {
        Theme::muted()
    };
    Paragraph::new(Span::styled(name, style))
}

fn underline(f: &mut Frame, area: Rect, focused: bool) {
    let style = if focused {
        Theme::border_focused()
    } else {
        Theme::border()
    };
    f.render_widget(
        Paragraph::new("\u{2500}".repeat(area.width as usize)).style(style),
        area,
    );
}

fn split_row(row: Rect) -> (Rect, Rect, Rect) {
    let [g, l, v] = Layout::horizontal([
        Constraint::Length(2),
        Constraint::Length(LABEL_WIDTH),
        Constraint::Min(1),
    ])
    .areas(row);
    (g, l, v)
}

/// Splits the tags buffer into the finished part (through the last comma) and the token being typed.
fn split_tags(tags: &str) -> (&str, &str) {
    match tags.rfind(',') {
        Some(i) => (&tags[..i], tags[i + 1..].trim()),
        None => ("", tags.trim()),
    }
}
