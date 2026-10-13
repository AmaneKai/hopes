use super::{
    field::field_or_placeholder,
    frame::{Anchor, popup_frame},
};
use crate::{app::App, ui::theme::Theme};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    text::{Line, Span},
    widgets::Paragraph,
};

pub(super) fn draw_filter_modal(f: &mut Frame, app: &App, area: Rect) {
    let inner = popup_frame(f, area, "Find", 60, 4, Anchor::Top);
    let [input_row, hint_row] =
        Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).areas(inner);

    let count = format!(
        "{} of {}",
        app.get_filtered_indices().len(),
        app.items.len()
    );
    let [text_area, count_area] = Layout::horizontal([
        Constraint::Min(1),
        Constraint::Length(count.len() as u16 + 1),
    ])
    .areas(input_row);

    let mut spans = vec![Span::styled("/ ", Theme::text_bold())];
    let text = field_or_placeholder(
        &app.filter_input,
        app.text_cursor,
        app.field_visual_anchor,
        "search tasks, tags...",
    );
    if let Some(line) = text.lines.into_iter().next() {
        spans.extend(line.spans);
    }
    f.render_widget(Paragraph::new(Line::from(spans)), text_area);
    f.render_widget(
        Paragraph::new(Span::styled(count, Theme::muted())).alignment(Alignment::Right),
        count_area,
    );

    let cursor_x =
        text_area.x + 2 + (app.text_cursor as u16).min(text_area.width.saturating_sub(3));
    f.set_cursor_position((cursor_x, text_area.y));

    let hint = Line::from(vec![
        Span::styled("[Enter] ", Theme::shortcut_key()),
        Span::styled("apply  ", Theme::shortcut_desc()),
        Span::styled("[Esc] ", Theme::shortcut_key()),
        Span::styled("clear", Theme::shortcut_desc()),
    ]);
    f.render_widget(Paragraph::new(hint), hint_row);
}
