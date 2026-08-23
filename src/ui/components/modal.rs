use crate::{
    app::{App, AppMode, FormField},
    models::Priority,
    ui::theme::Theme,
};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};

/// Renders `text` as a `Text`, highlighting the [lo, hi] char range (if any
/// Visual-mode selection is active) with `Theme::field_selection()`. Splits
/// correctly across embedded `\n`s so multi-line Description selections work
/// too — Paragraph needs one `Line` per visual row, not one long string.
fn styled_field_text(text: &str, cursor: usize, visual_anchor: Option<usize>) -> Text<'static> {
    let base_style = Style::default().fg(Color::White);
    let Some(anchor) = visual_anchor else {
        return Text::from(text.to_string()).style(base_style);
    };
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();
    if n == 0 {
        return Text::from("");
    }
    let (lo, hi) = crate::textedit::selection_range(anchor, cursor);
    let hi = hi.min(n - 1);

    let mut lines = Vec::new();
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut run = String::new();
    let mut run_style = base_style;

    for (i, &c) in chars.iter().enumerate() {
        let style = if i >= lo && i <= hi {
            Theme::field_selection()
        } else {
            base_style
        };
        if c == '\n' {
            if !run.is_empty() {
                spans.push(Span::styled(std::mem::take(&mut run), run_style));
            }
            lines.push(Line::from(std::mem::take(&mut spans)));
            continue;
        }
        if style != run_style && !run.is_empty() {
            spans.push(Span::styled(std::mem::take(&mut run), run_style));
        }
        run_style = style;
        run.push(c);
    }
    if !run.is_empty() {
        spans.push(Span::styled(run, run_style));
    }
    lines.push(Line::from(spans));
    Text::from(lines)
}

pub fn render_form_modal(f: &mut Frame, app: &App) {
    let area = centered_rect(68, 18, f.area());
    f.render_widget(Clear, area);

    let is_edit = app.mode == AppMode::Editing;
    let modal_title = if is_edit {
        " [EDIT TASK] "
    } else {
        " [NEW TASK] "
    };

    let modal_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan))
        .title(modal_title)
        .title_style(
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        );

    let inner = modal_block.inner(area);
    f.render_widget(modal_block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(4),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(inner);

    let title_focused = app.form_field == FormField::Title;
    let title_border = if title_focused {
        Theme::border_focused()
    } else {
        Theme::border()
    };
    let title_visual = title_focused.then_some(app.field_visual_anchor).flatten();
    let title_input = Paragraph::new(styled_field_text(
        &app.form_title,
        app.text_cursor,
        title_visual,
    ))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(title_border)
            .title(" Task Title "),
    );
    f.render_widget(title_input, chunks[0]);
    if title_focused {
        let cursor_x =
            chunks[0].x + 1 + (app.text_cursor as u16).min(chunks[0].width.saturating_sub(3));
        let cursor_y = chunks[0].y + 1;
        f.set_cursor_position((cursor_x, cursor_y));
    }

    let desc_focused = app.form_field == FormField::Description;
    let desc_border = if desc_focused {
        Theme::border_focused()
    } else {
        Theme::border()
    };
    let desc_visual = desc_focused.then_some(app.field_visual_anchor).flatten();
    let desc_input = Paragraph::new(styled_field_text(
        &app.form_description,
        app.text_cursor,
        desc_visual,
    ))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(desc_border)
            .title(" Description / Notes "),
    );
    f.render_widget(desc_input, chunks[1]);
    if desc_focused {
        let (row, col) = row_col_of_cursor(&app.form_description, app.text_cursor);
        let cursor_x = chunks[1].x + 1 + col.min(chunks[1].width.saturating_sub(3));
        let cursor_y = chunks[1].y + 1 + row.min(chunks[1].height.saturating_sub(3));
        f.set_cursor_position((cursor_x, cursor_y));
    }

    let prio_focused = app.form_field == FormField::Priority;
    let mut priority_spans = Vec::with_capacity(Priority::ALL.len() + 1);
    priority_spans.push(Span::raw(" "));
    for (i, p) in Priority::ALL.iter().enumerate() {
        let is_selected = *p == app.form_priority;
        let radio = if is_selected { "(x) " } else { "( ) " };
        let mut style = Style::default().fg(p.color());
        if is_selected {
            style = style.add_modifier(Modifier::BOLD | Modifier::UNDERLINED);
        }
        priority_spans.push(Span::styled(
            format!("{}{} [{}]  ", radio, p.label(), i + 1),
            style,
        ));
    }

    let prio_border = if prio_focused {
        Theme::border_focused()
    } else {
        Theme::border()
    };
    let priority_widget = Paragraph::new(Line::from(priority_spans)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(prio_border)
            .title(" Priority [h/l or 1-4] "),
    );
    f.render_widget(priority_widget, chunks[2]);

    let tags_focused = app.form_field == FormField::Tags;
    let tags_border = if tags_focused {
        Theme::border_focused()
    } else {
        Theme::border()
    };
    let tags_visual = tags_focused.then_some(app.field_visual_anchor).flatten();
    let tags_input = Paragraph::new(styled_field_text(
        &app.form_tags,
        app.text_cursor,
        tags_visual,
    ))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(tags_border)
            .title(" Tags (comma-separated, e.g. rust, cli) "),
    );
    f.render_widget(tags_input, chunks[3]);
    if tags_focused {
        let cursor_x =
            chunks[3].x + 1 + (app.text_cursor as u16).min(chunks[3].width.saturating_sub(3));
        let cursor_y = chunks[3].y + 1;
        f.set_cursor_position((cursor_x, cursor_y));
    }

    let hint_spans = vec![
        Span::styled("<Tab>/<C-j/k>", Theme::shortcut_key()),
        Span::styled(":Switch ", Theme::shortcut_desc()),
        Span::styled("<Enter>/<C-s>", Theme::shortcut_key()),
        Span::styled(":Save ", Theme::shortcut_desc()),
        Span::styled("<C-w>", Theme::shortcut_key()),
        Span::styled(":Del Word ", Theme::shortcut_desc()),
        Span::styled("<Esc>", Theme::shortcut_key()),
        Span::styled(":Cancel", Theme::shortcut_desc()),
    ];
    let hint_widget = Paragraph::new(Line::from(hint_spans)).alignment(Alignment::Center);
    f.render_widget(hint_widget, chunks[4]);
}

