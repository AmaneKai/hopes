use crate::{
    app::{App, FocusPane},
    models::Status,
    ui::theme::Theme,
};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
};

pub fn draw_kanban(f: &mut Frame, app: &App, area: Rect) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(34),
            Constraint::Percentage(33),
        ])
        .split(area);

    let is_main_focused = app.focus_pane == FocusPane::Main;

    render_column(
        f,
        app,
        Status::Todo,
        0,
        columns[0],
        is_main_focused && app.kanban_col == 0,
    );
    render_column(
        f,
        app,
        Status::InProgress,
        1,
        columns[1],
        is_main_focused && app.kanban_col == 1,
    );
    render_column(
        f,
        app,
        Status::Complete,
        2,
        columns[2],
        is_main_focused && app.kanban_col == 2,
    );
}

fn render_column(
    f: &mut Frame,
    app: &App,
    status: Status,
    col_idx: usize,
    area: Rect,
    is_col_active: bool,
) {
    let indices = app.get_kanban_column_indices(status);
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
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(p, inner);
        return;
    }

    let mut lines = Vec::with_capacity(indices.len() * 4);
    let selected_cursor = app.kanban_cursor[col_idx];

    for (item_pos, &real_idx) in indices.iter().enumerate() {
        let item = &app.items[real_idx];
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
