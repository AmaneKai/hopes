use super::frame::{Anchor, popup_frame};
use crate::{app::App, ui::theme::Theme};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    text::{Line, Span},
    widgets::Paragraph,
};

const KEY_WIDTH: usize = 16;
const WIDTH: u16 = 64;

/// (section, [(keys, description)])
const SECTIONS: &[(&str, &[(&str, &str)])] = &[
    (
        "Navigation",
        &[
            ("j / Down", "Move down"),
            ("k / Up", "Move up"),
            ("gg / Home", "Jump to top"),
            ("G / End", "Jump to bottom"),
            ("Ctrl-d / Ctrl-u", "Page down / up"),
            ("Tab", "Next pane"),
            ("v", "Switch table / board"),
            ("s", "Cycle sort"),
        ],
    ),
    (
        "Search",
        &[
            ("/", "Find tasks"),
            ("t", "Cycle tag filter"),
            ("Esc", "Clear filter"),
            (":", "Command line"),
        ],
    ),
    (
        "Tasks",
        &[
            ("Space", "Toggle status"),
            ("a / o / i", "Add task"),
            ("e / c / Enter", "Edit task"),
            ("dd / x", "Delete task"),
            ("u", "Undo delete"),
            ("J / K", "Move task down / up"),
            ("p / P", "Next / previous priority"),
            ("1 2 3 4", "Urgent / High / Medium / Low"),
            ("S / F5", "Sync now"),
        ],
    ),
    (
        "Editing",
        &[
            ("Tab / Ctrl-j", "Next field"),
            ("Shift-Tab / Ctrl-k", "Previous field"),
            ("Ctrl-s", "Save"),
            ("Esc", "Normal mode, then cancel"),
            ("h j k l w b e", "Move"),
            ("0 / $", "Line start / end"),
            ("i a I A o", "Insert"),
            ("v", "Visual selection"),
            ("x dd D", "Delete"),
            ("y p P", "Yank / paste"),
            ("Ctrl-w / Ctrl-u", "Delete word / clear"),
        ],
    ),
    (
        "General",
        &[("? / q / Esc", "Close help"), ("Ctrl-c", "Cancel form")],
    ),
];

fn content() -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    for (i, (section, rows)) in SECTIONS.iter().enumerate() {
        if i > 0 {
            lines.push(Line::raw(""));
        }
        lines.push(Line::from(Span::styled(*section, Theme::section())));
        for (keys, desc) in *rows {
            lines.push(Line::from(vec![
                Span::styled(format!("{keys:<KEY_WIDTH$}"), Theme::shortcut_key()),
                Span::styled(*desc, Theme::shortcut_desc()),
            ]));
        }
    }
    lines
}

/// Popup height for a screen of `screen_height` rows: content plus borders, a
/// spacer and the hint row, capped to the screen.
fn popup_height(content_len: usize, screen_height: u16) -> u16 {
    (content_len as u16 + 4).min(screen_height.saturating_sub(2))
}

/// Rows of content visible at once.
fn visible_rows(content_len: usize, screen_height: u16) -> usize {
    popup_height(content_len, screen_height).saturating_sub(4) as usize
}

/// Largest useful scroll offset for a screen of `screen_height` rows.
pub fn max_scroll(screen_height: u16) -> u16 {
    let len = content().len();
    len.saturating_sub(visible_rows(len, screen_height)) as u16
}

pub(super) fn draw_help_modal(f: &mut Frame, app: &App, area: Rect) {
    let lines = content();
    let height = popup_height(lines.len(), area.height);
    let inner = popup_frame(f, area, "Keyboard shortcuts", WIDTH, height, Anchor::Center);

    let [body, _, hint_row] = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(inner);

    let max = lines.len().saturating_sub(body.height as usize) as u16;
    let scroll = app.help_scroll.min(max);
    f.render_widget(Paragraph::new(lines).scroll((scroll, 0)), body);

    let hint = Line::from(vec![
        Span::styled("[j/k] ", Theme::shortcut_key()),
        Span::styled("scroll   ", Theme::shortcut_desc()),
        Span::styled("[Esc] ", Theme::shortcut_key()),
        Span::styled("close", Theme::shortcut_desc()),
    ]);
    f.render_widget(Paragraph::new(hint), hint_row);
}
