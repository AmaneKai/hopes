use crate::{
    app::{App, FocusPane, NavSection},
    models::Priority,
    ui::theme::Theme,
};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.focus_pane == FocusPane::Nav;
    let border_style = if is_focused {
        Theme::border_focused()
    } else {
        Theme::border()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .title(" NAVIGATOR ")
        .title_style(if is_focused {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Theme::header_title()
        });

    let stats = app.stats();
    let mut lines = Vec::with_capacity(18);

    lines.push(Line::from(Span::styled("VIEWS", Theme::table_header())));

    for (i, section) in NavSection::ALL.iter().enumerate() {
        let count = match section {
            NavSection::All => stats.total,
            NavSection::Todo => stats.todo,
            NavSection::InProgress => stats.in_progress,
            NavSection::Complete => stats.complete,
            NavSection::Urgent => stats.urgent,
        };

        let is_current = app.nav_section == *section && app.tag_filter.is_none();
        let is_cursor = is_focused && app.nav_index == i;

        let prefix = if is_cursor {
            "> "
        } else if is_current {
            "* "
        } else {
            "  "
        };

        let mut text_style = if is_current {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        if is_cursor {
            text_style = text_style
                .bg(Color::Rgb(38, 44, 58))
                .add_modifier(Modifier::BOLD);
        }

        let label = format!("{:<13}", section.label());
        let count_str = format!("({count})");

        lines.push(Line::from(vec![
            Span::styled(prefix, Style::default().fg(Color::Cyan)),
            Span::styled(format!("[{}] ", i + 1), Theme::muted()),
            Span::styled(label, text_style),
            Span::styled(count_str, Theme::muted()),
        ]));
    }

    lines.push(Line::raw(""));

    lines.push(Line::from(Span::styled(
        "PRIORITIES",
        Theme::table_header(),
    )));
    let prios = [
        ("Urgent", stats.urgent, Priority::Urgent.color()),
        ("High", stats.high, Priority::High.color()),
        ("Medium", stats.medium, Priority::Medium.color()),
        ("Low", stats.low, Priority::Low.color()),
    ];
    for (name, count, color) in prios {
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(format!("{:<8} ", name), Style::default().fg(color)),
            Span::styled(format!("({count})"), Theme::muted()),
        ]));
    }

    lines.push(Line::raw(""));

    lines.push(Line::from(Span::styled("TAGS", Theme::table_header())));
    let tags = app.all_tags_with_counts();
    if tags.is_empty() {
        lines.push(Line::from(Span::styled("  no tags", Theme::muted())));
    } else {
        for (idx, (tag, count)) in tags.iter().take(6).enumerate() {
            let nav_idx = NavSection::ALL.len() + idx;
            let is_current = match &app.tag_filter {
                Some(f) => f.strip_prefix('#').unwrap_or(f) == tag,
                None => false,
            };
            let is_cursor = is_focused && app.nav_index == nav_idx;

            let prefix = if is_cursor {
                "> "
            } else if is_current {
                "* "
            } else {
                "  "
            };

            let mut style = if is_current {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Theme::tag()
            };

            if is_cursor {
                style = style
                    .bg(Color::Rgb(38, 44, 58))
                    .add_modifier(Modifier::BOLD);
            }

            lines.push(Line::from(vec![
                Span::styled(prefix, Style::default().fg(Color::Cyan)),
                Span::styled(format!("#{:<10} ", tag), style),
                Span::styled(format!("({count})"), Theme::muted()),
            ]));
        }
    }

    let para = Paragraph::new(lines).block(block).wrap(Wrap { trim: true });
    f.render_widget(para, area);
}
