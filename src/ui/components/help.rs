use crate::{
    action::Action,
    domain::AppState,
    ui::{Component, KeyResult, popup::popup_frame, theme::Theme},
};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    text::{Line, Span},
    widgets::Paragraph,
};

const KEY_WIDTH: usize = 16;
const WIDTH: u16 = 64;

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
            ("s", "Cycle sort (incl. due date)"),
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
            ("Due field", "fri, tomorrow, +3d, 2w, oct 25"),
            ("Repeat field", "weekly, every fri, 2w, weekdays"),
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

fn popup_height(content_len: usize, screen_height: u16) -> u16 {
    (content_len as u16 + 4).min(screen_height.saturating_sub(2))
}

#[derive(Default)]
pub struct HelpComponent {
    scroll: u16,
    max_scroll: u16,
}

impl Component for HelpComponent {
    fn handle_key(&mut self, key: KeyEvent, _state: &AppState) -> KeyResult {
        match key.code {
            KeyCode::Esc
            | KeyCode::Char('?')
            | KeyCode::Char('q')
            | KeyCode::Enter
            | KeyCode::Char(' ') => return KeyResult::Emit(Action::CloseModal),
            KeyCode::Char('j') | KeyCode::Down => {
                self.scroll = (self.scroll + 1).min(self.max_scroll);
            }
            KeyCode::Char('k') | KeyCode::Up => self.scroll = self.scroll.saturating_sub(1),
            _ => {}
        }
        KeyResult::Handled
    }

    fn update(&mut self, action: &Action, _state: &AppState) {
        if matches!(action, Action::OpenHelp | Action::CloseModal) {
            self.scroll = 0;
        }
    }

    fn render(&mut self, f: &mut Frame, area: Rect, _state: &AppState, _focused: bool) {
        let lines = content();
        let height = popup_height(lines.len(), area.height);
        let inner = popup_frame(f, area, "Keyboard shortcuts", WIDTH, height);

        let [body, _, hint_row] = Layout::vertical([
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .areas(inner);

        self.max_scroll = lines.len().saturating_sub(body.height as usize) as u16;
        self.scroll = self.scroll.min(self.max_scroll);
        f.render_widget(Paragraph::new(lines).scroll((self.scroll, 0)), body);

        let hint = Line::from(vec![
            Span::styled("[j/k] ", Theme::shortcut_key()),
            Span::styled("scroll   ", Theme::shortcut_desc()),
            Span::styled("[Esc] ", Theme::shortcut_key()),
            Span::styled("close", Theme::shortcut_desc()),
        ]);
        f.render_widget(Paragraph::new(hint), hint_row);
    }
}
