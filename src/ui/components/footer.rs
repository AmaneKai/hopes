use crate::{
    app::{App, AppMode},
    ui::theme::Theme,
};
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    text::{Line, Span},
    widgets::Paragraph,
};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let mode_span = match app.mode {
        AppMode::Normal => Span::styled(" NORMAL ", Theme::mode_normal()),
        AppMode::Creating => Span::styled(" INSERT ", Theme::mode_insert()),
        AppMode::Editing => Span::styled(" EDIT ", Theme::mode_edit()),
        AppMode::Filtering => Span::styled(" FILTER ", Theme::mode_filter()),
        AppMode::Help => Span::styled(" HELP ", Theme::mode_help()),
    };

    let mut line1_left = vec![mode_span, Span::raw(" ")];

    if app.pending_g {
        line1_left.push(Span::styled("g...", Theme::status_message()));
        line1_left.push(Span::raw(" "));
    } else if app.pending_d {
        line1_left.push(Span::styled("d...", Theme::status_message()));
        line1_left.push(Span::raw(" "));
    }

    if let Some(msg) = app.get_status_message() {
        line1_left.push(Span::styled(msg, Theme::status_message()));
    }

    let filtered_indices = app.get_filtered_indices();
    let total_filtered = filtered_indices.len();
    let curr_pos = match app.table_state.selected() {
        Some(i) if total_filtered > 0 => i + 1,
        _ => 0,
    };
    let percent_pos = (curr_pos * 100usize)
        .checked_div(total_filtered)
        .unwrap_or(0);

    let line1_right = vec![Span::styled(
        format!("[ {curr_pos} / {total_filtered} ] ({percent_pos}%) "),
        Theme::muted(),
    )];

    let line2_spans = match app.mode {
        AppMode::Normal => vec![
            Span::styled("j/k", Theme::shortcut_key()),
            Span::styled(":Nav ", Theme::shortcut_desc()),
            Span::styled("Tab", Theme::shortcut_key()),
            Span::styled(":Pane ", Theme::shortcut_desc()),
            Span::styled("v", Theme::shortcut_key()),
            Span::styled(":View ", Theme::shortcut_desc()),
            Span::styled("s", Theme::shortcut_key()),
            Span::styled(":Sort ", Theme::shortcut_desc()),
            Span::styled("Space", Theme::shortcut_key()),
            Span::styled(":Toggle ", Theme::shortcut_desc()),
            Span::styled("a/o", Theme::shortcut_key()),
            Span::styled(":New ", Theme::shortcut_desc()),
            Span::styled("e", Theme::shortcut_key()),
            Span::styled(":Edit ", Theme::shortcut_desc()),
            Span::styled("dd", Theme::shortcut_key()),
            Span::styled(":Del ", Theme::shortcut_desc()),
            Span::styled("u", Theme::shortcut_key()),
            Span::styled(":Undo ", Theme::shortcut_desc()),
            Span::styled("J/K", Theme::shortcut_key()),
            Span::styled(":Move ", Theme::shortcut_desc()),
            Span::styled("/", Theme::shortcut_key()),
            Span::styled(":Find ", Theme::shortcut_desc()),
            Span::styled("t", Theme::shortcut_key()),
            Span::styled(":Tag ", Theme::shortcut_desc()),
            Span::styled("?", Theme::shortcut_key()),
            Span::styled(":Help ", Theme::shortcut_desc()),
            Span::styled("q", Theme::shortcut_key()),
            Span::styled(":Quit", Theme::shortcut_desc()),
        ],
        AppMode::Creating | AppMode::Editing => vec![
            Span::styled("<Tab>/<C-j/k>", Theme::shortcut_key()),
            Span::styled(":Next Field ", Theme::shortcut_desc()),
            Span::styled("<Enter>/<C-s>", Theme::shortcut_key()),
            Span::styled(":Save ", Theme::shortcut_desc()),
            Span::styled("<C-w>", Theme::shortcut_key()),
            Span::styled(":Del Word ", Theme::shortcut_desc()),
            Span::styled("<C-u>", Theme::shortcut_key()),
            Span::styled(":Clear ", Theme::shortcut_desc()),
            Span::styled("<Esc>", Theme::shortcut_key()),
            Span::styled(":Cancel", Theme::shortcut_desc()),
        ],
        AppMode::Filtering => vec![
            Span::styled("<Enter>", Theme::shortcut_key()),
            Span::styled(":Apply ", Theme::shortcut_desc()),
            Span::styled("<Esc>", Theme::shortcut_key()),
            Span::styled(":Cancel/Clear ", Theme::shortcut_desc()),
            Span::styled("<C-u>", Theme::shortcut_key()),
            Span::styled(":Clear Input", Theme::shortcut_desc()),
        ],
        AppMode::Help => vec![
            Span::styled("<Esc> / ? / q / <Enter>", Theme::shortcut_key()),
            Span::styled(":Close Cheatsheet", Theme::shortcut_desc()),
        ],
    };

    let p1_left = Paragraph::new(Line::from(line1_left)).alignment(Alignment::Left);
    let p1_right = Paragraph::new(Line::from(line1_right)).alignment(Alignment::Right);
    let p2 = Paragraph::new(Line::from(line2_spans)).alignment(Alignment::Left);

    if area.height >= 2 {
        let r1 = Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: 1,
        };
        let r2 = Rect {
            x: area.x,
            y: area.y + 1,
            width: area.width,
            height: 1,
        };
        f.render_widget(p1_left, r1);
        f.render_widget(p1_right, r1);
        f.render_widget(p2, r2);
    } else {
        f.render_widget(p1_left, area);
        f.render_widget(p1_right, area);
    }
}
