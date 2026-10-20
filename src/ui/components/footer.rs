use crate::{
    app::AppMode,
    domain::{AppState, FocusPane},
    ui::{text_input::TextInput, theme::Theme},
};
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::Color,
    text::{Line, Span},
    widgets::Paragraph,
};

pub struct FooterCtx<'a> {
    pub mode: AppMode,
    pub position: usize,
    pub input: Option<&'a TextInput>,
    pub pending: Option<&'static str>,
}

pub fn draw_status_line(f: &mut Frame, area: Rect, state: &AppState, ctx: &FooterCtx) {
    let (mode_label, mode_color) = match ctx.mode {
        AppMode::Normal => ("NORMAL", Color::Gray),
        AppMode::Creating => ("INSERT", Color::Cyan),
        AppMode::Editing => ("EDIT", Color::Magenta),
        AppMode::Filtering => ("FIND", Color::Yellow),
        AppMode::Command => ("COMMAND", Color::Reset),
        AppMode::Help => ("HELP", Color::Blue),
    };
    let mode_span = Span::styled(format!("  {mode_label}"), Theme::mode(mode_color));

    let mut left = vec![mode_span, Span::raw(" ")];

    if let Some(input) = ctx.input {
        let (label, style) = if input.insert {
            ("-- INSERT --", Theme::field_insert_badge())
        } else if input.visual_anchor.is_some() {
            ("-- VISUAL --", Theme::field_visual_badge())
        } else {
            ("-- NORMAL --", Theme::field_normal_badge())
        };
        left.push(Span::styled(label, style));
        left.push(Span::raw(" "));
    }

    if let Some(pending) = ctx.pending {
        left.push(Span::styled(pending, Theme::status_message()));
        left.push(Span::raw(" "));
    }

    if let Some(msg) = state.status() {
        left.push(Span::styled(msg, Theme::status_message()));
    }

    let right = vec![Span::styled(
        format!("{} / {}  ", ctx.position, state.filtered_indices().len()),
        Theme::muted(),
    )];

    let bar = Theme::status_bar();
    f.render_widget(
        Paragraph::new(Line::from(left))
            .style(bar)
            .alignment(Alignment::Left),
        area,
    );
    f.render_widget(
        Paragraph::new(Line::from(right))
            .style(bar)
            .alignment(Alignment::Right),
        area,
    );
}

pub fn draw_hints(f: &mut Frame, area: Rect, state: &AppState, ctx: &FooterCtx) {
    let editing = ctx.input;
    let spans = match ctx.mode {
        AppMode::Normal => {
            let mut spans = vec![
                Span::styled("  ? ", Theme::shortcut_key()),
                Span::styled("help   ", Theme::shortcut_desc()),
                Span::styled(": ", Theme::shortcut_key()),
                Span::styled("commands   ", Theme::shortcut_desc()),
                Span::styled("/ ", Theme::shortcut_key()),
                Span::styled("find", Theme::shortcut_desc()),
            ];
            let context: &[(&str, &str)] = match state.focus {
                FocusPane::Main => &[("e", "edit"), ("Space", "toggle"), ("dd", "delete")],
                FocusPane::Nav => &[("Enter", "open"), ("1-5", "jump")],
                FocusPane::Inspector => &[("j/k", "scroll"), ("Tab", "next pane")],
            };
            let context_width: usize = context.iter().map(|(k, d)| k.len() + d.len() + 5).sum();
            let left_width = Line::from(spans.clone()).width();
            let pad = (area.width as usize).saturating_sub(left_width + context_width);
            spans.push(Span::raw(" ".repeat(pad)));
            for (key, desc) in context {
                spans.push(Span::styled(format!("[{key}] "), Theme::shortcut_key()));
                spans.push(Span::styled(format!("{desc}  "), Theme::shortcut_desc()));
            }
            spans
        }
        AppMode::Creating | AppMode::Editing
            if editing.is_some_and(|i| i.visual_anchor.is_some()) =>
        {
            vec![
                Span::styled("h/j/k/l/w/b/e/0/$", Theme::shortcut_key()),
                Span::styled(":Extend ", Theme::shortcut_desc()),
                Span::styled("d/x", Theme::shortcut_key()),
                Span::styled(":Cut ", Theme::shortcut_desc()),
                Span::styled("y", Theme::shortcut_key()),
                Span::styled(":Yank ", Theme::shortcut_desc()),
                Span::styled("c", Theme::shortcut_key()),
                Span::styled(":Change ", Theme::shortcut_desc()),
                Span::styled("v / <Esc>", Theme::shortcut_key()),
                Span::styled(":Exit Visual", Theme::shortcut_desc()),
            ]
        }
        AppMode::Creating | AppMode::Editing if editing.is_some_and(|i| !i.insert) => vec![
            Span::styled("h/j/k/l", Theme::shortcut_key()),
            Span::styled(":Move ", Theme::shortcut_desc()),
            Span::styled("w/b/e", Theme::shortcut_key()),
            Span::styled(":Word ", Theme::shortcut_desc()),
            Span::styled("0/$", Theme::shortcut_key()),
            Span::styled(":Line ", Theme::shortcut_desc()),
            Span::styled("v", Theme::shortcut_key()),
            Span::styled(":Visual ", Theme::shortcut_desc()),
            Span::styled("x/dd/D", Theme::shortcut_key()),
            Span::styled(":Delete ", Theme::shortcut_desc()),
            Span::styled("p/P", Theme::shortcut_key()),
            Span::styled(":Paste ", Theme::shortcut_desc()),
            Span::styled("i/a/I/A/o", Theme::shortcut_key()),
            Span::styled(":Insert ", Theme::shortcut_desc()),
            Span::styled("<Esc>", Theme::shortcut_key()),
            Span::styled(":Cancel", Theme::shortcut_desc()),
        ],
        AppMode::Creating | AppMode::Editing => vec![
            Span::styled("<Tab>/<C-j/k>", Theme::shortcut_key()),
            Span::styled(":Next Field ", Theme::shortcut_desc()),
            Span::styled("<Enter>/<C-s>", Theme::shortcut_key()),
            Span::styled(":Save ", Theme::shortcut_desc()),
            Span::styled("<C-w>", Theme::shortcut_key()),
            Span::styled(":Del Word ", Theme::shortcut_desc()),
            Span::styled("<C-u>", Theme::shortcut_key()),
            Span::styled(":Clear ", Theme::shortcut_desc()),
            Span::styled("<Esc>", Theme::shortcut_key()),
            Span::styled(":Normal Mode / Cancel", Theme::shortcut_desc()),
        ],
        AppMode::Filtering | AppMode::Command => return,
        AppMode::Help => vec![
            Span::styled("<Esc> / ? / q / <Enter>", Theme::shortcut_key()),
            Span::styled(":Close Cheatsheet", Theme::shortcut_desc()),
        ],
    };

    f.render_widget(
        Paragraph::new(Line::from(spans)).alignment(Alignment::Left),
        area,
    );
}
