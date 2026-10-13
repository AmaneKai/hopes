use crate::{
    app::{App, AppMode, FocusPane},
    ui::theme::Theme,
};
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::Color,
    text::{Line, Span},
    widgets::Paragraph,
};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let (mode_label, mode_color) = match app.mode {
        AppMode::Normal => ("Normal", Color::DarkGray),
        AppMode::Creating => ("Insert", Color::Cyan),
        AppMode::Editing => ("Edit", Color::Magenta),
        AppMode::Filtering => ("Find", Color::Yellow),
        AppMode::Command => ("Command", Color::Reset),
        AppMode::Help => ("Help", Color::Blue),
    };
    let mode_span = Span::styled(format!("  {mode_label}"), Theme::mode(mode_color));

    let mut line1_left = vec![mode_span, Span::raw(" ")];

    let is_text_editing_field = matches!(app.mode, AppMode::Filtering | AppMode::Command)
        || (matches!(app.mode, AppMode::Creating | AppMode::Editing)
            && app.form_field != crate::app::FormField::Priority);
    if is_text_editing_field {
        let (label, style) = if app.field_insert {
            ("-- INSERT --", Theme::field_insert_badge())
        } else if app.field_visual_anchor.is_some() {
            ("-- VISUAL --", Theme::field_visual_badge())
        } else {
            ("-- NORMAL --", Theme::field_normal_badge())
        };
        line1_left.push(Span::styled(label, style));
        line1_left.push(Span::raw(" "));
    }

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
    let line1_right = vec![Span::styled(
        format!("{curr_pos} / {total_filtered}  "),
        Theme::muted(),
    )];

    let line2_spans = match app.mode {
        AppMode::Normal => {
            let mut spans = vec![
                Span::styled("  ? ", Theme::shortcut_key()),
                Span::styled("help   ", Theme::shortcut_desc()),
                Span::styled(": ", Theme::shortcut_key()),
                Span::styled("commands   ", Theme::shortcut_desc()),
                Span::styled("/ ", Theme::shortcut_key()),
                Span::styled("find", Theme::shortcut_desc()),
            ];
            let context: &[(&str, &str)] = match app.focus_pane {
                FocusPane::Main => &[("e", "edit"), ("Space", "toggle"), ("dd", "delete")],
                FocusPane::Nav => &[("Enter", "open"), ("1-5", "jump")],
                FocusPane::Inspector => &[("j/k", "scroll"), ("Tab", "next pane")],
            };
            let context_width: usize = context.iter().map(|(k, d)| k.len() + d.len() + 5).sum();
            let left_width = Line::from(spans.clone()).width();
            let pad = (area.width as usize).saturating_sub(left_width + context_width);
            spans.push(Span::raw(" ".repeat(pad)));
            for (key, desc) in context {
                spans.push(Span::styled(format!("[{key}] "), Theme::shortcut_key()));
                spans.push(Span::styled(format!("{desc}  "), Theme::shortcut_desc()));
            }
            spans
        }
        AppMode::Creating | AppMode::Editing
            if is_text_editing_field && app.field_visual_anchor.is_some() =>
        {
            vec![
                Span::styled("h/j/k/l/w/b/e/0/$", Theme::shortcut_key()),
                Span::styled(":Extend ", Theme::shortcut_desc()),
                Span::styled("d/x", Theme::shortcut_key()),
                Span::styled(":Cut ", Theme::shortcut_desc()),
                Span::styled("y", Theme::shortcut_key()),
                Span::styled(":Yank ", Theme::shortcut_desc()),
                Span::styled("c", Theme::shortcut_key()),
                Span::styled(":Change ", Theme::shortcut_desc()),
                Span::styled("v / <Esc>", Theme::shortcut_key()),
                Span::styled(":Exit Visual", Theme::shortcut_desc()),
            ]
        }
        AppMode::Creating | AppMode::Editing if is_text_editing_field && !app.field_insert => vec![
            Span::styled("h/j/k/l", Theme::shortcut_key()),
            Span::styled(":Move ", Theme::shortcut_desc()),
            Span::styled("w/b/e", Theme::shortcut_key()),
            Span::styled(":Word ", Theme::shortcut_desc()),
            Span::styled("0/$", Theme::shortcut_key()),
            Span::styled(":Line ", Theme::shortcut_desc()),
            Span::styled("v", Theme::shortcut_key()),
            Span::styled(":Visual ", Theme::shortcut_desc()),
            Span::styled("x/dd/D", Theme::shortcut_key()),
            Span::styled(":Delete ", Theme::shortcut_desc()),
            Span::styled("p/P", Theme::shortcut_key()),
            Span::styled(":Paste ", Theme::shortcut_desc()),
            Span::styled("i/a/I/A/o", Theme::shortcut_key()),
            Span::styled(":Insert ", Theme::shortcut_desc()),
            Span::styled("<Esc>", Theme::shortcut_key()),
            Span::styled(":Cancel", Theme::shortcut_desc()),
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
            Span::styled(":Normal Mode / Cancel", Theme::shortcut_desc()),
        ],
        AppMode::Filtering if !app.field_insert => vec![
            Span::styled("h/j/k/l/w/b/e", Theme::shortcut_key()),
            Span::styled(":Motions ", Theme::shortcut_desc()),
            Span::styled("x/dd/D", Theme::shortcut_key()),
            Span::styled(":Delete ", Theme::shortcut_desc()),
            Span::styled("i/a/I/A", Theme::shortcut_key()),
            Span::styled(":Insert ", Theme::shortcut_desc()),
            Span::styled("<Enter>", Theme::shortcut_key()),
            Span::styled(":Apply ", Theme::shortcut_desc()),
            Span::styled("<Esc>", Theme::shortcut_key()),
            Span::styled(":Cancel", Theme::shortcut_desc()),
        ],
        AppMode::Filtering => vec![
            Span::styled("<Enter>", Theme::shortcut_key()),
            Span::styled(":Apply ", Theme::shortcut_desc()),
            Span::styled("<Esc>", Theme::shortcut_key()),
            Span::styled(":Normal Mode / Clear ", Theme::shortcut_desc()),
            Span::styled("<C-u>", Theme::shortcut_key()),
            Span::styled(":Clear Input", Theme::shortcut_desc()),
        ],
        AppMode::Command => vec![
            Span::styled("sync", Theme::shortcut_key()),
            Span::styled(":Force Sync ", Theme::shortcut_desc()),
            Span::styled("q / quit", Theme::shortcut_key()),
            Span::styled(":Exit ", Theme::shortcut_desc()),
            Span::styled("w", Theme::shortcut_key()),
            Span::styled(":Save (auto) ", Theme::shortcut_desc()),
            Span::styled("help", Theme::shortcut_key()),
            Span::styled(":Cheatsheet ", Theme::shortcut_desc()),
            Span::styled("<Enter>", Theme::shortcut_key()),
            Span::styled(":Run ", Theme::shortcut_desc()),
            Span::styled("<Esc>", Theme::shortcut_key()),
            Span::styled(":Cancel", Theme::shortcut_desc()),
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
