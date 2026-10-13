use crate::{
    app::{App, FocusPane},
    models::Priority,
    ui::theme::Theme,
};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
};

/// Width of the property-name column ("Priority" plus breathing room).
const KEY_WIDTH: usize = 11;

pub fn draw_inspector(f: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.focus_pane == FocusPane::Inspector;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(8),
            Constraint::Length(6),
            Constraint::Length(8),
        ])
        .split(area);

    draw_task_details(f, app, chunks[0], is_focused);
    draw_analytics(f, app, chunks[1]);
    draw_audit_trail(f, app, chunks[2]);
}

/// One row of the properties block: muted name, normal value.
fn property(name: &str, mut value: Vec<Span<'static>>) -> Line<'static> {
    let mut spans = vec![Span::styled(format!("{name:<KEY_WIDTH$}"), Theme::muted())];
    spans.append(&mut value);
    Line::from(spans)
}

fn draw_task_details(f: &mut Frame, app: &App, area: Rect, is_focused: bool) {
    let block = Theme::titled_pane("Inspector", is_focused);

    let Some(item) = app.get_selected_item() else {
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
    let tags = if item.tags.is_empty() {
        vec![Span::styled("Empty", Theme::muted())]
    } else {
        vec![Span::styled(
            item.tags
                .iter()
                .map(|t| format!("#{t}"))
                .collect::<Vec<_>>()
                .join(", "),
            Theme::tag(),
        )]
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
        .scroll((app.inspector_scroll, 0))
        .wrap(Wrap { trim: false });
    f.render_widget(para, area);
}

fn draw_analytics(f: &mut Frame, app: &App, area: Rect) {
    let stats = app.stats();

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
    let prio_line = property(
        "Priority",
        prio_counts
            .into_iter()
            .flat_map(|(priority, count)| {
                [
                    Theme::priority_marker(priority),
                    Span::raw(format!("{count}  ")),
                ]
            })
            .collect(),
    );

    let para = Paragraph::new(vec![progress_line, counts_line, prio_line]).block(block);
    f.render_widget(para, area);
}

fn draw_audit_trail(f: &mut Frame, app: &App, area: Rect) {
    let block = Theme::titled_pane("Activity", false);

    let visible_rows = area.height.saturating_sub(1) as usize;

    let lines: Vec<Line> = if visible_rows == 0 {
        Vec::new()
    } else {
        let mut entries: Vec<Line> = app
            .audit_log
            .recent(visible_rows)
            .map(|event| {
                Line::from(vec![
                    Span::styled(
                        format!("{} ", event.timestamp.format("%H:%M")),
                        Theme::muted(),
                    ),
                    Span::raw(event.message.as_str()),
                ])
            })
            .collect();

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
