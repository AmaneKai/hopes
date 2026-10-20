use crate::{action::Action, domain::Priority, ui::KeyResult};
use crossterm::event::KeyCode;

#[derive(Default)]
pub struct Chords {
    g: bool,
    d: bool,
}

impl Chords {
    pub fn press_g(&mut self) -> bool {
        self.d = false;
        let completed = self.g;
        self.g = !self.g;
        completed
    }

    pub fn press_d(&mut self) -> bool {
        self.g = false;
        let completed = self.d;
        self.d = !self.d;
        completed
    }

    pub fn clear(&mut self) {
        self.g = false;
        self.d = false;
    }

    pub fn label(&self) -> Option<&'static str> {
        if self.g {
            Some("g...")
        } else if self.d {
            Some("d...")
        } else {
            None
        }
    }
}

pub fn on_selected(selected: Option<usize>, make: impl Fn(usize) -> Action) -> KeyResult {
    match selected {
        Some(idx) => KeyResult::Emit(make(idx)),
        None => KeyResult::Handled,
    }
}

fn cycle_priority(forward: bool) -> impl Fn(usize) -> Action {
    move |idx| Action::CyclePriority { idx, forward }
}

fn set_priority(priority: Priority) -> impl Fn(usize) -> Action {
    move |idx| Action::SetPriority { idx, priority }
}

pub fn task_command(code: KeyCode, selected: Option<usize>) -> Option<KeyResult> {
    let result = match code {
        KeyCode::Char('e') | KeyCode::Char('c') | KeyCode::Enter => {
            on_selected(selected, Action::OpenEditModal)
        }
        KeyCode::Char('x') => on_selected(selected, Action::DeleteTask),
        KeyCode::Char('J') => on_selected(selected, Action::MoveTaskDown),
        KeyCode::Char('K') => on_selected(selected, Action::MoveTaskUp),
        KeyCode::Char('p') => on_selected(selected, cycle_priority(true)),
        KeyCode::Char('P') => on_selected(selected, cycle_priority(false)),
        KeyCode::Char(c @ '1'..='4') => {
            let priority = Priority::from_digit(c)?;
            on_selected(selected, set_priority(priority))
        }
        _ => return None,
    };
    Some(result)
}