pub fn render_filter_modal(f: &mut Frame, app: &App) {
    let area = centered_rect(60, 5, f.area());
    f.render_widget(Clear, area);

    let matching = app.get_filtered_indices().len();
    let total = app.items.len();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Yellow))
        .title(format!(
            " LIVE SEARCH / FILTER [{matching}/{total} MATCHES] "
        ))
        .title_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(inner);

    let mut input_spans = vec![Span::styled(
        "/ ",
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )];
    let text = styled_field_text(&app.filter_input, app.text_cursor, app.field_visual_anchor);
    if let Some(line) = text.lines.into_iter().next() {
        input_spans.extend(line.spans);
    }
    f.render_widget(Paragraph::new(Line::from(input_spans)), chunks[0]);

    let cursor_x =
        chunks[0].x + 2 + (app.text_cursor as u16).min(chunks[0].width.saturating_sub(3));
    let cursor_y = chunks[0].y;
    f.set_cursor_position((cursor_x, cursor_y));

    let hint_line = Line::from(vec![
        Span::styled("<Enter>", Theme::shortcut_key()),
        Span::styled(":Apply  ", Theme::shortcut_desc()),
        Span::styled("<Esc>", Theme::shortcut_key()),
        Span::styled(":Normal Mode / Clear  ", Theme::shortcut_desc()),
        Span::styled("<C-u>", Theme::shortcut_key()),
        Span::styled(":Clear Input", Theme::shortcut_desc()),
    ]);
    f.render_widget(
        Paragraph::new(hint_line).alignment(Alignment::Center),
        chunks[1],
    );
}

pub fn render_command_modal(f: &mut Frame, app: &App) {
    let area = centered_rect(50, 3, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::White))
        .title(" COMMAND ")
        .title_style(
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        );

    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut input_spans = vec![Span::styled(
        ": ",
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    )];
    let text = styled_field_text(&app.command_input, app.text_cursor, app.field_visual_anchor);
    if let Some(line) = text.lines.into_iter().next() {
        input_spans.extend(line.spans);
    }
    f.render_widget(Paragraph::new(Line::from(input_spans)), inner);

    let cursor_x = inner.x + 2 + (app.text_cursor as u16).min(inner.width.saturating_sub(3));
    let cursor_y = inner.y;
    f.set_cursor_position((cursor_x, cursor_y));
}

