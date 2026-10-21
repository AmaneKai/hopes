use crate::{
    action::Action,
    domain::{
        AppState, Status, ViewMode,
        due::{relative_label, urgency},
    },
    ui::{
        Component, KeyResult, TaskView,
        keys::{Chords, on_selected, task_command},
        theme::Theme,
    },
};
use chrono::Local;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
};

#[derive(Default)]
pub struct KanbanComponent {
    col: usize,
    cursor: [usize; 3],
    chords: Chords,
}

impl KanbanComponent {
    pub fn column_status(&self) -> Status {
        Status::from_column(self.col)
    }

    fn left(&mut self) {
        self.col = if self.col > 0 { self.col - 1 } else { 2 };
    }

    fn right(&mut self) {
        self.col = if self.col < 2 { self.col + 1 } else { 0 };
    }

    fn next(&mut self, count: usize) {
        if count > 0 {
            let curr = self.cursor[self.col];
            self.cursor[self.col] = if curr >= count - 1 { 0 } else { curr + 1 };
        }
    }

    fn prev(&mut self, count: usize) {
        if count > 0 {
            let curr = self.cursor[self.col];
            self.cursor[self.col] = if curr == 0 { count - 1 } else { curr - 1 };
        }
    }

    fn bottom(&mut self, count: usize) {
        if count > 0 {
            self.cursor[self.col] = count - 1;
        }
    }

    fn follow_card(&mut self, forward: bool) {
        if forward {
            self.col = (self.col + 1).min(2);
        } else {
            self.col = self.col.saturating_sub(1);
        }
    }

    fn column_len(&self, state: &AppState) -> usize {
        state.kanban_column(self.column_status()).len()
    }
}

fn shift(forward: bool) -> impl Fn(usize) -> Action {
    move |idx| Action::ShiftTaskStatus { idx, forward }
}

impl TaskView for KanbanComponent {
    fn selected_index(&self, state: &AppState) -> Option<usize> {
        let items = state.kanban_column(self.column_status());
        if items.is_empty() {
            return None;
        }
        let idx = self.cursor[self.col].min(items.len().saturating_sub(1));
        items.get(idx).copied()
    }

    fn pending_label(&self) -> Option<&'static str> {
        self.chords.label()
    }

    fn clear_pending(&mut self) {
        self.chords.clear();
    }
}

impl Component for KanbanComponent {
    fn handle_key(&mut self, key: KeyEvent, state: &AppState) -> KeyResult {
        let count = self.column_len(state);
        let selected = self.selected_index(state);

        if let Some(result) = task_command(key.code, selected) {
            return result;
        }

        match key.code {
            KeyCode::Char('j') | KeyCode::Down => self.next(count),
            KeyCode::Char('k') | KeyCode::Up => self.prev(count),
            KeyCode::Char('h') | KeyCode::Left => self.left(),
            KeyCode::Char('l') | KeyCode::Right => self.right(),
            KeyCode::Char('G') | KeyCode::End => self.bottom(count),
            KeyCode::Home => self.cursor[self.col] = 0,
            KeyCode::Char('g') => {
                if self.chords.press_g() {
                    self.cursor[self.col] = 0;
                }
            }
            KeyCode::Char('d') => {
                if self.chords.press_d() {
                    return on_selected(selected, Action::DeleteTask);
                }
            }

            KeyCode::Char('H') => return on_selected(selected, shift(false)),
            KeyCode::Char('L') | KeyCode::Char(' ') => {
                return on_selected(selected, shift(true));
            }

            KeyCode::Char('a') | KeyCode::Char('o') | KeyCode::Char('i') => {
                let status = self.column_status();
                return KeyResult::Emit(Action::OpenCreateModal { status });
            }

            _ => return KeyResult::Ignored,
        }
        KeyResult::Handled
    }

