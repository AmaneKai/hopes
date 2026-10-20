use crate::{
    action::{Action, NavTarget},
    domain::{AppState, FocusPane, NavSection, Priority},
    ui::{Component, KeyResult, theme::Theme},
};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

#[derive(Default)]
pub struct NavComponent {
    index: usize,
}

impl NavComponent {
    fn total(state: &AppState) -> usize {
        NavSection::ALL.len() + state.tags().len()
    }

    fn target(&self, state: &AppState) -> Option<NavTarget> {
        if let Some(section) = NavSection::ALL.get(self.index) {
            return Some(NavTarget::Section(*section));
        }
        let (tag, _) = state.tags().get(self.index - NavSection::ALL.len())?;
        Some(NavTarget::Tag(tag.clone()))
    }

    fn apply(&self, state: &AppState) -> Option<Action> {
        self.target(state).map(Action::ApplyNav)
    }
}

impl Component for NavComponent {
    fn handle_key(&mut self, key: KeyEvent, state: &AppState) -> KeyResult {
        let total = Self::total(state);
        let emit = |action: Option<Action>| match action {
            Some(a) => KeyResult::Emit(a),
            None => KeyResult::Handled,
        };

        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if total == 0 {
                    return KeyResult::Handled;
                }
                self.index = (self.index + 1) % total;
                emit(self.apply(state))
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if total == 0 {
                    return KeyResult::Handled;
                }
                self.index = if self.index == 0 {
                    total - 1
                } else {
                    self.index - 1
                };
                emit(self.apply(state))
            }
            KeyCode::Char(' ') => emit(self.apply(state)),
            KeyCode::Enter => match self.apply(state) {
                Some(open) => {
                    KeyResult::Emit(Action::Batch(vec![open, Action::SetFocus(FocusPane::Main)]))
                }
                None => KeyResult::Emit(Action::SetFocus(FocusPane::Main)),
            },
            KeyCode::Char(c @ '1'..='5') => {
                let idx = (c as usize) - ('1' as usize);
                if idx < NavSection::ALL.len() {
                    self.index = idx;
                }
                emit(self.apply(state))
            }
            _ => KeyResult::Ignored,
        }
    }

    fn update(&mut self, action: &Action, _state: &AppState) {
        if matches!(action, Action::ResetNavCursor) {
            self.index = 0;
        }
    }

    fn render(&mut self, f: &mut Frame, area: Rect, state: &AppState, focused: bool) {
        let block = Theme::titled_pane("Navigator", focused);
        let inner_width = block.inner(area).width as usize;

        let stats = state.stats();
        let mut lines = Vec::with_capacity(18);

        let row = |prefix: Span<'static>, label: String, label_style: Style, count: usize| {
            let used = prefix.width() + label.chars().count();
            let count_str = count.to_string();
            let gap = inner_width.saturating_sub(used + count_str.len()).max(1);
            Line::from(vec![
                prefix,
                Span::styled(label, label_style),
                Span::raw(" ".repeat(gap)),
                Span::styled(count_str, Theme::muted()),
            ])
        };

        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled("  Views", Theme::section())));

        for (i, section) in NavSection::ALL.iter().enumerate() {
            let count = match section {
                NavSection::All => stats.total,
                NavSection::Todo => stats.todo,
                NavSection::InProgress => stats.in_progress,
                NavSection::Complete => stats.complete,
                NavSection::Urgent => stats.urgent,
            };

            let is_current = state.nav_section == *section && state.tag_filter.is_none();
            let is_cursor = focused && self.index == i;

            let style = if is_current || is_cursor {
                Theme::text_bold()
            } else {
                Style::default()
            };

            lines.push(row(
                Span::raw(Theme::cursor_prefix(is_cursor, is_current)),
                section.label().to_string(),
                style,
                count,
            ));
        }

        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled("  Priority", Theme::section())));
        let prios = [
            ("Urgent", stats.urgent, Priority::Urgent),
            ("High", stats.high, Priority::High),
            ("Medium", stats.medium, Priority::Medium),
            ("Low", stats.low, Priority::Low),
        ];
        for (name, count, priority) in prios {
            lines.push(row(
                Theme::priority_marker(priority),
                name.to_string(),
                Style::default(),
                count,
            ));
        }

        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled("  Tags", Theme::section())));
        let tags = state.tags();
        if tags.is_empty() {
            lines.push(Line::from(Span::styled("  No tags yet", Theme::muted())));
        } else {
            for (idx, (tag, count)) in tags.iter().take(6).enumerate() {
                let nav_idx = NavSection::ALL.len() + idx;
                let is_current = match &state.tag_filter {
                    Some(f) => f.strip_prefix('#').unwrap_or(f) == tag,
                    None => false,
                };
                let is_cursor = focused && self.index == nav_idx;

                let style = if is_current || is_cursor {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Theme::tag()
                };

                lines.push(row(
                    Span::raw(Theme::cursor_prefix(is_cursor, is_current)),
                    format!("#{tag}"),
                    style,
                    *count,
                ));
            }
        }

        f.render_widget(Paragraph::new(lines).block(block), area);
    }
}
