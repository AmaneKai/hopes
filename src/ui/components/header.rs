use crate::{
    app::{App, ViewMode},
    sync::SyncStatus,
    ui::theme::Theme,
};
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

/// Breadcrumb on the left (`Hopes / Tasks / All`), completion and sync state on the
/// right. A hairline rule is drawn on the row below by the layout orchestrator.
pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let stats = app.stats();

    let section = match app.view_mode {
        ViewMode::Table => "Tasks",
        ViewMode::Kanban => "Board",
    };
    let current = match &app.tag_filter {
        Some(tag) => format!("#{}", tag.strip_prefix('#').unwrap_or(tag)),
        None => app.nav_section.label().to_string(),
    };

    let sep = Span::styled(" / ", Theme::muted());
    let mut left = vec![
        Span::raw("  "),
        Span::styled("Hopes", Theme::muted()),
        sep.clone(),
        Span::styled(section, Theme::muted()),
        sep,
        Span::styled(current, Theme::text_bold()),
    ];
    if app.tag_filter.is_some() {
        let shown = app.get_filtered_indices().len();
        left.push(Span::styled(
            format!("  {shown}/{}", stats.total),
            Theme::muted(),
        ));
    }

    let mut right = vec![Span::styled(
        format!("{}% done", stats.percent),
        Theme::muted(),
    )];
    if let Some(sync) = sync_span(&app.sync_status) {
        right.push(Span::raw("   "));
        right.push(sync);
    }
    right.push(Span::raw("  "));

    f.render_widget(
        Paragraph::new(Line::from(left)).alignment(Alignment::Left),
        area,
    );
    f.render_widget(
        Paragraph::new(Line::from(right)).alignment(Alignment::Right),
        area,
    );
}

fn sync_span(status: &SyncStatus) -> Option<Span<'static>> {
    let (text, style) = match status {
        SyncStatus::Disabled => return None,
        SyncStatus::Idle => ("Idle".to_string(), Theme::muted()),
        SyncStatus::Syncing(msg) => (msg.clone(), Style::default().fg(Color::Yellow)),
        SyncStatus::Synced | SyncStatus::UpdatedRemoteData => {
            ("Synced".to_string(), Theme::muted())
        }
        SyncStatus::Offline => ("Offline".to_string(), Theme::muted()),
        SyncStatus::Conflict(_) => (
            "Conflict".to_string(),
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ),
        SyncStatus::Error(_) => (
            "Sync error".to_string(),
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ),
    };
    Some(Span::styled(format!("\u{2601} {text}"), style))
}
