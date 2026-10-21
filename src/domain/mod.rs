pub mod audit;
pub mod due;
pub mod item;
pub mod priority;
pub mod status;

pub use audit::AuditLog;
pub use item::Item;
pub use priority::Priority;
pub use status::Status;

use crate::systems::sync::SyncStatus;
use chrono::{Local, NaiveDate};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

pub fn snippet(title: &str, max: usize) -> String {
    if title.chars().count() <= max {
        title.to_string()
    } else {
        let truncated: String = title.chars().take(max).collect();
        format!("{truncated}...")
    }
}

#[inline(always)]
fn contains_ignore_case(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    if needle.is_ascii() && haystack.is_ascii() {
        let h = haystack.as_bytes();
        let n = needle.as_bytes();
        if n.len() > h.len() {
            return false;
        }
        for i in 0..=(h.len() - n.len()) {
            if h[i..i + n.len()].eq_ignore_ascii_case(n) {
                return true;
            }
        }
        return false;
    }
    haystack.to_lowercase().contains(&needle.to_lowercase())
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ViewMode {
    Table,
    Kanban,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum FocusPane {
    Nav,
    Main,
    Inspector,
}

impl FocusPane {
    #[inline(always)]
    pub fn next(&self) -> Self {
        match self {
            Self::Nav => Self::Main,
            Self::Main => Self::Inspector,
            Self::Inspector => Self::Nav,
        }
    }

    #[inline(always)]
    pub fn prev(&self) -> Self {
        match self {
            Self::Nav => Self::Inspector,
            Self::Main => Self::Nav,
            Self::Inspector => Self::Main,
        }
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Serialize, Deserialize)]
pub enum SortMode {
    Natural,
    Priority,
    Status,
    Due,
    Title,
}

impl SortMode {
    #[inline(always)]
    pub fn next(&self) -> Self {
        match self {
            Self::Natural => Self::Priority,
            Self::Priority => Self::Status,
            Self::Status => Self::Due,
            Self::Due => Self::Title,
            Self::Title => Self::Natural,
        }
    }

    #[inline(always)]
    pub fn label(&self) -> &'static str {
        match self {
            Self::Natural => "Natural",
            Self::Priority => "Priority",
            Self::Status => "Status",
            Self::Due => "Due",
            Self::Title => "Title",
        }
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Serialize, Deserialize)]
pub enum NavSection {
    All,
    Todo,
    InProgress,
    Complete,
    Urgent,
    Overdue,
}

impl NavSection {
    pub const ALL: [NavSection; 6] = [
        NavSection::All,
        NavSection::Todo,
        NavSection::InProgress,
        NavSection::Complete,
        NavSection::Urgent,
        NavSection::Overdue,
    ];

    #[inline(always)]
    pub fn label(&self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Todo => "To Do",
            Self::InProgress => "In Progress",
            Self::Complete => "Completed",
            Self::Urgent => "Urgent",
            Self::Overdue => "Overdue",
        }
    }
}

#[derive(Clone, Copy, Default)]
pub struct WorkspaceStats {
    pub total: usize,
    pub todo: usize,
    pub in_progress: usize,
    pub complete: usize,
    pub percent: usize,
    pub urgent: usize,
    pub overdue: usize,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
}

const STATUS_TTL: Duration = Duration::from_secs(4);

pub struct AppState {
    pub items: Vec<Item>,
    pub view_mode: ViewMode,
    pub focus: FocusPane,
    pub sort_mode: SortMode,
    pub nav_section: NavSection,
    pub tag_filter: Option<String>,
    pub last_deleted: Option<(usize, Item)>,
    pub selection: Option<usize>,
    pub sync_status: SyncStatus,
    pub audit: AuditLog,
    status_message: Option<(String, Instant)>,

    filtered_indices: Vec<usize>,
    stats: WorkspaceStats,
    tags: Vec<(String, usize)>,
    kanban_cols: [Vec<usize>; 3],
}

impl AppState {
    pub fn new(items: Vec<Item>, sync_status: SyncStatus) -> Self {
        let mut state = Self {
            items,
            view_mode: ViewMode::Table,
            focus: FocusPane::Main,
            sort_mode: SortMode::Natural,
            nav_section: NavSection::All,
            tag_filter: None,
            last_deleted: None,
            selection: None,
            sync_status,
            audit: AuditLog::default(),
            status_message: None,
            filtered_indices: Vec::new(),
            stats: WorkspaceStats::default(),
            tags: Vec::new(),
            kanban_cols: [Vec::new(), Vec::new(), Vec::new()],
        };
        state.refresh_cache();
        state
    }

    #[inline(always)]
    pub fn filtered_indices(&self) -> &[usize] {
        &self.filtered_indices
    }

    #[inline(always)]
    pub fn kanban_column(&self, status: Status) -> &[usize] {
        match status {
            Status::Todo => &self.kanban_cols[0],
            Status::InProgress => &self.kanban_cols[1],
            Status::Complete => &self.kanban_cols[2],
        }
    }

