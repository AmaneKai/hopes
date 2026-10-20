use crate::{
    action::{Action, TaskDraft},
    domain::{AppState, Priority, Status},
    ui::{
        Component, KeyResult,
        field::{field_or_placeholder, row_col_of_cursor},
        popup::popup_frame,
        text_input::TextInput,
        theme::Theme,
    },
};
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
const FORM_HEIGHT: u16 = 14;

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum FormField {
    Title,
    Description,
    Priority,
    Tags,
}

impl FormField {
    #[inline(always)]
    pub fn next(&self) -> Self {
        match self {
            Self::Title => Self::Description,
            Self::Description => Self::Priority,
            Self::Priority => Self::Tags,
            Self::Tags => Self::Title,
        }
    }

    #[inline(always)]
    pub fn prev(&self) -> Self {
        match self {
            Self::Title => Self::Tags,
            Self::Description => Self::Title,
            Self::Priority => Self::Description,
            Self::Tags => Self::Priority,
        }
    }
}

pub struct EditorComponent {
    title: String,
    description: String,
    tags: String,
    priority: Priority,
    field: FormField,
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
            priority: Priority::Medium,
            field: FormField::Title,
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
        self.field = FormField::Title;
        self.edit_idx = Some(idx);
        self.focus_field();
    }

    fn input_and_buf(&mut self) -> (&mut TextInput, Option<&mut String>) {
        let buf = match self.field {
            FormField::Title => Some(&mut self.title),
            FormField::Description => Some(&mut self.description),
            FormField::Tags => Some(&mut self.tags),
            FormField::Priority => None,
        };
        (&mut self.input, buf)
    }

    fn focus_field(&mut self) {
        let (input, buf) = self.input_and_buf();
        input.focus(buf.map(|b| b.as_str()).unwrap_or(""));
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

    fn save_action(&self) -> Option<Action> {
        if self.title.trim().is_empty() {
            return None;
        }
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
            status: self.create_status,
        }))
    }
}

impl Component for EditorComponent {
    fn handle_key(&mut self, key: KeyEvent, _state: &AppState) -> KeyResult {
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
            FormField::Title | FormField::Description | FormField::Tags => {
                let allow_newline = self.field == FormField::Description;
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

    fn render(&mut self, f: &mut Frame, area: Rect, _state: &AppState, _focused: bool) {
        let title = if self.edit_idx.is_some() {
            "Edit task"
        } else {
            "New task"
        };
        let inner = popup_frame(f, area, title, 68, FORM_HEIGHT);

        let [
            _,
            title_row,
            title_rule,
            notes_row,
            notes_rule,
            _,
            prio_row,
            tags_row,
            _,
            hint_row,
        ] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(NOTES_ROWS),
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

        let hint = Line::from(vec![
            Span::styled("[Ctrl-s] ", Theme::shortcut_key()),
            Span::styled("save   ", Theme::shortcut_desc()),
            Span::styled("[Tab] ", Theme::shortcut_key()),
            Span::styled("next   ", Theme::shortcut_desc()),
            Span::styled("[Esc] ", Theme::shortcut_key()),
            Span::styled("cancel", Theme::shortcut_desc()),
        ]);
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
