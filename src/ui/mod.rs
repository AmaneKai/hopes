pub mod components;
pub mod theme;

use crate::app::{App, AppMode, ViewMode};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
};

pub fn render(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(2),
        ])
        .split(f.area());

    components::header::render(f, app, chunks[0]);

    let width = chunks[1].width;

    if width >= 115 {
        let main_split = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(18),
                Constraint::Percentage(48),
                Constraint::Percentage(34),
            ])
            .split(chunks[1]);

        components::nav::render(f, app, main_split[0]);

        match app.view_mode {
            ViewMode::Table => components::list::render(f, app, main_split[1]),
            ViewMode::Kanban => components::kanban::render(f, app, main_split[1]),
        }

        components::inspector::render(f, app, main_split[2]);
    } else if width >= 80 {
        let main_split = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(chunks[1]);

        match app.view_mode {
            ViewMode::Table => components::list::render(f, app, main_split[0]),
            ViewMode::Kanban => components::kanban::render(f, app, main_split[0]),
        }

        components::inspector::render(f, app, main_split[1]);
    } else {
        match app.view_mode {
            ViewMode::Table => components::list::render(f, app, chunks[1]),
            ViewMode::Kanban => components::kanban::render(f, app, chunks[1]),
        }
    }

    components::footer::render(f, app, chunks[2]);

    match app.mode {
        AppMode::Creating | AppMode::Editing => components::modal::render_form_modal(f, app),
        AppMode::Filtering => components::modal::render_filter_modal(f, app),
        AppMode::Help => components::modal::render_help_modal(f),
        AppMode::Normal => {}
    }
}