    #[inline(always)]
    pub fn tags(&self) -> &[(String, usize)] {
        &self.tags
    }

    #[inline(always)]
    pub fn stats(&self) -> WorkspaceStats {
        self.stats
    }

    #[inline(always)]
    pub fn selected_item(&self) -> Option<&Item> {
        self.items.get(self.selection?)
    }

    #[inline(always)]
    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = Some((msg.into(), Instant::now()));
    }

    #[inline(always)]
    pub fn status(&self) -> Option<&str> {
        self.status_message
            .as_ref()
            .filter(|(_, at)| at.elapsed() < STATUS_TTL)
            .map(|(msg, _)| msg.as_str())
    }

    #[inline(always)]
    pub fn log(&mut self, message: impl Into<String>) {
        self.audit.push(message);
    }

    pub fn refresh_cache(&mut self) {
        self.refresh_summary();
        self.refresh_visible();
    }

    fn refresh_summary(&mut self) {
        let mut stats = WorkspaceStats {
            total: self.items.len(),
            ..WorkspaceStats::default()
        };
        let mut columns: [Vec<usize>; 3] = [Vec::new(), Vec::new(), Vec::new()];
        let mut tag_counts: HashMap<String, usize> = HashMap::new();

        let today = Local::now().date_naive();
        for (i, item) in self.items.iter().enumerate() {
            if item.is_overdue(today) {
                stats.overdue += 1;
            }
            count_status(&mut stats, item.status);
            count_priority(&mut stats, item.priority);
            columns[column_of(item.status)].push(i);
            for tag in &item.tags {
                *tag_counts.entry(tag.clone()).or_insert(0) += 1;
            }
        }

        stats.percent = (stats.complete * 100usize)
            .checked_div(stats.total)
            .unwrap_or(0);
        self.stats = stats;
        self.kanban_cols = columns;

        let mut tags: Vec<(String, usize)> = tag_counts.into_iter().collect();
        tags.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        self.tags = tags;
    }

    fn refresh_visible(&mut self) {
        let mut indices: Vec<usize> = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| self.is_visible(item))
            .map(|(i, _)| i)
            .collect();

        match self.sort_mode {
            SortMode::Natural => {}
            SortMode::Priority => {
                indices.sort_by_key(|&i| priority_rank(self.items[i].priority));
            }
            SortMode::Status => indices.sort_by_key(|&i| column_of(self.items[i].status)),
            SortMode::Due => indices.sort_by_key(|&i| {
                let item = &self.items[i];
                (item.due.is_none(), item.due)
            }),
            SortMode::Title => indices.sort_by_key(|&i| self.items[i].title.to_lowercase()),
        }

        self.filtered_indices = indices;
    }

    fn is_visible(&self, item: &Item) -> bool {
        if !matches_section(self.nav_section, item, Local::now().date_naive()) {
            return false;
        }
        match &self.tag_filter {
            Some(filter) => matches_query(item, filter),
            None => true,
        }
    }
}

fn count_status(stats: &mut WorkspaceStats, status: Status) {
    match status {
        Status::Todo => stats.todo += 1,
        Status::InProgress => stats.in_progress += 1,
        Status::Complete => stats.complete += 1,
    }
}

fn count_priority(stats: &mut WorkspaceStats, priority: Priority) {
    match priority {
        Priority::Urgent => stats.urgent += 1,
        Priority::High => stats.high += 1,
        Priority::Medium => stats.medium += 1,
        Priority::Low => stats.low += 1,
    }
}

fn column_of(status: Status) -> usize {
    match status {
        Status::Todo => 0,
        Status::InProgress => 1,
        Status::Complete => 2,
    }
}

fn priority_rank(priority: Priority) -> u8 {
    match priority {
        Priority::Urgent => 0,
        Priority::High => 1,
        Priority::Medium => 2,
        Priority::Low => 3,
    }
}

fn matches_section(section: NavSection, item: &Item, today: NaiveDate) -> bool {
    match section {
        NavSection::All => true,
        NavSection::Todo => item.status == Status::Todo,
        NavSection::InProgress => item.status == Status::InProgress,
        NavSection::Complete => item.status == Status::Complete,
        NavSection::Urgent => item.priority == Priority::Urgent,
        NavSection::Overdue => item.is_overdue(today),
    }
}

fn matches_query(item: &Item, filter: &str) -> bool {
    let query = filter.trim();
    if query.is_empty() {
        return true;
    }
    if let Some(tag_query) = query.strip_prefix('#') {
        return item.tags.iter().any(|t| contains_ignore_case(t, tag_query));
    }
    contains_ignore_case(&item.title, query)
        || contains_ignore_case(&item.description, query)
        || item.tags.iter().any(|t| contains_ignore_case(t, query))
        || contains_ignore_case(item.priority.label(), query)
        || contains_ignore_case(item.status.label(), query)
}
