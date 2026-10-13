pub mod components;
pub mod panes;
pub mod theme;
pub mod views;

use crate::app::{App, ViewMode};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::Paragraph,
};
use theme::Theme;

/// Width at which the sidebar and inspector are both shown.
const WIDE: u16 = 115;
/// Width at which the inspector is shown.
const MEDIUM: u16 = 80;

/// Splits the terminal into regions and hands each one to its drawing module. This
/// file knows where things go, not how they look.
pub fn render(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(2),
        ])
        .split(f.area());

    components::header::draw_header(f, app, chunks[0]);
    draw_rule(f, chunks[1]);
    draw_body(f, app, chunks[2]);
    components::footer::draw_footer(f, app, chunks[3]);

    components::modal::draw_modal(f, app, f.area());
}

fn draw_body(f: &mut Frame, app: &mut App, body: Rect) {
    if body.width >= WIDE {
        let [sidebar, main, inspector] = Layout::horizontal([
            Constraint::Percentage(18),
            Constraint::Percentage(48),
            Constraint::Percentage(34),
        ])
        .areas(body);

        panes::sidebar::draw_sidebar(f, app, sidebar);
        draw_main(f, app, main);
        panes::inspector::draw_inspector(f, app, inspector);
    } else if body.width >= MEDIUM {
        let [main, inspector] =
            Layout::horizontal([Constraint::Percentage(60), Constraint::Percentage(40)])
                .areas(body);

        draw_main(f, app, main);
        panes::inspector::draw_inspector(f, app, inspector);
    } else {
        draw_main(f, app, body);
    }
}

/// The central pane. Adding a view means one new arm here.
fn draw_main(f: &mut Frame, app: &mut App, area: Rect) {
    match app.view_mode {
        ViewMode::Table => views::table::draw_table(f, app, area),
        ViewMode::Kanban => views::kanban::draw_kanban(f, app, area),
    }
}

/// Hairline under the header.
fn draw_rule(f: &mut Frame, area: Rect) {
    let rule = "\u{2500}".repeat(area.width as usize);
    f.render_widget(Paragraph::new(rule).style(Theme::border()), area);
}
