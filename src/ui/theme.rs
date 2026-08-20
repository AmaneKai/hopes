use ratatui::style::{Color, Modifier, Style};

pub struct Theme;

impl Theme {
    #[inline(always)]
    pub fn border() -> Style {
        Style::default().fg(Color::Rgb(75, 82, 99))
    }

    #[inline(always)]
    pub fn border_focused() -> Style {
        Style::default().fg(Color::Cyan)
    }

    #[inline(always)]
    pub fn header_title() -> Style {
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    }

    #[inline(always)]
    pub fn header_brand() -> Style {
        Style::default()
            .bg(Color::Rgb(32, 40, 54))
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    }

    #[inline(always)]
    pub fn highlight() -> Style {
        Style::default()
            .bg(Color::Rgb(38, 44, 58))
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    }

    #[inline(always)]
    pub fn table_header() -> Style {
        Style::default()
            .fg(Color::Rgb(150, 160, 180))
            .add_modifier(Modifier::BOLD)
    }

    #[inline(always)]
    pub fn tag() -> Style {
        Style::default().fg(Color::Cyan)
    }

    #[inline(always)]
    pub fn completed() -> Style {
        Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::CROSSED_OUT)
    }

    #[inline(always)]
    pub fn muted() -> Style {
        Style::default().fg(Color::DarkGray)
    }

    #[inline(always)]
    pub fn text_bold() -> Style {
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    }

    #[inline(always)]
    pub fn mode_normal() -> Style {
        Style::default()
            .bg(Color::Green)
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD)
    }

    #[inline(always)]
    pub fn mode_insert() -> Style {
        Style::default()
            .bg(Color::Cyan)
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD)
    }

    #[inline(always)]
    pub fn mode_edit() -> Style {
        Style::default()
            .bg(Color::Magenta)
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD)
    }

    #[inline(always)]
    pub fn mode_filter() -> Style {
        Style::default()
            .bg(Color::Yellow)
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD)
    }

    #[inline(always)]
    pub fn mode_help() -> Style {
        Style::default()
            .bg(Color::Blue)
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    }

    #[inline(always)]
    pub fn status_message() -> Style {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    }

    #[inline(always)]
    pub fn shortcut_key() -> Style {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    }

    #[inline(always)]
    pub fn shortcut_desc() -> Style {
        Style::default().fg(Color::Rgb(180, 185, 200))
    }
}
