use super::{
    field::{field_or_placeholder, row_col_of_cursor},
    frame::{Anchor, popup_frame},
};
use crate::{
    app::{App, AppMode, FormField},
    models::Priority,
    ui::theme::Theme,
};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

const LABEL_WIDTH: u16 = 10;
const NOTES_ROWS: u16 = 3;
const FORM_HEIGHT: u16 = 14;

/// Cyan marker in the gutter of the focused field.
fn gutter(focused: bool) -> Paragraph<'static> {
    let marker = if focused { "\u{258d} " } else { "  " };
    Paragraph::new(Span::styled(marker, Style::default().fg(Color::Cyan)))
}

fn label(name: &'static str, focused: bool) -> Paragraph<'static> {
    let style = if focused {
        Theme::text_bold()
    } else {
        Theme::muted()
    };
    Paragraph::new(Span::styled(name, style))
}

/// Thin rule under a field; default foreground when that field has focus.
fn underline(f: &mut Frame, area: Rect, focused: bool) {
    let style = if focused {
        Theme::border_focused()
    } else {
        Theme::border()
    };
    f.render_widget(
        Paragraph::new("\u{2500}".repeat(area.width as usize)).style(style),
        area,
    );
}

/// Splits a form row into gutter, label and value areas.
fn split_row(row: Rect) -> (Rect, Rect, Rect) {
    let [g, l, v] = Layout::horizontal([
        Constraint::Length(2),
        Constraint::Length(LABEL_WIDTH),
        Constraint::Min(1),
    ])
    .areas(row);
    (g, l, v)
}

pub(super) fn draw_form_modal(f: &mut Frame, app: &App, area: Rect) {
    let title = if app.mode == AppMode::Editing {
        "Edit task"
    } else {
        "New task"
    };
    let inner = popup_frame(f, area, title, 68, FORM_HEIGHT, Anchor::Center);

    let [
        _,
        title_row,
        title_rule,
        notes_row,
        notes_rule,
        _,
        prio_row,
        tags_row,
        _,
        hint_row,
    ] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(NOTES_ROWS),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(inner);

    let title_focused = app.form_field == FormField::Title;
    let notes_focused = app.form_field == FormField::Description;
    let prio_focused = app.form_field == FormField::Priority;
    let tags_focused = app.form_field == FormField::Tags;

    // Title
    let (g, l, v) = split_row(title_row);
    f.render_widget(gutter(title_focused), g);
    f.render_widget(label("Title", title_focused), l);
    f.render_widget(
        Paragraph::new(field_or_placeholder(
            &app.form_title,
            app.text_cursor,
            title_focused.then_some(app.field_visual_anchor).flatten(),
            "Untitled",
        )),
        v,
    );
    underline(f, title_rule, title_focused);
    if title_focused {
        let x = v.x + (app.text_cursor as u16).min(v.width.saturating_sub(1));
        f.set_cursor_position((x, v.y));
    }

    // Notes (multi-line)
    let (g, l, v) = split_row(notes_row);
    f.render_widget(gutter(notes_focused), g);
    f.render_widget(label("Notes", notes_focused), l);
    f.render_widget(
        Paragraph::new(field_or_placeholder(
            &app.form_description,
            app.text_cursor,
            notes_focused.then_some(app.field_visual_anchor).flatten(),
            "Add notes...",
        )),
        v,
    );
    underline(f, notes_rule, notes_focused);
    if notes_focused {
        let (row, col) = row_col_of_cursor(&app.form_description, app.text_cursor);
        let x = v.x + col.min(v.width.saturating_sub(1));
        let y = v.y + row.min(v.height.saturating_sub(1));
        f.set_cursor_position((x, y));
    }

    // Priority
    let (g, l, v) = split_row(prio_row);
    f.render_widget(gutter(prio_focused), g);
    f.render_widget(label("Priority", prio_focused), l);
    let mut spans = Vec::with_capacity(Priority::ALL.len() * 2);
    for p in Priority::ALL {
        let mut style = Style::default().fg(p.color());
        if p == app.form_priority {
            style = style.add_modifier(Modifier::BOLD | Modifier::UNDERLINED);
        }
        spans.push(Span::styled(format!("{} {}", p.glyph(), p.label()), style));
        spans.push(Span::raw("   "));
    }
    f.render_widget(Paragraph::new(Line::from(spans)), v);

    // Tags
    let (g, l, v) = split_row(tags_row);
    f.render_widget(gutter(tags_focused), g);
    f.render_widget(label("Tags", tags_focused), l);
    f.render_widget(
        Paragraph::new(field_or_placeholder(
            &app.form_tags,
            app.text_cursor,
            tags_focused.then_some(app.field_visual_anchor).flatten(),
            "design, ui",
        )),
        v,
    );
    if tags_focused {
        let x = v.x + (app.text_cursor as u16).min(v.width.saturating_sub(1));
        f.set_cursor_position((x, v.y));
    }

    let hint = Line::from(vec![
        Span::styled("[Ctrl-s] ", Theme::shortcut_key()),
        Span::styled("save   ", Theme::shortcut_desc()),
        Span::styled("[Tab] ", Theme::shortcut_key()),
        Span::styled("next   ", Theme::shortcut_desc()),
        Span::styled("[Esc] ", Theme::shortcut_key()),
        Span::styled("cancel", Theme::shortcut_desc()),
    ]);
    f.render_widget(Paragraph::new(hint), hint_row);
}