    fn update(&mut self, action: &Action, state: &AppState) {
        match action {
            Action::ShiftTaskStatus { forward, .. } => self.follow_card(*forward),
            Action::SelectNext if state.view_mode == ViewMode::Kanban => {
                self.next(self.column_len(state));
            }
            Action::SelectPrev if state.view_mode == ViewMode::Kanban => {
                self.prev(self.column_len(state));
            }
            _ => {}
        }
    }

    fn render(&mut self, f: &mut Frame, area: Rect, state: &AppState, focused: bool) {
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(33),
                Constraint::Percentage(34),
                Constraint::Percentage(33),
            ])
            .split(area);

        for (idx, status) in Status::ALL.into_iter().enumerate() {
            self.render_column(
                f,
                state,
                status,
                idx,
                columns[idx],
                focused && self.col == idx,
            );
        }
    }
}

impl KanbanComponent {
    fn render_column(
        &self,
        f: &mut Frame,
        state: &AppState,
        status: Status,
        col_idx: usize,
        area: Rect,
        is_col_active: bool,
    ) {
        let indices = state.kanban_column(status);
        let count = indices.len();

        let (title_text, title_color) = match status {
            Status::Todo => (format!("To Do \u{b7} {count}"), Color::Reset),
            Status::InProgress => (format!("In Progress \u{b7} {count}"), Color::Cyan),
            Status::Complete => (format!("Completed \u{b7} {count}"), Color::Green),
        };

        let block = Theme::pane(
            title_text,
            Style::default()
                .fg(title_color)
                .add_modifier(Modifier::BOLD),
            is_col_active,
        );

        let inner = block.inner(area);
        f.render_widget(block, area);

        if indices.is_empty() {
            let empty_msg = match status {
                Status::Todo => "Nothing to do\nPress 'a' to add",
                Status::InProgress => "Nothing in progress\nPress <Space> to start one",
                Status::Complete => "Nothing completed yet",
            };
            let p = Paragraph::new(empty_msg)
                .style(Theme::muted())
                .alignment(Alignment::Center);
            f.render_widget(p, inner);
            return;
        }

        let today = Local::now().date_naive();
        let mut lines = Vec::with_capacity(indices.len() * 4);
        let selected_cursor = self.cursor[col_idx];

        for (item_pos, &real_idx) in indices.iter().enumerate() {
            let item = &state.items[real_idx];
            let is_card_selected = is_col_active && item_pos == selected_cursor;

            let cursor_sym = Theme::cursor_prefix(is_card_selected, false);

            let card_title_style = if item.status == Status::Complete {
                Theme::completed()
            } else if is_card_selected {
                Theme::text_bold()
            } else {
                Style::default()
            };

            let prio_span = Theme::priority_marker(item.priority);

            lines.push(Line::from(vec![
                Span::raw(cursor_sym),
                prio_span,
                Span::styled(item.title.as_str(), card_title_style),
            ]));

            if !item.description.is_empty() {
                let desc_snippet = if item.description.len() > 32 {
                    format!("{}...", &item.description[..32])
                } else {
                    item.description.clone()
                };
                lines.push(Line::from(vec![
                    Span::raw("    "),
                    Span::styled(desc_snippet, Theme::muted()),
                ]));
            }

            if let (Some(due), false) = (item.due, item.status == Status::Complete) {
                lines.push(Line::from(vec![
                    Span::raw("    "),
                    Span::styled(
                        format!("due {}", relative_label(due, today)),
                        Theme::due(urgency(due, today)),
                    ),
                ]));
            }

            if !item.tags.is_empty() {
                let mut tag_spans = Vec::with_capacity(item.tags.len() + 1);
                tag_spans.push(Span::raw("    "));
                for t in &item.tags {
                    tag_spans.push(Span::styled(format!("#{t} "), Theme::tag()));
                }
                lines.push(Line::from(tag_spans));
            }

            lines.push(Line::raw(""));
        }

        let p = Paragraph::new(lines).wrap(Wrap { trim: false });
        f.render_widget(p, inner);
    }
}
