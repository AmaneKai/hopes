mod field;
mod form;
mod frame;
mod help;

use crate::app::{App, AppMode};
use ratatui::{Frame, layout::Rect};

pub use field::styled_field_text;
pub use help::max_scroll as help_max_scroll;

/// Draws whichever popup the current app mode calls for (nothing in Normal mode).
pub fn draw_modal(f: &mut Frame, app: &App, area: Rect) {
    match app.mode {
        AppMode::Creating | AppMode::Editing => form::draw_form_modal(f, app, area),
        AppMode::Help => help::draw_help_modal(f, app, area),
        AppMode::Filtering | AppMode::Command | AppMode::Normal => {}
    }
}