/// Splits a char-index cursor over a `\n`-separated buffer into (row, col),
/// both as terminal cell offsets, for placing the terminal cursor.
fn row_col_of_cursor(s: &str, cursor: usize) -> (u16, u16) {
    let mut row: u16 = 0;
    let mut col: u16 = 0;
    for c in s.chars().take(cursor) {
        if c == '\n' {
            row += 1;
            col = 0;
        } else {
            col += 1;
        }
    }
    (row, col)
}

pub fn render_help_modal(f: &mut Frame) {
    let area = centered_rect(74, 18, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" KEYBOARD SHORTCUTS & VIM MOTIONS ")
        .title_style(
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        );

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(inner);

    let left_lines = vec![
        Line::from(Span::styled("NAVIGATION", Theme::text_bold())),
        Line::from(vec![
            Span::styled("  j / Down        ", Theme::shortcut_key()),
            Span::styled("Move cursor down", Theme::shortcut_desc()),
        ]),
        Line::from(vec![
            Span::styled("  k / Up          ", Theme::shortcut_key()),
            Span::styled("Move cursor up", Theme::shortcut_desc()),
        ]),
        Line::from(vec![
            Span::styled("  gg / Home       ", Theme::shortcut_key()),
            Span::styled("Jump to top", Theme::shortcut_desc()),
        ]),
        Line::from(vec![
            Span::styled("  G / End         ", Theme::shortcut_key()),
            Span::styled("Jump to bottom", Theme::shortcut_desc()),
        ]),
        Line::from(vec![
            Span::styled("  <C-d> / <C-u>   ", Theme::shortcut_key()),
            Span::styled("Page down / Page up", Theme::shortcut_desc()),
        ]),
        Line::raw(""),
        Line::from(Span::styled("FILTER & SEARCH", Theme::text_bold())),
        Line::from(vec![
            Span::styled("  /               ", Theme::shortcut_key()),
            Span::styled("Live search / filter", Theme::shortcut_desc()),
        ]),
        Line::from(vec![
            Span::styled("  t               ", Theme::shortcut_key()),
            Span::styled("Cycle tag filters", Theme::shortcut_desc()),
        ]),
        Line::from(vec![
            Span::styled("  <Esc>           ", Theme::shortcut_key()),
            Span::styled("Clear active filter", Theme::shortcut_desc()),
        ]),
    ];

    let right_lines = vec![
        Line::from(Span::styled("TASK ACTIONS", Theme::text_bold())),
        Line::from(vec![
            Span::styled("  <Space>         ", Theme::shortcut_key()),
            Span::styled("Toggle status (Todo/WIP/Done)", Theme::shortcut_desc()),
        ]),
        Line::from(vec![
            Span::styled("  a / o / i       ", Theme::shortcut_key()),
            Span::styled("Add new task", Theme::shortcut_desc()),
        ]),
        Line::from(vec![
            Span::styled("  e / c / <Enter> ", Theme::shortcut_key()),
            Span::styled("Edit selected task", Theme::shortcut_desc()),
        ]),
        Line::from(vec![
            Span::styled("  dd / x          ", Theme::shortcut_key()),
            Span::styled("Delete selected task", Theme::shortcut_desc()),
        ]),
        Line::from(vec![
            Span::styled("  u               ", Theme::shortcut_key()),
            Span::styled("Undo last deletion", Theme::shortcut_desc()),
        ]),
        Line::from(vec![
            Span::styled("  J / K           ", Theme::shortcut_key()),
            Span::styled("Move task down / up", Theme::shortcut_desc()),
        ]),
        Line::from(vec![
            Span::styled("  p / P           ", Theme::shortcut_key()),
            Span::styled("Cycle priority next/prev", Theme::shortcut_desc()),
        ]),
        Line::from(vec![
            Span::styled("  1 / 2 / 3 / 4   ", Theme::shortcut_key()),
            Span::styled("Set Urgent/High/Med/Low", Theme::shortcut_desc()),
        ]),
        Line::from(vec![
            Span::styled("  S / <F5>        ", Theme::shortcut_key()),
            Span::styled("Force manual sync", Theme::shortcut_desc()),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled("  ? / q / <Esc>   ", Theme::shortcut_key()),
            Span::styled("Close cheatsheet / Quit", Theme::shortcut_desc()),
        ]),
    ];

    f.render_widget(Paragraph::new(left_lines), chunks[0]);
    f.render_widget(Paragraph::new(right_lines), chunks[1]);
}

fn centered_rect(percent_x: u16, exact_height: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(r.height.saturating_sub(exact_height) / 2),
            Constraint::Length(exact_height),
            Constraint::Min(0),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
