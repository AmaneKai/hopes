use crate::ui::theme::Theme;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    widgets::Paragraph,
};

const WIDE: u16 = 115;
const MEDIUM: u16 = 80;

pub struct Screen {
    pub header: Rect,
    pub rule: Rect,
    pub body: Rect,
    pub status: Rect,
    pub bottom: Rect,
}

pub fn split_screen(area: Rect) -> Screen {
    let [header, rule, body, status, bottom] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(area);
    Screen {
        header,
        rule,
        body,
        status,
        bottom,
    }
}

pub struct Body {
    pub sidebar: Option<Rect>,
    pub main: Rect,
    pub inspector: Option<Rect>,
}

pub fn split_body(body: Rect) -> Body {
    if body.width >= WIDE {
        let [sidebar, main, inspector] = Layout::horizontal([
            Constraint::Percentage(18),
            Constraint::Percentage(48),
            Constraint::Percentage(34),
        ])
        .areas(body);
        Body {
            sidebar: Some(sidebar),
            main,
            inspector: Some(inspector),
        }
    } else if body.width >= MEDIUM {
        let [main, inspector] =
            Layout::horizontal([Constraint::Percentage(60), Constraint::Percentage(40)])
                .areas(body);
        Body {
            sidebar: None,
            main,
            inspector: Some(inspector),
        }
    } else {
        Body {
            sidebar: None,
            main: body,
            inspector: None,
        }
    }
}

pub fn draw_rule(f: &mut Frame, area: Rect) {
    let rule = "\u{2500}".repeat(area.width as usize);
    f.render_widget(Paragraph::new(rule).style(Theme::border()), area);
}
