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
    widgets::Paragraph,
};

/// Navigator: saved views, priority counts and tags. Counts are right-aligned.
pub fn draw_sidebar(f: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.focus_pane == FocusPane::Nav;
    let block = Theme::titled_pane("Navigator", is_focused);
    let inner_width = block.inner(area).width as usize;

    let stats = app.stats();
    let mut lines = Vec::with_capacity(18);

    // Label left, count right, filling the pane width.
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

        let is_current = app.nav_section == *section && app.tag_filter.is_none();
        let is_cursor = is_focused && app.nav_index == i;

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
    let tags = app.all_tags_with_counts();
    if tags.is_empty() {
        lines.push(Line::from(Span::styled("  No tags yet", Theme::muted())));
    } else {
        for (idx, (tag, count)) in tags.iter().take(6).enumerate() {
            let nav_idx = NavSection::ALL.len() + idx;
            let is_current = match &app.tag_filter {
                Some(f) => f.strip_prefix('#').unwrap_or(f) == tag,
                None => false,
            };
            let is_cursor = is_focused && app.nav_index == nav_idx;

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
