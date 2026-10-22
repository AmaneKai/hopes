use crate::{
    action::Action,
    domain::{AppState, Priority, Status, ViewMode, due::DueUrgency},
    ui::{
        Component, KeyResult, TaskView,
        keys::{Chords, on_selected, task_command},
        theme::Theme,
    },
};
use chrono::Local;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Rect},
    style::{Modifier, Style},
    widgets::{Cell, Paragraph, Row, Table, TableState},
};

const PAGE: usize = 5;
/// Highlight symbol, status, due, priority and the four column gaps.
const MAX_DUE_WIDTH: usize = 16;
const FIXED_COLS: usize = 2 + 2 + 7 + 4;
const MIN_TITLE: usize = 12;

fn ellipsize(text: &str, width: usize) -> String {
    if text.chars().count() <= width {
        return text.to_string();
    }
    let mut out: String = text.chars().take(width.saturating_sub(1)).collect();
    out.push('\u{2026}');
    out
}

#[derive(Default)]
pub struct TableComponent {
    table_state: TableState,
    chords: Chords,
}

impl TableComponent {
    pub fn new(has_items: bool) -> Self {
        let mut table_state = TableState::default();
        if has_items {
            table_state.select(Some(0));
        }
        Self {
            table_state,
            ..Self::default()
        }
    }

    pub fn position(&self, state: &AppState) -> usize {
        match self.table_state.selected() {
            Some(i) if !state.filtered_indices().is_empty() => i + 1,
            _ => 0,
        }
    }

