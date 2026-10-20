use crate::ui::theme::Theme;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    widgets::{Block, BorderType, Borders, Clear, Padding},
};

/// Where a popup sits on screen.
#[derive(Clone, Copy)]
pub enum Anchor {
    /// Vertically centered: heavier context switches (form, help).
    Center,
}

/// Clears a `width` x `height` region (clamped to `area`), draws the rounded popup
/// frame with `title`, and returns the padded inner rect for the content.
/// Popups are the only rounded elements: rounded means "floating above the page".
pub fn popup_frame(
    f: &mut Frame,
    area: Rect,
    title: &str,
    width: u16,
    height: u16,
    anchor: Anchor,
) -> Rect {
    let width = width.min(area.width.saturating_sub(2)).max(1);
    let height = height.min(area.height.saturating_sub(2)).max(1);
    let x = area.x + (area.width - width) / 2;
    let y = match anchor {
        Anchor::Center => area.y + (area.height - height) / 2,
    };
    let rect = Rect::new(x, y, width, height);

    f.render_widget(Clear, rect);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Theme::border())
        .padding(Padding::horizontal(1))
        .title(format!(" {title} "))
        .title_style(Style::default().add_modifier(Modifier::BOLD));
    let inner = block.inner(rect);
    f.render_widget(block, rect);
    inner
}
