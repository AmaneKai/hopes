pub mod components;
pub mod field;
pub mod keys;
pub mod layout;
pub mod popup;
pub mod text_input;
pub mod theme;

use crate::{action::Action, domain::AppState};
use crossterm::event::KeyEvent;
use ratatui::{Frame, layout::Rect};

pub enum KeyResult {
    Ignored,
    Handled,
    Emit(Action),
}

pub trait Component {
    fn handle_key(&mut self, _key: KeyEvent, _state: &AppState) -> KeyResult {
        KeyResult::Ignored
    }

    fn update(&mut self, _action: &Action, _state: &AppState) {}

    fn render(&mut self, f: &mut Frame, area: Rect, state: &AppState, focused: bool);
}

pub trait TaskView: Component {
    fn selected_index(&self, state: &AppState) -> Option<usize>;
    fn pending_label(&self) -> Option<&'static str>;
    fn clear_pending(&mut self);
}
