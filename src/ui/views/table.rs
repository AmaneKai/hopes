use crate::{
    app::{App, FocusPane},
    models::{Priority, Status},
    ui::theme::Theme,
};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Rect},
    style::{Modifier, Style},
    widgets::{Cell, Paragraph, Row, Table},
};

/// Task list as a stack of blocks: status glyph, title, priority, tags. No header
/// row and no index column.
pub fn draw_table(f: &mut Frame, app: &mut App, area: Rect) {
    let visible_indices = app.get_filtered_indices();

    let title_text = match &app.tag_filter {
        Some(filter) => format!("Tasks \u{b7} {filter} \u{b7} {}", visible_indices.len()),
        None => format!("Tasks \u{b7} {}", visible_indices.len()),
    };

    let is_focused = app.focus_pane == FocusPane::Main;
    let block = Theme::titled_pane(title_text, is_focused);

    if visible_indices.is_empty() {
        let msg = if app.items.is_empty() {
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

    let mut rows: Vec<Row> = Vec::with_capacity(visible_indices.len());
    for &real_idx in visible_indices {
        let item = &app.items[real_idx];
        let (badge, badge_color) = item.status.badge();
        let is_complete = item.status == Status::Complete;

        let status_cell = Cell::from(badge).style(Style::default().fg(badge_color));

        let title_style = if is_complete {
            Theme::completed()
        } else {
            Style::default()
        };
        let title_cell = Cell::from(item.title.as_str()).style(title_style);

        // Medium is the default, so it stays quiet; the rest get a labeled glyph.
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

        let tags_cell = Cell::from(
            item.tags
                .iter()
                .map(|t| format!("#{t}"))
                .collect::<Vec<_>>()
                .join(" "),
        )
        .style(Theme::tag());

        rows.push(Row::new(vec![
            status_cell,
            title_cell,
            prio_cell,
            tags_cell,
        ]));
    }

    let table = Table::new(
        rows,
        [
            Constraint::Length(2),
            Constraint::Min(12),
            Constraint::Length(7),
            Constraint::Percentage(25),
        ],
    )
    .block(block)
    .row_highlight_style(Theme::highlight())
    .highlight_symbol("> ");

    f.render_stateful_widget(table, area, &mut app.table_state);
}