    fn next(&mut self, count: usize) {
        if count == 0 {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) if i >= count - 1 => 0,
            Some(i) => i + 1,
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    fn prev(&mut self, count: usize) {
        if count == 0 {
            return;
        }
        let i = match self.table_state.selected() {
            Some(0) => count - 1,
            Some(i) => i - 1,
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    fn top(&mut self, count: usize) {
        if count > 0 {
            self.table_state.select(Some(0));
        }
    }

    fn bottom(&mut self, count: usize) {
        if count > 0 {
            self.table_state.select(Some(count - 1));
        }
    }

    fn page_down(&mut self, count: usize) {
        if count == 0 {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => (i + PAGE).min(count - 1),
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    fn page_up(&mut self, count: usize) {
        if count == 0 {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => i.saturating_sub(PAGE),
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    fn clamp(&mut self, count: usize) {
        if count == 0 {
            self.table_state.select(None);
        } else {
            let row = self.table_state.selected().unwrap_or(0).min(count - 1);
            self.table_state.select(Some(row));
        }
    }
}

impl TaskView for TableComponent {
    fn selected_index(&self, state: &AppState) -> Option<usize> {
        let row = self.table_state.selected()?;
        state.filtered_indices().get(row).copied()
    }

    fn pending_label(&self) -> Option<&'static str> {
        self.chords.label()
    }

    fn clear_pending(&mut self) {
        self.chords.clear();
    }
}

impl Component for TableComponent {
    fn handle_key(&mut self, key: KeyEvent, state: &AppState) -> KeyResult {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let count = state.filtered_indices().len();
        let selected = self.selected_index(state);

        if let Some(result) = task_command(key.code, selected) {
            return result;
        }

        match key.code {
            KeyCode::Char('d') if ctrl => self.page_down(count),
            KeyCode::Char('u') if ctrl => self.page_up(count),
            KeyCode::PageDown => self.page_down(count),
            KeyCode::PageUp => self.page_up(count),

            KeyCode::Char('j') | KeyCode::Down => self.next(count),
            KeyCode::Char('k') | KeyCode::Up => self.prev(count),
            KeyCode::Char('G') | KeyCode::End | KeyCode::Char('L') => self.bottom(count),
            KeyCode::Char('H') | KeyCode::Home => self.top(count),
            KeyCode::Char('g') => {
                if self.chords.press_g() {
                    self.top(count);
                }
            }
            KeyCode::Char('d') => {
                if self.chords.press_d() {
                    return on_selected(selected, Action::DeleteTask);
                }
            }

            KeyCode::Char('a') | KeyCode::Char('o') | KeyCode::Char('i') => {
                let status = Status::Todo;
                return KeyResult::Emit(Action::OpenCreateModal { status });
            }
            KeyCode::Char(' ') => return on_selected(selected, Action::ToggleTaskStatus),

            _ => return KeyResult::Ignored,
        }
        KeyResult::Handled
    }

    fn update(&mut self, action: &Action, state: &AppState) {
        let count = state.filtered_indices().len();
        match action {
            Action::DeleteTask(_) | Action::DataReloaded => self.clamp(count),
            Action::SelectIndex(i) => self.table_state.select(Some(*i)),
            Action::SelectNext if state.view_mode == ViewMode::Table => {
                self.next(count);
            }
            Action::SelectPrev if state.view_mode == ViewMode::Table => {
                self.prev(count);
            }
            _ => {}
        }
    }

    fn render(&mut self, f: &mut Frame, area: Rect, state: &AppState, focused: bool) {
        let visible_indices = state.filtered_indices();

        let title_text = match &state.tag_filter {
            Some(filter) => format!("Tasks \u{b7} {filter} \u{b7} {}", visible_indices.len()),
            None => format!("Tasks \u{b7} {}", visible_indices.len()),
        };

        let block = Theme::titled_pane(title_text, focused);

        if visible_indices.is_empty() {
            let msg = if state.items.is_empty() {
                "Nothing here yet. Press 'a' to add a task."
            } else {
                "No tasks match. Press <Esc> to clear the filter."
            };
            let empty_para = Paragraph::new(msg)
                .alignment(Alignment::Center)
                .style(Theme::muted())
                .block(block);
            f.render_widget(empty_para, area);
            return;
        }

        let tag_texts: Vec<String> = visible_indices
            .iter()
            .map(|&i| {
                state.items[i]
                    .tags
                    .iter()
                    .map(|t| format!("#{t}"))
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect();
        let inner_width = area.width.saturating_sub(3) as usize;
        let tags_width = tag_texts
            .iter()
            .map(|t| t.chars().count())
            .max()
            .unwrap_or(0)
            .min(inner_width / 3);
        let today = Local::now().date_naive();
        let due_texts: Vec<Option<(String, DueUrgency)>> = visible_indices
            .iter()
            .map(|&i| {
                let item = &state.items[i];
                let window = item.window().filter(|_| item.status != Status::Complete)?;
                let mark = if item.repeat.is_some() {
                    "\u{21bb} "
                } else {
                    ""
                };
                Some((
                    format!("{mark}{}", window.label(today)),
                    window.urgency(today),
                ))
            })
            .collect();
        let due_width = due_texts
            .iter()
            .flatten()
            .map(|(text, _)| text.chars().count())
            .max()
            .unwrap_or(0)
            .min(MAX_DUE_WIDTH);
        let title_width = inner_width
            .saturating_sub(FIXED_COLS + tags_width + due_width)
            .max(MIN_TITLE);

        let mut rows: Vec<Row> = Vec::with_capacity(visible_indices.len());
        for (row, &real_idx) in visible_indices.iter().enumerate() {
            let item = &state.items[real_idx];
            let (badge, badge_color) = item.status.badge();
            let is_complete = item.status == Status::Complete;

            let status_cell = Cell::from(badge).style(Style::default().fg(badge_color));

            let title_style = if is_complete {
                Theme::completed()
            } else {
                Style::default()
            };
            let title_cell = Cell::from(ellipsize(&item.title, title_width)).style(title_style);

            let due_cell = match &due_texts[row] {
                Some((text, urgency)) => {
                    Cell::from(ellipsize(text, due_width)).style(Theme::due(*urgency))
                }
                None => Cell::from(""),
            };

            let prio_cell = if is_complete {
                Cell::from("")
            } else {
                let label = format!("{} {}", item.priority.glyph(), item.priority.short_label());
                let mut style = Style::default().fg(item.priority.color());
                if matches!(item.priority, Priority::Urgent | Priority::High) {
                    style = style.add_modifier(Modifier::BOLD);
                }
                Cell::from(label).style(style)
            };

            let tags_cell = Cell::from(ellipsize(&tag_texts[row], tags_width)).style(Theme::tag());

            rows.push(Row::new(vec![
                status_cell,
                title_cell,
                due_cell,
                prio_cell,
                tags_cell,
            ]));
        }

        let table = Table::new(
            rows,
            [
                Constraint::Length(2),
                Constraint::Min(MIN_TITLE as u16),
                Constraint::Length(due_width as u16),
                Constraint::Length(7),
                Constraint::Length(tags_width as u16),
            ],
        )
        .block(block)
        .row_highlight_style(Theme::highlight())
        .highlight_symbol("> ");

        f.render_stateful_widget(table, area, &mut self.table_state);
    }
}
