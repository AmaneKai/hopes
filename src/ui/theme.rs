use crate::domain::Priority;
use ratatui::{
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Padding},
};

pub struct Theme;

impl Theme {
    pub fn priority_marker(priority: Priority) -> Span<'static> {
        Span::styled(
            format!("{} ", priority.glyph()),
            Style::default().fg(priority.color()),
        )
    }

    pub fn cursor_prefix(is_cursor: bool, is_current: bool) -> &'static str {
        if is_cursor {
            "> "
        } else if is_current {
            "\u{25cf} "
        } else {
            "  "
        }
    }

    pub fn titled_pane(title: impl Into<String>, focused: bool) -> Block<'static> {
        Self::pane(title.into(), Self::pane_title(focused), focused)
    }

    pub fn pane(title: String, title_style: Style, focused: bool) -> Block<'static> {
        Block::default()
            .borders(Borders::LEFT)
            .border_style(if focused {
                Self::border_focused()
            } else {
                Self::border()
            })
            .padding(Padding::horizontal(1))
            .title(format!(" {title}"))
            .title_style(title_style)
    }

    #[inline(always)]
    pub fn pane_title(focused: bool) -> Style {
        if focused {
            Self::text_bold()
        } else {
            Self::section()
        }
    }

    #[inline(always)]
    pub fn status_bar() -> Style {
        Style::default().bg(Color::Indexed(236))
    }

    #[inline(always)]
    pub fn border() -> Style {
        Style::default().fg(Color::DarkGray)
    }

    #[inline(always)]
    pub fn border_focused() -> Style {
        Style::default().fg(Color::Reset)
    }

    #[inline(always)]
    pub fn highlight() -> Style {
        Style::default().add_modifier(Modifier::BOLD)
    }

    #[inline(always)]
    pub fn section() -> Style {
        Style::default()
            .fg(Color::DarkGray)
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
        Style::default().add_modifier(Modifier::BOLD)
    }

    #[inline(always)]
    pub fn mode(color: Color) -> Style {
        Style::default().fg(color).add_modifier(Modifier::BOLD)
    }

    #[inline(always)]
    pub fn field_insert_badge() -> Style {
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD)
    }

    #[inline(always)]
    pub fn field_normal_badge() -> Style {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    }

    #[inline(always)]
    pub fn field_visual_badge() -> Style {
        Style::default()
            .fg(Color::Magenta)
            .add_modifier(Modifier::BOLD)
    }

    #[inline(always)]
    pub fn field_selection() -> Style {
        Style::default().add_modifier(Modifier::REVERSED)
    }

    #[inline(always)]
    pub fn status_message() -> Style {
        Style::default().fg(Color::Yellow)
    }

    #[inline(always)]
    pub fn shortcut_key() -> Style {
        Style::default().add_modifier(Modifier::BOLD)
    }

    #[inline(always)]
    pub fn shortcut_desc() -> Style {
        Style::default().fg(Color::DarkGray)
    }
}
