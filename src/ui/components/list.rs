use crate::{app::App, models::Status, ui::theme::Theme};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Rect},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table},
};

pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
    let visible_indices = app.get_filtered_indices();

    let title_text = match &app.tag_filter {
        Some(filter) => format!(
            " TASKS [FILTER: {filter} - {} MATCHES] ",
            visible_indices.len()
        ),
        None => format!(" TASKS [{}/{}] ", visible_indices.len(), app.items.len()),
    };

    let is_focused = app.focus_pane == crate::app::FocusPane::Main;
    let border_style = if is_focused {
        Theme::border_focused()
    } else {
        Theme::border()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .title(title_text)
        .title_style(if is_focused {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Theme::header_title()
        });

    if visible_indices.is_empty() {
        let msg = if app.items.is_empty() {
            "No tasks found. Press 'a' or 'o' to add a task."
        } else {
            "No tasks match filter. Press <Esc> to clear filter."
        };
        let empty_para = Paragraph::new(msg)
            .alignment(Alignment::Center)
            .style(Theme::muted())
            .block(block);
        f.render_widget(empty_para, area);
        return;
    }

    let header_cells = [" #", "STS", "PRIO", "TASK TITLE", "TAGS"]
        .into_iter()
        .map(|h| Cell::from(h).style(Theme::table_header()));
    let header = Row::new(header_cells).height(1).bottom_margin(0);

    let mut rows: Vec<Row> = Vec::with_capacity(visible_indices.len());
    for (display_idx, &real_idx) in visible_indices.iter().enumerate() {
        let item = &app.items[real_idx];
        let (badge, badge_color) = item.status.badge();
        let is_complete = item.status == Status::Complete;

        let idx_cell = Cell::from(format!("{:>2}", display_idx + 1)).style(Theme::muted());
        let status_cell = Cell::from(badge).style(
            Style::default()
                .fg(badge_color)
                .add_modifier(Modifier::BOLD),
        );
        let prio_cell = Cell::from(format!("[{:<4}]", item.priority.short_label())).style(
            Style::default()
                .fg(item.priority.color())
                .add_modifier(Modifier::BOLD),
        );

        let title_style = if is_complete {
            Theme::completed()
        } else {
            Style::default().fg(Color::White)
        };
        let title_cell = Cell::from(item.title.as_str()).style(title_style);

        let tags_str = if item.tags.is_empty() {
            Span::raw("")
        } else {
            Span::styled(
                item.tags
                    .iter()
                    .map(|t| format!("#{t}"))
                    .collect::<Vec<_>>()
                    .join(" "),
                Theme::tag(),
            )
        };
        let tags_cell = Cell::from(tags_str);

        rows.push(
            Row::new(vec![
                idx_cell,
                status_cell,
                prio_cell,
                title_cell,
                tags_cell,
            ])
            .height(1),
        );
    }

    let table = Table::new(
        rows,
        [
            Constraint::Length(3),
            Constraint::Length(5),
            Constraint::Length(7),
            Constraint::Percentage(55),
            Constraint::Percentage(35),
        ],
    )
    .header(header)
    .block(block)
    .row_highlight_style(Theme::highlight())
    .highlight_symbol("> ");

    f.render_stateful_widget(table, area, &mut app.table_state);
}
