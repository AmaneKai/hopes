use crate::ui::theme::Theme;
use ratatui::{
    style::{Color, Style},
    text::{Line, Span, Text},
};

/// Renders `text` as a `Text`, highlighting the [lo, hi] char range (if any
/// Visual-mode selection is active) with `Theme::field_selection()`. Splits
/// correctly across embedded `\n`s so multi-line Description selections work
/// too — Paragraph needs one `Line` per visual row, not one long string.
pub(super) fn styled_field_text(
    text: &str,
    cursor: usize,
    visual_anchor: Option<usize>,
) -> Text<'static> {
    let base_style = Style::default().fg(Color::Reset);
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

/// Splits a char-index cursor over a `\n`-separated buffer into (row, col),
/// both as terminal cell offsets, for placing the terminal cursor.
pub(super) fn row_col_of_cursor(s: &str, cursor: usize) -> (u16, u16) {
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

/// Field contents, or a muted placeholder while the field is empty.
pub(super) fn field_or_placeholder(
    value: &str,
    cursor: usize,
    visual_anchor: Option<usize>,
    placeholder: &'static str,
) -> Text<'static> {
    if value.is_empty() {
        Text::from(Span::styled(placeholder, Theme::muted()))
    } else {
        styled_field_text(value, cursor, visual_anchor)
    }
}
