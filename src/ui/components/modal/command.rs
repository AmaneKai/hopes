use super::{
    field::styled_field_text,
    frame::{Anchor, popup_frame},
};
use crate::{app::App, ui::theme::Theme};
use ratatui::{
    Frame,
    layout::Rect,
    text::{Line, Span},
    widgets::Paragraph,
};

const SUGGESTIONS: [(&str, &str); 4] = [
    ("sync", "Force a sync"),
    ("w", "Save (auto)"),
    ("q", "Quit"),
    ("help", "Open help"),
];

pub(super) fn draw_command_modal(f: &mut Frame, app: &App, area: Rect) {
    let empty = app.command_input.is_empty();
    let height = if empty {
        3 + SUGGESTIONS.len() as u16 + 1
    } else {
        3
    };
    let inner = popup_frame(f, area, "Command", 50, height, Anchor::Top);

    let mut spans = vec![Span::styled(": ", Theme::text_bold())];
    if empty {
        spans.push(Span::styled("type a command", Theme::muted()));
    } else {
        let text = styled_field_text(&app.command_input, app.text_cursor, app.field_visual_anchor);
        if let Some(line) = text.lines.into_iter().next() {
            spans.extend(line.spans);
        }
    }
    let mut lines = vec![Line::from(spans)];

    if empty {
        lines.push(Line::raw(""));
        for (cmd, desc) in SUGGESTIONS {
            lines.push(Line::from(vec![
                Span::styled(format!("{cmd:<7}"), Theme::shortcut_key()),
                Span::styled(desc, Theme::shortcut_desc()),
            ]));
        }
    }
    f.render_widget(Paragraph::new(lines), inner);

    let cursor_x = inner.x + 2 + (app.text_cursor as u16).min(inner.width.saturating_sub(3));
    f.set_cursor_position((cursor_x, inner.y));
}
