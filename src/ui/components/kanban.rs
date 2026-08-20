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
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
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
        Status::Todo => (format!(" [ ] TODO ({count}) "), Color::White),
        Status::InProgress => (format!(" [-] IN PROGRESS ({count}) "), Color::Cyan),
        Status::Complete => (format!(" [x] COMPLETED ({count}) "), Color::Green),
    };

    let border_style = if is_col_active {
        Theme::border_focused()
    } else {
        Theme::border()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .title(title_text)
        .title_style(
            Style::default()
                .fg(title_color)
                .add_modifier(Modifier::BOLD),
        );

    let inner = block.inner(area);
    f.render_widget(block, area);

    if indices.is_empty() {
        let empty_msg = match status {
            Status::Todo => "No todo tasks\nPress 'a' to add",
            Status::InProgress => "No active tasks\nShift with <Space>",
            Status::Complete => "No done tasks",
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

        let cursor_sym = if is_card_selected { "> " } else { "  " };

        let mut card_title_style = if item.status == Status::Complete {
            Theme::completed()
        } else if is_card_selected {
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        if is_card_selected {
            card_title_style = card_title_style.bg(Color::Rgb(38, 44, 58));
        }

        let prio_span = Span::styled(
            format!("[{}] ", item.priority.short_label()),
            Style::default()
                .fg(item.priority.color())
                .add_modifier(Modifier::BOLD),
        );

        lines.push(Line::from(vec![
            Span::styled(cursor_sym, Style::default().fg(Color::Cyan)),
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

    let p = Paragraph::new(lines).wrap(Wrap { trim: true });
    f.render_widget(p, inner);
}
