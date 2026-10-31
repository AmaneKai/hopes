use crate::{
    domain::{AppState, Priority, Status},
    ui::{Component, KeyResult, hashtags, theme::Theme},
};
use chrono::Local;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
};

const KEY_WIDTH: usize = 11;

#[derive(Default)]
pub struct InspectorComponent {
    scroll: u16,
}

impl Component for InspectorComponent {
    fn handle_key(&mut self, key: KeyEvent, _state: &AppState) -> KeyResult {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => self.scroll = self.scroll.saturating_add(1),
            KeyCode::Char('k') | KeyCode::Up => self.scroll = self.scroll.saturating_sub(1),
            _ => return KeyResult::Ignored,
        }
        KeyResult::Handled
    }

    fn render(&mut self, f: &mut Frame, area: Rect, state: &AppState, focused: bool) {
        draw_inspector(f, state, area, focused, self.scroll);
    }
}

fn draw_inspector(f: &mut Frame, state: &AppState, area: Rect, is_focused: bool, scroll: u16) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(8),
            Constraint::Length(6),
            Constraint::Length(8),
        ])
        .split(area);

    draw_task_details(f, state, chunks[0], is_focused, scroll);
    draw_analytics(f, state, chunks[1]);
    draw_audit_trail(f, state, chunks[2]);
}

fn property(name: &str, mut value: Vec<Span<'static>>) -> Line<'static> {
    let mut spans = vec![Span::styled(format!("{name:<KEY_WIDTH$}"), Theme::muted())];
    spans.append(&mut value);
    Line::from(spans)
}

fn draw_task_details(f: &mut Frame, state: &AppState, area: Rect, is_focused: bool, scroll: u16) {
    let block = Theme::titled_pane("Inspector", is_focused);

    let Some(item) = state.selected_item() else {
        let empty_para = Paragraph::new("\nNo task selected")
            .style(Theme::muted())
            .block(block);
        f.render_widget(empty_para, area);
        return;
    };

    let (badge, badge_color) = item.status.badge();
    let rule_width = block.inner(area).width as usize;

    let mut lines = Vec::with_capacity(10);
    lines.push(Line::raw(""));
    lines.push(Line::from(Span::styled(
        item.title.clone(),
        Theme::text_bold(),
    )));
    lines.push(Line::raw(""));

    lines.push(property(
        "Status",
        vec![Span::styled(
            format!("{badge} {}", item.status.label()),
            Style::default().fg(badge_color),
        )],
    ));
    lines.push(property(
        "Priority",
        vec![Span::styled(
            format!("{} {}", item.priority.glyph(), item.priority.label()),
            Style::default().fg(item.priority.color()),
        )],
    ));
    let due = match item.window() {
        Some(window) => {
            let today = Local::now().date_naive();
            let style = if item.status == Status::Complete {
                Theme::muted()
            } else {
                Theme::due(window.urgency(today))
            };
            vec![
                Span::raw(format!("{} ", window.full_label())),
                Span::styled(format!("\u{b7} {}", window.label(today)), style),
            ]
        }
        None => vec![Span::styled("None", Theme::muted())],
    };
    lines.push(property("Due", due));
    let repeat = match item.repeat {
        Some(rule) => vec![Span::raw(format!("\u{21bb} {rule}"))],
        None => vec![Span::styled("None", Theme::muted())],
    };
    lines.push(property("Repeat", repeat));
    let tags = if item.tags.is_empty() {
        vec![Span::styled("Empty", Theme::muted())]
    } else {
        vec![Span::styled(hashtags(&item.tags, ", "), Theme::tag())]
    };
    lines.push(property("Tags", tags));

    if !item.description.is_empty() {
        lines.push(Line::from(Span::styled(
            "\u{2500}".repeat(rule_width),
            Theme::muted(),
        )));
        for desc_line in item.description.lines() {
            lines.push(Line::from(desc_line.to_string()));
        }
    }

    let para = Paragraph::new(lines)
        .block(block)
        .scroll((scroll, 0))
        .wrap(Wrap { trim: false });
    f.render_widget(para, area);
}

fn draw_analytics(f: &mut Frame, state: &AppState, area: Rect) {
    let stats = state.stats();

    let block = Theme::titled_pane("Progress", false);

    let bar_width = 16usize;
    let filled = (stats.percent * bar_width)
        .checked_div(100usize)
        .unwrap_or(0);
    let empty = bar_width.saturating_sub(filled);

    let progress_line = property(
        "Done",
        vec![
            Span::styled("\u{2501}".repeat(filled), Style::default().fg(Color::Green)),
            Span::styled("\u{2501}".repeat(empty), Theme::muted()),
            Span::styled(format!(" {}%", stats.percent), Theme::muted()),
        ],
    );

    let counts_line = property(
        "Status",
        vec![Span::raw(format!(
            "{} to do \u{b7} {} active \u{b7} {} done",
            stats.todo, stats.in_progress, stats.complete
        ))],
    );

    let prio_counts = [
        (Priority::Urgent, stats.urgent),
        (Priority::High, stats.high),
        (Priority::Medium, stats.medium),
        (Priority::Low, stats.low),
    ];
    let mut prio_spans = Vec::new();
    for (priority, count) in prio_counts {
        prio_spans.push(Theme::priority_marker(priority));
        prio_spans.push(Span::raw(format!("{count}  ")));
    }
    let prio_line = property("Priority", prio_spans);

    let para = Paragraph::new(vec![progress_line, counts_line, prio_line]).block(block);
    f.render_widget(para, area);
}

fn draw_audit_trail(f: &mut Frame, state: &AppState, area: Rect) {
    let block = Theme::titled_pane("Activity", false);

    let visible_rows = area.height.saturating_sub(1) as usize;

    let lines: Vec<Line> = if visible_rows == 0 {
        Vec::new()
    } else {
        let mut entries: Vec<Line> = Vec::new();
        for event in state.audit.recent(visible_rows) {
            let time = format!("{} ", event.timestamp.format("%H:%M"));
            entries.push(Line::from(vec![
                Span::styled(time, Theme::muted()),
                Span::raw(event.message.as_str()),
            ]));
        }

        if entries.is_empty() {
            entries.push(Line::from(Span::styled(
                "No activity yet this session",
                Theme::muted(),
            )));
        }
        entries
    };

    let para = Paragraph::new(lines).block(block);
    f.render_widget(para, area);
}
