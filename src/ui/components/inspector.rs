use crate::{
    app::{App, FocusPane},
    models::Priority,
    ui::theme::Theme,
};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.focus_pane == FocusPane::Inspector;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(8),
            Constraint::Length(7),
            Constraint::Length(8),
        ])
        .split(area);

    render_task_details(f, app, chunks[0], is_focused);
    render_analytics(f, app, chunks[1]);
    render_audit_trail(f, app, chunks[2]);
}

fn render_task_details(f: &mut Frame, app: &App, area: Rect, is_focused: bool) {
    let border_style = if is_focused {
        Theme::border_focused()
    } else {
        Theme::border()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .title(" TASK INSPECTOR ")
        .title_style(if is_focused {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Theme::header_title()
        });

    if let Some(item) = app.get_selected_item() {
        let (badge, badge_color) = item.status.badge();
        let prio_style = Style::default()
            .fg(item.priority.color())
            .add_modifier(Modifier::BOLD);

        let status_span = Span::styled(
            format!("{badge} {}", item.status.label()),
            Style::default()
                .fg(badge_color)
                .add_modifier(Modifier::BOLD),
        );
        let prio_span = Span::styled(
            format!(
                "[{}] {}",
                item.priority.short_label(),
                item.priority.label()
            ),
            prio_style,
        );

        let mut lines = Vec::with_capacity(8);
        lines.push(Line::from(vec![
            Span::styled("Status: ", Theme::muted()),
            status_span,
            Span::raw("   "),
            Span::styled("Priority: ", Theme::muted()),
            prio_span,
        ]));
        lines.push(Line::raw(""));
        lines.push(Line::from(vec![
            Span::styled("Title: ", Theme::muted()),
            Span::styled(item.title.as_str(), Theme::text_bold()),
        ]));

        if !item.description.is_empty() {
            lines.push(Line::raw(""));
            lines.push(Line::from(Span::styled(
                "Notes / Description:",
                Theme::table_header(),
            )));
            for desc_line in item.description.lines() {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(desc_line, Style::default().fg(Color::Rgb(210, 215, 230))),
                ]));
            }
        }

        lines.push(Line::raw(""));
        if item.tags.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("Tags: ", Theme::muted()),
                Span::styled("none", Theme::muted()),
            ]));
        } else {
            let mut tag_spans = Vec::with_capacity(item.tags.len() + 1);
            tag_spans.push(Span::styled("Tags: ", Theme::muted()));
            for tag in &item.tags {
                tag_spans.push(Span::styled(format!("#{tag} "), Theme::tag()));
            }
            lines.push(Line::from(tag_spans));
        }

        let para = Paragraph::new(lines)
            .block(block)
            .scroll((app.inspector_scroll, 0))
            .wrap(Wrap { trim: true });
        f.render_widget(para, area);
    } else {
        let empty_para = Paragraph::new("No task selected")
            .style(Theme::muted())
            .block(block);
        f.render_widget(empty_para, area);
    }
}

fn render_analytics(f: &mut Frame, app: &App, area: Rect) {
    let stats = app.stats();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Theme::border())
        .title(" WORKSPACE ANALYTICS ")
        .title_style(Theme::header_title());

    let bar_width = 16usize;
    let filled = (stats.percent * bar_width)
        .checked_div(100usize)
        .unwrap_or(0);
    let empty = bar_width.saturating_sub(filled);
    let progress_bar = format!(
        "[{}{}] {}%",
        "=".repeat(filled),
        ".".repeat(empty),
        stats.percent
    );

    let progress_line = Line::from(vec![
        Span::styled("Progress: ", Theme::muted()),
        Span::styled(
            progress_bar,
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
    ]);

    let counts_line = Line::from(vec![
        Span::styled("[ ] Todo: ", Theme::muted()),
        Span::styled(
            format!("{:<2} ", stats.todo),
            Style::default().fg(Color::White),
        ),
        Span::styled("[-] WIP: ", Theme::muted()),
        Span::styled(
            format!("{:<2} ", stats.in_progress),
            Style::default().fg(Color::Cyan),
        ),
        Span::styled("[x] Done: ", Theme::muted()),
        Span::styled(
            format!("{:<2} ", stats.complete),
            Style::default().fg(Color::Green),
        ),
    ]);

    let prio_line = Line::from(vec![
        Span::styled("[URG] ", Style::default().fg(Priority::Urgent.color())),
        Span::styled(format!("{:<2} ", stats.urgent), Theme::text_bold()),
        Span::styled("[HI] ", Style::default().fg(Priority::High.color())),
        Span::styled(format!("{:<2} ", stats.high), Theme::text_bold()),
        Span::styled("[MED] ", Style::default().fg(Priority::Medium.color())),
        Span::styled(format!("{:<2} ", stats.medium), Theme::text_bold()),
        Span::styled("[LOW] ", Style::default().fg(Priority::Low.color())),
        Span::styled(format!("{:<2} ", stats.low), Theme::text_bold()),
    ]);

    let para = Paragraph::new(vec![progress_line, counts_line, prio_line]).block(block);
    f.render_widget(para, area);
}

fn render_audit_trail(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Theme::border())
        .title(" AUDIT TRAIL ")
        .title_style(Theme::header_title());

    let visible_rows = area.height.saturating_sub(2) as usize;

    let lines: Vec<Line> = if visible_rows == 0 {
        Vec::new()
    } else {
        let mut entries: Vec<Line> = app
            .audit_log
            .recent(visible_rows)
            .map(|event| {
                Line::from(vec![
                    Span::styled(
                        format!("{} ", event.timestamp.format("%m-%d %H:%M:%S")),
                        Theme::muted(),
                    ),
                    Span::styled(event.message.as_str(), Theme::shortcut_desc()),
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
