use crate::{
    action::Action,
    domain::AppState,
    ui::{Component, KeyResult, field::styled_field_text, text_input::TextInput},
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::Rect,
    text::{Line, Span},
    widgets::Paragraph,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
enum Kind {
    #[default]
    Command,
    Find,
}

#[derive(Default)]
pub struct CmdlineComponent {
    kind: Kind,
    buf: String,
    input: TextInput,
}

impl CmdlineComponent {
    pub fn text_input(&self) -> &TextInput {
        &self.input
    }

    fn submit_action(&self) -> Action {
        match self.kind {
            Kind::Command => Action::RunCommand(self.buf.clone()),
            Kind::Find => self.find_action(),
        }
    }

    fn find_action(&self) -> Action {
        let query = self.buf.trim();
        let filter = if query.is_empty() {
            None
        } else {
            Some(query.to_string())
        };
        Action::Batch(vec![Action::SetFilter(filter), Action::CloseModal])
    }

    fn prefix(&self) -> &'static str {
        match self.kind {
            Kind::Command => ":",
            Kind::Find => "/",
        }
    }
}

impl Component for CmdlineComponent {
    fn handle_key(&mut self, key: KeyEvent, _state: &AppState) -> KeyResult {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

        match key.code {
            KeyCode::Enter => KeyResult::Emit(self.submit_action()),
            KeyCode::Esc => {
                if self.input.handle_esc(&mut self.buf) {
                    KeyResult::Handled
                } else {
                    KeyResult::Emit(Action::CloseModal)
                }
            }
            code => {
                self.input.handle_key(&mut self.buf, code, ctrl, false);
                KeyResult::Handled
            }
        }
    }

    fn update(&mut self, action: &Action, state: &AppState) {
        match action {
            Action::OpenCommand => {
                self.kind = Kind::Command;
                self.buf.clear();
                self.input.focus(&self.buf);
            }
            Action::OpenFind => {
                self.kind = Kind::Find;
                self.buf = state.tag_filter.clone().unwrap_or_default();
                self.input.focus(&self.buf);
            }
            _ => {}
        }
    }

    fn render(&mut self, f: &mut Frame, area: Rect, _state: &AppState, _focused: bool) {
        let mut spans = vec![Span::raw(self.prefix())];
        let text = styled_field_text(&self.buf, self.input.cursor, self.input.visual_anchor);
        if let Some(line) = text.lines.into_iter().next() {
            spans.extend(line.spans);
        }
        f.render_widget(Paragraph::new(Line::from(spans)), area);

        let max_x = area.x + area.width.saturating_sub(1);
        f.set_cursor_position(((area.x + 1 + self.input.cursor as u16).min(max_x), area.y));
    }
}
