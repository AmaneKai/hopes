use crate::{
    app::{App, FocusPane, ViewMode},
    ui::theme::Theme,
};
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let stats = app.stats();
    let filtered_len = app.get_filtered_indices().len();

    let view_pill = match app.view_mode {
        ViewMode::Table => Span::styled(
            " TABLE ",
            Style::default()
                .bg(Color::Rgb(30, 60, 90))
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        ViewMode::Kanban => Span::styled(
            " KANBAN ",
            Style::default()
                .bg(Color::Rgb(70, 40, 90))
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ),
    };

    let sort_pill = Span::styled(
        format!(" SORT: {} ", app.sort_mode.label()),
        Style::default()
            .bg(Color::Rgb(40, 45, 55))
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    );

    let pane_name = match app.focus_pane {
        FocusPane::Nav => "NAV",
        FocusPane::Main => "MAIN",
        FocusPane::Inspector => "INSPECTOR",
    };
    let pane_pill = Span::styled(
        format!(" FOCUS: {pane_name} "),
        Style::default().bg(Color::Rgb(35, 45, 50)).fg(Color::White),
    );

    let mut left_spans = Vec::with_capacity(9);
    left_spans.push(Span::styled(" HOPES ", Theme::header_brand()));
    left_spans.push(Span::raw(" "));
    left_spans.push(view_pill);
    left_spans.push(Span::raw(" "));
    left_spans.push(sort_pill);
    left_spans.push(Span::raw(" "));
    left_spans.push(pane_pill);

    if let Some(ref filter) = app.tag_filter {
        left_spans.push(Span::raw(" "));
        left_spans.push(Span::styled(
            format!(" FILTER: {filter} ({filtered_len}/{}) ", stats.total),
            Style::default()
                .bg(Color::Yellow)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        ));
    }

    let right_spans = vec![
        Span::styled("Total: ", Theme::muted()),
        Span::styled(format!("{:<2} ", stats.total), Theme::text_bold()),
        Span::styled("Todo: ", Theme::muted()),
        Span::styled(
            format!("{:<2} ", stats.todo),
            Style::default().fg(Color::White),
        ),
        Span::styled("WIP: ", Theme::muted()),
        Span::styled(
            format!("{:<2} ", stats.in_progress),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("Done: ", Theme::muted()),
        Span::styled(
            format!("{:<2} ", stats.complete),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("({}%)", stats.percent),
            Style::default().fg(Color::Green),
        ),
    ];

    let header_line = Line::from(left_spans);
    let right_line = Line::from(right_spans);

    let header_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Theme::border());

    let inner = header_block.inner(area);
    f.render_widget(header_block, area);

    let left_para = Paragraph::new(header_line).alignment(Alignment::Left);
    let right_para = Paragraph::new(right_line).alignment(Alignment::Right);

    f.render_widget(left_para, inner);
    f.render_widget(right_para, inner);
}
