use crate::{
    config::Config,
    models::{Item, Priority, Status},
    storage::JsonStore,
    sync::{SyncEngine, SyncStatus},
    textedit,
};
use crossterm::event::KeyCode;
use ratatui::widgets::TableState;
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

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

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum FormField {
    Title,
    Description,
    Priority,
    Tags,
}

impl FormField {
    #[inline(always)]
    pub fn next(&self) -> Self {
        match self {
            Self::Title => Self::Description,
            Self::Description => Self::Priority,
            Self::Priority => Self::Tags,
            Self::Tags => Self::Title,
        }
    }

    #[inline(always)]
    pub fn prev(&self) -> Self {
        match self {
            Self::Title => Self::Tags,
            Self::Description => Self::Title,
            Self::Priority => Self::Description,
            Self::Tags => Self::Priority,
        }
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum AppMode {
    Normal,
    Creating,
    Editing,
    Filtering,
    Command,
    Help,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
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

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum SortMode {
    Natural,
    Priority,
    Status,
    Title,
}

impl SortMode {
    #[inline(always)]
    pub fn next(&self) -> Self {
        match self {
            Self::Natural => Self::Priority,
            Self::Priority => Self::Status,
            Self::Status => Self::Title,
            Self::Title => Self::Natural,
        }
    }

    #[inline(always)]
    pub fn label(&self) -> &'static str {
        match self {
            Self::Natural => "NATURAL",
            Self::Priority => "PRIORITY",
            Self::Status => "STATUS",
            Self::Title => "TITLE",
        }
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum NavSection {
    All,
    Todo,
    InProgress,
    Complete,
    Urgent,
}

impl NavSection {
    pub const ALL: [NavSection; 5] = [
        NavSection::All,
        NavSection::Todo,
        NavSection::InProgress,
        NavSection::Complete,
        NavSection::Urgent,
    ];

    #[inline(always)]
    pub fn label(&self) -> &'static str {
        match self {
            Self::All => "All Tasks",
            Self::Todo => "Todo Queue",
            Self::InProgress => "In Progress",
            Self::Complete => "Completed",
            Self::Urgent => "Urgent Only",
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
    pub high: usize,
    pub medium: usize,
    pub low: usize,
}

pub struct App {
    pub items: Vec<Item>,
    pub table_state: TableState,
    pub mode: AppMode,
    pub view_mode: ViewMode,
    pub focus_pane: FocusPane,
    pub sort_mode: SortMode,
    pub nav_section: NavSection,
    pub nav_index: usize,

    pub kanban_col: usize,
    pub kanban_cursor: [usize; 3],

    pub form_field: FormField,
    pub form_title: String,
    pub form_description: String,
    pub form_priority: Priority,
    pub form_tags: String,
    pub form_edit_idx: Option<usize>,

    pub filter_input: String,
    pub tag_filter: Option<String>,

    pub command_input: String,

    /// Whether the currently focused text field (form title/description/tags,
    /// filter box) is in vim Insert mode (typing) or Normal mode (motions).
    pub field_insert: bool,
    /// Char-index cursor within whichever text field is currently focused.
    pub text_cursor: usize,
    /// Pending first `d` of a `dd` chord within a text field's Normal mode.
    pub field_pending_d: bool,
    /// `Some(anchor)` while a text field is in Visual mode (char-wise only).
    pub field_visual_anchor: Option<usize>,
    /// Last yanked/deleted text field selection, for `p`/`P`.
    pub field_clipboard: String,

    pub inspector_scroll: u16,

    pub pending_g: bool,
    pub pending_d: bool,
    pub last_deleted: Option<(usize, Item)>,
    pub status_message: Option<(String, Instant)>,
    pub store: JsonStore,
    pub sync_engine: SyncEngine,
    pub sync_status: SyncStatus,
    pub should_quit: bool,

    cached_filtered_indices: Vec<usize>,
    cached_stats: WorkspaceStats,
    cached_tags: Vec<(String, usize)>,
    cached_kanban_cols: [Vec<usize>; 3],
}

impl App {
    pub fn new() -> Self {
        let config = Config::load();
        let store = match &config.sync.data_dir {
            Some(dir) => JsonStore::at_dir(dir.clone()),
            None => JsonStore::default_path(),
        };
        let sync_engine = SyncEngine::spawn(
            store.data_dir().to_path_buf(),
            config.sync.remote.clone(),
            config.sync.branch.clone(),
            Duration::from_secs(config.sync.debounce_seconds),
            config.sync.pull_on_startup,
            config.sync.enabled,
        );
        Self::with_store(store, sync_engine)
    }

    /// Shared constructor so tests can point at an isolated, throwaway store
    /// instead of the real on-disk data — see `tests::create_test_app`.
    fn with_store(store: JsonStore, sync_engine: SyncEngine) -> Self {
        let items = store.load().unwrap_or_default();
        let mut table_state = TableState::default();
        if !items.is_empty() {
            table_state.select(Some(0));
        }

        let sync_status = if sync_engine.enabled {
            SyncStatus::Idle
        } else {
            SyncStatus::Disabled
        };

        let mut app = Self {
            items,
            table_state,
            mode: AppMode::Normal,
            view_mode: ViewMode::Table,
            focus_pane: FocusPane::Main,
            sort_mode: SortMode::Natural,
            nav_section: NavSection::All,
            nav_index: 0,
            kanban_col: 0,
            kanban_cursor: [0, 0, 0],
            form_field: FormField::Title,
            form_title: String::new(),
            form_description: String::new(),
            form_priority: Priority::Medium,
            form_tags: String::new(),
            form_edit_idx: None,
            filter_input: String::new(),
            tag_filter: None,
            command_input: String::new(),
            field_insert: true,
            text_cursor: 0,
            field_pending_d: false,
            field_visual_anchor: None,
            field_clipboard: String::new(),
            inspector_scroll: 0,
            pending_g: false,
            pending_d: false,
            last_deleted: None,
            status_message: None,
            store,
            sync_engine,
            sync_status,
            should_quit: false,
            cached_filtered_indices: Vec::new(),
            cached_stats: WorkspaceStats::default(),
            cached_tags: Vec::new(),
            cached_kanban_cols: [Vec::new(), Vec::new(), Vec::new()],
        };

        app.refresh_cache();
        app
    }

    pub fn refresh_cache(&mut self) {
        let total = self.items.len();
        let mut todo = 0;
        let mut in_progress = 0;
        let mut complete = 0;
        let mut urgent = 0;
        let mut high = 0;
        let mut medium = 0;
        let mut low = 0;

        let mut col_todo = Vec::new();
        let mut col_in_progress = Vec::new();
        let mut col_complete = Vec::new();
        let mut tag_counts: HashMap<String, usize> = HashMap::new();

        for (i, item) in self.items.iter().enumerate() {
            match item.status {
                Status::Todo => {
                    todo += 1;
                    col_todo.push(i);
                }
                Status::InProgress => {
                    in_progress += 1;
                    col_in_progress.push(i);
                }
                Status::Complete => {
                    complete += 1;
                    col_complete.push(i);
                }
            }
            match item.priority {
                Priority::Urgent => urgent += 1,
                Priority::High => high += 1,
                Priority::Medium => medium += 1,
                Priority::Low => low += 1,
            }
            for tag in &item.tags {
                *tag_counts.entry(tag.clone()).or_insert(0) += 1;
            }
        }

        let percent = (complete * 100usize).checked_div(total).unwrap_or(0);
        self.cached_stats = WorkspaceStats {
            total,
            todo,
            in_progress,
            complete,
            percent,
            urgent,
            high,
            medium,
            low,
        };

        self.cached_kanban_cols = [col_todo, col_in_progress, col_complete];

        let mut tags: Vec<(String, usize)> = tag_counts.into_iter().collect();
        tags.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        self.cached_tags = tags;

        let mut indices: Vec<usize> = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                let section_match = match self.nav_section {
                    NavSection::All => true,
                    NavSection::Todo => item.status == Status::Todo,
                    NavSection::InProgress => item.status == Status::InProgress,
                    NavSection::Complete => item.status == Status::Complete,
                    NavSection::Urgent => item.priority == Priority::Urgent,
                };
                if !section_match {
                    return false;
                }

                if let Some(ref filter) = self.tag_filter {
                    let f = filter.trim();
                    if f.is_empty() {
                        return true;
                    }

                    if let Some(tag_query) = f.strip_prefix('#') {
                        item.tags.iter().any(|t| contains_ignore_case(t, tag_query))
                    } else {
                        contains_ignore_case(&item.title, f)
                            || contains_ignore_case(&item.description, f)
                            || item.tags.iter().any(|t| contains_ignore_case(t, f))
                            || contains_ignore_case(item.priority.label(), f)
                            || contains_ignore_case(item.status.label(), f)
                    }
                } else {
                    true
                }
            })
            .map(|(i, _)| i)
            .collect();

        match self.sort_mode {
            SortMode::Natural => {}
            SortMode::Priority => {
                indices.sort_by(|&a, &b| {
                    let prio_val = |p: Priority| match p {
                        Priority::Urgent => 0,
                        Priority::High => 1,
                        Priority::Medium => 2,
                        Priority::Low => 3,
                    };
                    prio_val(self.items[a].priority).cmp(&prio_val(self.items[b].priority))
                });
            }
            SortMode::Status => {
                indices.sort_by(|&a, &b| {
                    let st_val = |s: Status| match s {
                        Status::Todo => 0,
                        Status::InProgress => 1,
                        Status::Complete => 2,
                    };
                    st_val(self.items[a].status).cmp(&st_val(self.items[b].status))
                });
            }
            SortMode::Title => {
                indices.sort_by(|&a, &b| {
                    self.items[a]
                        .title
                        .to_lowercase()
                        .cmp(&self.items[b].title.to_lowercase())
                });
            }
        }

        self.cached_filtered_indices = indices;
    }

    #[inline(always)]
    pub fn set_status_message(&mut self, msg: impl Into<String>) {
        self.status_message = Some((msg.into(), Instant::now()));
    }

    #[inline(always)]
    pub fn get_status_message(&self) -> Option<&str> {
        self.status_message
            .as_ref()
            .filter(|(_, instant)| instant.elapsed() < Duration::from_secs(4))
            .map(|(msg, _)| msg.as_str())
    }

    /// Drains pending sync worker events, applying UI-visible effects (reload,
    /// status messages). Returns `true` if anything was received, so the caller
    /// knows to redraw.
    pub fn poll_sync_events(&mut self) -> bool {
        let mut received = false;
        while let Some(status) = self.sync_engine.try_recv() {
            received = true;
            match &status {
                SyncStatus::UpdatedRemoteData => {
                    self.reload_from_disk();
                    self.set_status_message("Pulled remote changes");
                }
                SyncStatus::Offline => {
                    self.set_status_message("Offline \u{2014} will sync when connection returns");
                }
                SyncStatus::Conflict(backup) => {
                    self.reload_from_disk();
                    self.set_status_message(format!(
                        "Sync conflict \u{2014} local backup saved to {backup}"
                    ));
                }
                SyncStatus::Error(e) => {
                    self.set_status_message(format!("Sync error: {e}"));
                }
                _ => {}
            }
            self.sync_status = status;
        }
        received
    }

    pub fn reload_from_disk(&mut self) {
        if let Ok(items) = self.store.load() {
            self.items = items;
            self.refresh_cache();
            let len = self.cached_filtered_indices.len();
            if len == 0 {
                self.table_state.select(None);
            } else {
                let sel = self.table_state.selected().unwrap_or(0).min(len - 1);
                self.table_state.select(Some(sel));
            }
        }
    }

    pub fn trigger_manual_sync(&mut self) {
        if self.sync_engine.enabled {
            self.sync_engine.force_sync();
            self.set_status_message("Manual sync requested");
        } else {
            self.set_status_message("Sync is disabled (no Git repo configured)");
        }
    }

    pub fn shutdown_sync(&mut self) {
        self.sync_engine.shutdown(Duration::from_secs(3));
    }

    pub fn enter_command_mode(&mut self) {
        self.command_input.clear();
        self.mode = AppMode::Command;
        self.focus_text_field();
    }

    /// Parses and runs the `:command` line, then always returns to Normal
    /// mode (matching vim, where the command line closes after Enter).
    pub fn execute_command(&mut self) {
        match self.command_input.trim() {
            "sync" | "s" => self.trigger_manual_sync(),
            "q" | "quit" | "wq" => self.should_quit = true,
            "w" => self.set_status_message("Already saved automatically"),
            "help" | "h" => {
                self.mode = AppMode::Help;
                return;
            }
            "" => {}
            other => self.set_status_message(format!("Unknown command: {other}")),
        }
        self.mode = AppMode::Normal;
    }

    pub fn toggle_view_mode(&mut self) {
        self.view_mode = match self.view_mode {
            ViewMode::Table => ViewMode::Kanban,
            ViewMode::Kanban => ViewMode::Table,
        };
        let mode_name = match self.view_mode {
            ViewMode::Table => "Table Grid",
            ViewMode::Kanban => "Kanban Board",
        };
        self.set_status_message(format!("Switched to {mode_name} view"));
    }

    pub fn cycle_sort(&mut self) {
        self.sort_mode = self.sort_mode.next();
        self.refresh_cache();
        self.set_status_message(format!("Sort: {}", self.sort_mode.label()));
    }

    #[inline(always)]
    pub fn next_pane(&mut self) {
        self.focus_pane = self.focus_pane.next();
    }

    #[inline(always)]
    pub fn prev_pane(&mut self) {
        self.focus_pane = self.focus_pane.prev();
    }

    #[inline(always)]
    pub fn get_filtered_indices(&self) -> &[usize] {
        &self.cached_filtered_indices
    }

    pub fn select_next(&mut self) {
        match self.focus_pane {
            FocusPane::Nav => {
                let tags_count = self.cached_tags.len();
                let total_nav = NavSection::ALL.len() + tags_count;
                if total_nav > 0 {
                    self.nav_index = (self.nav_index + 1) % total_nav;
                    self.apply_nav_selection();
                }
            }
            FocusPane::Main => {
                if self.view_mode == ViewMode::Kanban {
                    self.kanban_next();
                } else {
                    let count = self.cached_filtered_indices.len();
                    if count == 0 {
                        return;
                    }
                    let i = match self.table_state.selected() {
                        Some(i) => {
                            if i >= count - 1 {
                                0
                            } else {
                                i + 1
                            }
                        }
                        None => 0,
                    };
                    self.table_state.select(Some(i));
                }
            }
            FocusPane::Inspector => {
                self.inspector_scroll = self.inspector_scroll.saturating_add(1);
            }
        }
    }

    pub fn select_prev(&mut self) {
        match self.focus_pane {
            FocusPane::Nav => {
                let tags_count = self.cached_tags.len();
                let total_nav = NavSection::ALL.len() + tags_count;
                if total_nav > 0 {
                    self.nav_index = if self.nav_index == 0 {
                        total_nav - 1
                    } else {
                        self.nav_index - 1
                    };
                    self.apply_nav_selection();
                }
            }
            FocusPane::Main => {
                if self.view_mode == ViewMode::Kanban {
                    self.kanban_prev();
                } else {
                    let count = self.cached_filtered_indices.len();
                    if count == 0 {
                        return;
                    }
                    let i = match self.table_state.selected() {
                        Some(i) => {
                            if i == 0 {
                                count - 1
                            } else {
                                i - 1
                            }
                        }
                        None => 0,
                    };
                    self.table_state.select(Some(i));
                }
            }
            FocusPane::Inspector => {
                self.inspector_scroll = self.inspector_scroll.saturating_sub(1);
            }
        }
    }

    pub fn apply_nav_selection(&mut self) {
        if self.nav_index < NavSection::ALL.len() {
            self.nav_section = NavSection::ALL[self.nav_index];
            self.tag_filter = None;
            self.set_status_message(format!("View: {}", self.nav_section.label()));
        } else {
            let tag_idx = self.nav_index - NavSection::ALL.len();
            if let Some((tag, _)) = self.cached_tags.get(tag_idx) {
                let tag_name = tag.clone();
                self.nav_section = NavSection::All;
                self.tag_filter = Some(format!("#{tag_name}"));
                self.set_status_message(format!("Filtered by tag #{tag_name}"));
            }
        }
        self.refresh_cache();
        self.table_state.select(Some(0));
    }

    #[inline(always)]
    pub fn get_kanban_column_indices(&self, status: Status) -> &[usize] {
        match status {
            Status::Todo => &self.cached_kanban_cols[0],
            Status::InProgress => &self.cached_kanban_cols[1],
            Status::Complete => &self.cached_kanban_cols[2],
        }
    }

    pub fn kanban_left(&mut self) {
        if self.kanban_col > 0 {
            self.kanban_col -= 1;
        } else {
            self.kanban_col = 2;
        }
    }

    pub fn kanban_right(&mut self) {
        if self.kanban_col < 2 {
            self.kanban_col += 1;
        } else {
            self.kanban_col = 0;
        }
    }

    pub fn kanban_next(&mut self) {
        let status = match self.kanban_col {
            0 => Status::Todo,
            1 => Status::InProgress,
            _ => Status::Complete,
        };
        let items = self.get_kanban_column_indices(status);
        if !items.is_empty() {
            let curr = self.kanban_cursor[self.kanban_col];
            self.kanban_cursor[self.kanban_col] =
                if curr >= items.len() - 1 { 0 } else { curr + 1 };
        }
    }

    pub fn kanban_prev(&mut self) {
        let status = match self.kanban_col {
            0 => Status::Todo,
            1 => Status::InProgress,
            _ => Status::Complete,
        };
        let items = self.get_kanban_column_indices(status);
        if !items.is_empty() {
            let curr = self.kanban_cursor[self.kanban_col];
            self.kanban_cursor[self.kanban_col] =
                if curr == 0 { items.len() - 1 } else { curr - 1 };
        }
    }

    pub fn kanban_move_task_left(&mut self) {
        if let Some(real_idx) = self.get_selected_real_index() {
            let curr_status = self.items[real_idx].status;
            let new_status = match curr_status {
                Status::Complete => Status::InProgress,
                Status::InProgress => Status::Todo,
                Status::Todo => Status::Complete,
            };
            self.items[real_idx].status = new_status;
            let _ = self.store.save(&self.items);
            self.sync_engine.request_push();
            self.refresh_cache();
            if self.kanban_col > 0 {
                self.kanban_col -= 1;
            }
            self.set_status_message(format!("Shifted task to {}", new_status.label()));
        }
    }

    pub fn kanban_move_task_right(&mut self) {
        if let Some(real_idx) = self.get_selected_real_index() {
            let curr_status = self.items[real_idx].status;
            let new_status = match curr_status {
                Status::Todo => Status::InProgress,
                Status::InProgress => Status::Complete,
                Status::Complete => Status::Todo,
            };
            self.items[real_idx].status = new_status;
            let _ = self.store.save(&self.items);
            self.sync_engine.request_push();
            self.refresh_cache();
            if self.kanban_col < 2 {
                self.kanban_col += 1;
            }
            self.set_status_message(format!("Shifted task to {}", new_status.label()));
        }
    }

    pub fn jump_to_top(&mut self) {
        if self.view_mode == ViewMode::Kanban {
            self.kanban_cursor[self.kanban_col] = 0;
        } else if !self.cached_filtered_indices.is_empty() {
            self.table_state.select(Some(0));
        }
    }

    pub fn jump_to_bottom(&mut self) {
        if self.view_mode == ViewMode::Kanban {
            let status = match self.kanban_col {
                0 => Status::Todo,
                1 => Status::InProgress,
                _ => Status::Complete,
            };
            let count = self.get_kanban_column_indices(status).len();
            if count > 0 {
                self.kanban_cursor[self.kanban_col] = count - 1;
            }
        } else {
            let len = self.cached_filtered_indices.len();
            if len > 0 {
                self.table_state.select(Some(len - 1));
            }
        }
    }

    pub fn page_down(&mut self, step: usize) {
        let count = self.cached_filtered_indices.len();
        if count == 0 {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => (i + step).min(count - 1),
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    pub fn page_up(&mut self, step: usize) {
        let count = self.cached_filtered_indices.len();
        if count == 0 {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => i.saturating_sub(step),
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    pub fn get_selected_real_index(&self) -> Option<usize> {
        if self.view_mode == ViewMode::Kanban {
            let status = match self.kanban_col {
                0 => Status::Todo,
                1 => Status::InProgress,
                _ => Status::Complete,
            };
            let items = self.get_kanban_column_indices(status);
            if items.is_empty() {
                return None;
            }
            let cursor = self.kanban_cursor[self.kanban_col];
            let idx = cursor.min(items.len().saturating_sub(1));
            items.get(idx).copied()
        } else {
            let sel = self.table_state.selected()?;
            self.cached_filtered_indices.get(sel).copied()
        }
    }

    #[inline(always)]
    pub fn get_selected_item(&self) -> Option<&Item> {
        let real_idx = self.get_selected_real_index()?;
        self.items.get(real_idx)
    }

    pub fn toggle_status(&mut self) {
        if let Some(real_idx) = self.get_selected_real_index() {
            self.items[real_idx].status = self.items[real_idx].status.next();
            let new_status = self.items[real_idx].status.label();
            let _ = self.store.save(&self.items);
            self.sync_engine.request_push();
            self.refresh_cache();
            self.set_status_message(format!("Status changed to {new_status}"));
        }
    }

    pub fn cycle_priority_forward(&mut self) {
        if let Some(real_idx) = self.get_selected_real_index() {
            self.items[real_idx].priority = self.items[real_idx].priority.next();
            let new_prio = self.items[real_idx].priority.label();
            let _ = self.store.save(&self.items);
            self.sync_engine.request_push();
            self.refresh_cache();
            self.set_status_message(format!("Priority set to {new_prio}"));
        }
    }

    pub fn cycle_priority_backward(&mut self) {
        if let Some(real_idx) = self.get_selected_real_index() {
            self.items[real_idx].priority = self.items[real_idx].priority.prev();
            let new_prio = self.items[real_idx].priority.label();
            let _ = self.store.save(&self.items);
            self.sync_engine.request_push();
            self.refresh_cache();
            self.set_status_message(format!("Priority set to {new_prio}"));
        }
    }

    pub fn set_priority(&mut self, priority: Priority) {
        if let Some(real_idx) = self.get_selected_real_index() {
            self.items[real_idx].priority = priority;
            let new_prio = priority.label();
            let _ = self.store.save(&self.items);
            self.sync_engine.request_push();
            self.refresh_cache();
            self.set_status_message(format!("Priority set to {new_prio}"));
        }
    }

    pub fn delete_selected(&mut self) {
        if let Some(real_idx) = self.get_selected_real_index() {
            let deleted = self.items.remove(real_idx);
            let snippet = if deleted.title.len() > 28 {
                format!("{}...", &deleted.title[..28])
            } else {
                deleted.title.clone()
            };
            self.last_deleted = Some((real_idx, deleted));

            self.refresh_cache();
            let filtered_len = self.cached_filtered_indices.len();
            if filtered_len == 0 {
                self.table_state.select(None);
            } else if self.table_state.selected().unwrap_or(0) >= filtered_len {
                self.table_state.select(Some(filtered_len - 1));
            }
            let _ = self.store.save(&self.items);
            self.sync_engine.request_push();
            self.set_status_message(format!("Deleted \"{snippet}\" (press 'u' to undo)"));
        }
    }

    pub fn undo_delete(&mut self) {
        if let Some((idx, item)) = self.last_deleted.take() {
            let insert_idx = idx.min(self.items.len());
            let snippet = if item.title.len() > 28 {
                format!("{}...", &item.title[..28])
            } else {
                item.title.clone()
            };
            self.items.insert(insert_idx, item);
            let _ = self.store.save(&self.items);
            self.sync_engine.request_push();
            self.refresh_cache();
            self.table_state.select(Some(insert_idx));
            self.set_status_message(format!("Restored \"{snippet}\""));
        } else {
            self.set_status_message("Nothing to undo");
        }
    }

    pub fn move_selected_up(&mut self) {
        if let Some(real_idx) = self.get_selected_real_index().filter(|&i| i > 0) {
            self.items.swap(real_idx, real_idx - 1);
            let _ = self.store.save(&self.items);
            self.sync_engine.request_push();
            self.refresh_cache();
            self.select_prev();
            self.set_status_message("Moved task up");
        }
    }

    pub fn move_selected_down(&mut self) {
        if let Some(real_idx) = self
            .get_selected_real_index()
            .filter(|&i| i + 1 < self.items.len())
        {
            self.items.swap(real_idx, real_idx + 1);
            let _ = self.store.save(&self.items);
            self.sync_engine.request_push();
            self.refresh_cache();
            self.select_next();
            self.set_status_message("Moved task down");
        }
    }

    pub fn open_create_modal(&mut self) {
        self.form_title.clear();
        self.form_description.clear();
        self.form_tags.clear();
        self.form_priority = Priority::Medium;
        self.form_field = FormField::Title;
        self.form_edit_idx = None;
        self.mode = AppMode::Creating;
        self.focus_text_field();
    }

    pub fn open_edit_modal(&mut self) {
        if let Some(real_idx) = self.get_selected_real_index() {
            let item = &self.items[real_idx];
            self.form_title = item.title.clone();
            self.form_description = item.description.clone();
            self.form_priority = item.priority;
            self.form_tags = item.tags.join(", ");
            self.form_field = FormField::Title;
            self.form_edit_idx = Some(real_idx);
            self.mode = AppMode::Editing;
            self.focus_text_field();
        }
    }

    /// Returns the text buffer belonging to whichever field is currently
    /// focused, or `None` when focus is on a non-text field (e.g. Priority).
    fn active_text_mut(&mut self) -> Option<&mut String> {
        match self.mode {
            AppMode::Creating | AppMode::Editing => match self.form_field {
                FormField::Title => Some(&mut self.form_title),
                FormField::Description => Some(&mut self.form_description),
                FormField::Tags => Some(&mut self.form_tags),
                FormField::Priority => None,
            },
            AppMode::Filtering => Some(&mut self.filter_input),
            AppMode::Command => Some(&mut self.command_input),
            _ => None,
        }
    }

    /// Only the Description field is allowed to contain literal newlines
    /// (`o`/`O` fall back to end/start-of-line for every other field).
    #[inline(always)]
    fn active_field_allows_newline(&self) -> bool {
        matches!(self.mode, AppMode::Creating | AppMode::Editing)
            && self.form_field == FormField::Description
    }

    /// Resets the currently focused text field to fresh Insert-mode focus
    /// with the cursor at the end — the entry point every time a field
    /// becomes focused (opening a modal, tabbing fields, opening `/` or `:`).
    pub fn focus_text_field(&mut self) {
        self.field_insert = true;
        self.field_pending_d = false;
        self.field_visual_anchor = None;
        self.text_cursor = self
            .active_text_mut()
            .map(|s| textedit::char_len(s))
            .unwrap_or(0);
    }

    /// Runs an edit against the currently focused text field, safely — the
    /// borrow on the field has to end before `self.text_cursor` can be
    /// written back, so this can't just be `f(self.active_text_mut()?, ...)`.
    fn edit_active(&mut self, f: impl FnOnce(&mut String, usize) -> usize) {
        let cursor = self.text_cursor;
        let mut new_cursor = None;
        if let Some(buf) = self.active_text_mut() {
            new_cursor = Some(f(buf, cursor));
        }
        if let Some(nc) = new_cursor {
            self.text_cursor = nc;
        }
    }

    /// Handles Esc for a focused text field: leaves Insert mode (clamping the
    /// cursor onto a real character, vim-style) and reports that it did, or
    /// reports `false` if the field was already in Normal mode — the caller
    /// (`main.rs`) then treats that as "close/cancel".
    pub fn handle_field_esc(&mut self) -> bool {
        if self.field_insert {
            self.field_insert = false;
            self.field_pending_d = false;
            self.edit_active(|buf, cursor| textedit::clamp_normal(buf, cursor));
            return true;
        }
        if self.field_visual_anchor.take().is_some() {
            return true;
        }
        false
    }

    /// Handles every other keystroke for a focused text field (form
    /// title/description/tags, filter box, `:` command line) in whichever of
    /// Insert/Normal mode it's currently in. Callers intercept Esc and Enter
    /// themselves first — see the `AppMode::Creating`/`Filtering` handling in
    /// `main.rs`.
    pub fn handle_text_key(&mut self, code: KeyCode, ctrl: bool) {
        let allow_newline = self.active_field_allows_newline();

        if self.field_insert {
            match code {
                KeyCode::Char('w') if ctrl => {
                    self.edit_active(|buf, _| {
                        App::delete_word_back(buf);
                        textedit::char_len(buf)
                    });
                }
                KeyCode::Char('u') if ctrl => {
                    self.edit_active(|buf, _| {
                        buf.clear();
                        0
                    });
                }
                KeyCode::Enter if allow_newline => {
                    self.edit_active(|buf, cursor| textedit::insert_char(buf, cursor, '\n'));
                }
                KeyCode::Char(c) => {
                    self.edit_active(|buf, cursor| textedit::insert_char(buf, cursor, c));
                }
                KeyCode::Backspace => {
                    self.edit_active(textedit::backspace);
                }
                KeyCode::Left => {
                    self.edit_active(|buf, cursor| textedit::caret_left(buf, cursor));
                }
                KeyCode::Right => {
                    self.edit_active(|buf, cursor| textedit::caret_right(buf, cursor));
                }
                KeyCode::Up if allow_newline => {
                    self.edit_active(|buf, cursor| textedit::caret_up(buf, cursor));
                }
                KeyCode::Down if allow_newline => {
                    self.edit_active(|buf, cursor| textedit::caret_down(buf, cursor));
                }
                _ => {}
            }
            return;
        }

        // Field-Normal mode: vim motions.
        if self.field_pending_d {
            self.field_pending_d = false;
            if code == KeyCode::Char('d') {
                self.edit_active(textedit::delete_line);
                return;
            }
            // any other key cancels the pending 'd' and falls through below
        }

        if let Some(anchor) = self.field_visual_anchor {
            self.handle_visual_key(code, anchor, allow_newline);
            return;
        }

        match code {
            KeyCode::Char('d') => self.field_pending_d = true,
            KeyCode::Char('v') => self.field_visual_anchor = Some(self.text_cursor),
            KeyCode::Char('p') => {
                let clip = self.field_clipboard.clone();
                self.edit_active(move |buf, cursor| textedit::paste_after(buf, cursor, &clip));
            }
            KeyCode::Char('P') => {
                let clip = self.field_clipboard.clone();
                self.edit_active(move |buf, cursor| textedit::paste_before(buf, cursor, &clip));
            }
            KeyCode::Char('h') | KeyCode::Left => {
                self.edit_active(|buf, cursor| textedit::move_left(buf, cursor));
            }
            KeyCode::Char('l') | KeyCode::Right => {
                self.edit_active(|buf, cursor| textedit::move_right(buf, cursor));
            }
            KeyCode::Char('j') | KeyCode::Down if allow_newline => {
                self.edit_active(|buf, cursor| textedit::move_down(buf, cursor));
            }
            KeyCode::Char('k') | KeyCode::Up if allow_newline => {
                self.edit_active(|buf, cursor| textedit::move_up(buf, cursor));
            }
            KeyCode::Char('0') => {
                self.edit_active(|buf, cursor| textedit::line_start(buf, cursor));
            }
            KeyCode::Char('$') => {
                self.edit_active(|buf, cursor| textedit::line_end(buf, cursor));
            }
            KeyCode::Char('w') => {
                self.edit_active(|buf, cursor| textedit::word_forward(buf, cursor));
            }
            KeyCode::Char('e') => {
                self.edit_active(|buf, cursor| textedit::word_end(buf, cursor));
            }
            KeyCode::Char('b') => {
                self.edit_active(|buf, cursor| textedit::word_back(buf, cursor));
            }
            KeyCode::Char('x') => {
                self.edit_active(textedit::delete_char);
            }
            KeyCode::Char('D') => {
                self.edit_active(textedit::delete_to_line_end);
            }
            KeyCode::Char('i') => {
                self.edit_active(|buf, cursor| textedit::enter_insert_before(buf, cursor));
                self.field_insert = true;
            }
            KeyCode::Char('a') => {
                self.edit_active(|buf, cursor| textedit::enter_insert_after(buf, cursor));
                self.field_insert = true;
            }
            KeyCode::Char('I') => {
                self.edit_active(|buf, cursor| textedit::enter_insert_line_start(buf, cursor));
                self.field_insert = true;
            }
            KeyCode::Char('A') => {
                self.edit_active(|buf, cursor| textedit::enter_insert_line_end(buf, cursor));
                self.field_insert = true;
            }
            KeyCode::Char('o') => {
                self.edit_active(|buf, cursor| textedit::open_below(buf, cursor, allow_newline));
                self.field_insert = true;
            }
            KeyCode::Char('O') => {
                self.edit_active(|buf, cursor| textedit::open_above(buf, cursor, allow_newline));
                self.field_insert = true;
            }
            _ => {}
        }
    }

    /// Handles a keystroke while a text field is in Visual mode (char-wise
    /// only — no linewise `V` or blockwise `<C-v>`). Motions extend the
    /// selection by moving the cursor while `anchor` stays fixed; `d`/`x`/`y`
    /// act on the normalized [lo, hi] range and exit back to Normal mode.
    fn handle_visual_key(&mut self, code: KeyCode, anchor: usize, allow_newline: bool) {
        match code {
            KeyCode::Char('v') => self.field_visual_anchor = None,
            KeyCode::Char('h') | KeyCode::Left => {
                self.edit_active(|buf, cursor| textedit::move_left(buf, cursor));
            }
            KeyCode::Char('l') | KeyCode::Right => {
                self.edit_active(|buf, cursor| textedit::move_right(buf, cursor));
            }
            KeyCode::Char('j') | KeyCode::Down if allow_newline => {
                self.edit_active(|buf, cursor| textedit::move_down(buf, cursor));
            }
            KeyCode::Char('k') | KeyCode::Up if allow_newline => {
                self.edit_active(|buf, cursor| textedit::move_up(buf, cursor));
            }
            KeyCode::Char('0') => {
                self.edit_active(|buf, cursor| textedit::line_start(buf, cursor));
            }
            KeyCode::Char('$') => {
                self.edit_active(|buf, cursor| textedit::line_end(buf, cursor));
            }
            KeyCode::Char('w') => {
                self.edit_active(|buf, cursor| textedit::word_forward(buf, cursor));
            }
            KeyCode::Char('e') => {
                self.edit_active(|buf, cursor| textedit::word_end(buf, cursor));
            }
            KeyCode::Char('b') => {
                self.edit_active(|buf, cursor| textedit::word_back(buf, cursor));
            }
            KeyCode::Char('y') => {
                let (lo, hi) = textedit::selection_range(anchor, self.text_cursor);
                if let Some(buf) = self.active_text_mut() {
                    self.field_clipboard = textedit::yank_range(buf, lo, hi);
                }
                self.text_cursor = lo;
                self.field_visual_anchor = None;
            }
            KeyCode::Char('d') | KeyCode::Char('x') | KeyCode::Char('c') => {
                let (lo, hi) = textedit::selection_range(anchor, self.text_cursor);
                if let Some(buf) = self.active_text_mut() {
                    self.field_clipboard = textedit::yank_range(buf, lo, hi);
                }
                self.edit_active(|buf, _| textedit::delete_range(buf, lo, hi));
                self.field_visual_anchor = None;
                self.field_insert = code == KeyCode::Char('c');
            }
            _ => {}
        }
    }

    pub fn commit_form(&mut self) {
        if self.form_title.trim().is_empty() {
            return;
        }

        let tags: Vec<String> = self
            .form_tags
            .split(',')
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
            .collect();

        if let Some(edit_idx) = self.form_edit_idx {
            if edit_idx < self.items.len() {
                self.items[edit_idx].title = self.form_title.trim().to_string();
                self.items[edit_idx].description = self.form_description.trim().to_string();
                self.items[edit_idx].priority = self.form_priority;
                self.items[edit_idx].tags = tags;
                let _ = self.store.save(&self.items);
                self.sync_engine.request_push();
                self.refresh_cache();
                self.set_status_message("Task updated");
            }
        } else {
            let mut new_item = Item::new(self.form_title.trim().to_string(), tags);
            new_item.description = self.form_description.trim().to_string();
            new_item.priority = self.form_priority;
            if self.view_mode == ViewMode::Kanban {
                new_item.status = match self.kanban_col {
                    0 => Status::Todo,
                    1 => Status::InProgress,
                    _ => Status::Complete,
                };
            }
            self.items.push(new_item);
            let _ = self.store.save(&self.items);
            self.sync_engine.request_push();
            self.refresh_cache();
            let new_sel = self.cached_filtered_indices.len().saturating_sub(1);
            self.table_state.select(Some(new_sel));
            self.set_status_message("Task created");
        }

        self.form_edit_idx = None;
        self.mode = AppMode::Normal;
    }

    pub fn cycle_tag_filter(&mut self) {
        if self.cached_tags.is_empty() {
            self.tag_filter = None;
            self.set_status_message("No tags available to filter");
            return;
        }

        match &self.tag_filter {
            None => {
                let first = self.cached_tags[0].0.clone();
                self.tag_filter = Some(format!("#{first}"));
                self.set_status_message(format!("Filter: #{first}"));
            }
            Some(curr) => {
                let curr_clean = curr.strip_prefix('#').unwrap_or(curr);
                if let Some(idx) = self.cached_tags.iter().position(|(t, _)| t == curr_clean) {
                    if idx + 1 < self.cached_tags.len() {
                        let next_tag = self.cached_tags[idx + 1].0.clone();
                        self.tag_filter = Some(format!("#{next_tag}"));
                        self.set_status_message(format!("Filter: #{next_tag}"));
                    } else {
                        self.tag_filter = None;
                        self.set_status_message("Filter cleared");
                    }
                } else {
                    let first = self.cached_tags[0].0.clone();
                    self.tag_filter = Some(format!("#{first}"));
                    self.set_status_message(format!("Filter: #{first}"));
                }
            }
        }
        self.refresh_cache();
        self.table_state.select(Some(0));
    }

    pub fn clear_filter(&mut self) {
        let mut cleared = false;
        if self.tag_filter.is_some() {
            self.tag_filter = None;
            cleared = true;
        }
        if self.nav_section != NavSection::All {
            self.nav_section = NavSection::All;
            self.nav_index = 0;
            cleared = true;
        }
        if cleared {
            self.refresh_cache();
            self.table_state.select(Some(0));
            self.set_status_message("Filter reset to All Tasks");
        }
    }

    #[inline(always)]
    pub fn all_tags_with_counts(&self) -> &[(String, usize)] {
        &self.cached_tags
    }

    #[inline(always)]
    pub fn stats(&self) -> WorkspaceStats {
        self.cached_stats
    }

    pub fn delete_word_back(buffer: &mut String) {
        while buffer.ends_with(' ') {
            buffer.pop();
        }
        while !buffer.is_empty() && !buffer.ends_with(' ') {
            buffer.pop();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_app() -> App {
        // Use a throwaway temp directory, never the real on-disk data — a
        // previous version of this test called `App::new()` directly, which
        // pointed straight at the user's real ~/.local/share/hopes/items.json
        // and clobbered it the moment any mutating test method ran.
        let dir = std::env::temp_dir().join(format!(
            "hopes-app-test-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let store = crate::storage::JsonStore::at_dir(dir);
        let sync_engine = crate::sync::SyncEngine::spawn(
            store.data_dir().to_path_buf(),
            "origin".to_string(),
            "main".to_string(),
            Duration::from_secs(5),
            false,
            false,
        );
        let mut app = App::with_store(store, sync_engine);
        app.items = vec![
            Item::new(
                "Write compiler backend".to_string(),
                vec!["rust".to_string(), "core".to_string()],
            ),
            Item::new(
                "Design responsive TUI".to_string(),
                vec!["ui".to_string(), "tui".to_string()],
            ),
            Item::new(
                "Implement undo buffer".to_string(),
                vec!["rust".to_string(), "feature".to_string()],
            ),
        ];
        app.items[0].priority = Priority::Urgent;
        app.items[1].priority = Priority::High;
        app.items[2].priority = Priority::Medium;
        app.refresh_cache();
        app.table_state.select(Some(0));
        app
    }

    #[test]
    fn test_navigation_vim_motions() {
        let mut app = create_test_app();
        assert_eq!(app.table_state.selected(), Some(0));

        app.select_next();
        assert_eq!(app.table_state.selected(), Some(1));

        app.jump_to_bottom();
        assert_eq!(app.table_state.selected(), Some(2));

        app.select_next();
        assert_eq!(app.table_state.selected(), Some(0));

        app.select_prev();
        assert_eq!(app.table_state.selected(), Some(2));

        app.jump_to_top();
        assert_eq!(app.table_state.selected(), Some(0));
    }

    #[test]
    fn test_status_toggle() {
        let mut app = create_test_app();
        assert_eq!(app.items[0].status, Status::Todo);

        app.toggle_status();
        assert_eq!(app.items[0].status, Status::InProgress);

        app.toggle_status();
        assert_eq!(app.items[0].status, Status::Complete);

        app.toggle_status();
        assert_eq!(app.items[0].status, Status::Todo);
    }

    #[test]
    fn test_priority_operations() {
        let mut app = create_test_app();
        assert_eq!(app.items[0].priority, Priority::Urgent);

        app.cycle_priority_forward();
        assert_eq!(app.items[0].priority, Priority::High);

        app.set_priority(Priority::Low);
        assert_eq!(app.items[0].priority, Priority::Low);
    }

    #[test]
    fn test_reorder_tasks() {
        let mut app = create_test_app();
        assert_eq!(app.items[0].title, "Write compiler backend");

        app.move_selected_down();
        assert_eq!(app.items[0].title, "Design responsive TUI");
        assert_eq!(app.items[1].title, "Write compiler backend");

        app.move_selected_up();
        assert_eq!(app.items[0].title, "Write compiler backend");
    }

    #[test]
    fn test_delete_and_undo() {
        let mut app = create_test_app();
        let original_count = app.items.len();

        app.delete_selected();
        assert_eq!(app.items.len(), original_count - 1);
        assert_eq!(app.items[0].title, "Design responsive TUI");

        app.undo_delete();
        assert_eq!(app.items.len(), original_count);
        assert_eq!(app.items[0].title, "Write compiler backend");
    }

    #[test]
    fn test_filtering_and_stats() {
        let mut app = create_test_app();
        let stats = app.stats();
        assert_eq!(stats.total, 3);
        assert_eq!(stats.todo, 3);
        assert_eq!(stats.urgent, 1);
        assert_eq!(stats.high, 1);

        app.tag_filter = Some("#rust".to_string());
        app.refresh_cache();
        let filtered = app.get_filtered_indices();
        assert_eq!(filtered.len(), 2);

        app.tag_filter = Some("responsive".to_string());
        app.refresh_cache();
        let filtered2 = app.get_filtered_indices();
        assert_eq!(filtered2.len(), 1);
    }

    #[test]
    fn test_view_mode_and_kanban() {
        let mut app = create_test_app();
        assert_eq!(app.view_mode, ViewMode::Table);

        app.toggle_view_mode();
        assert_eq!(app.view_mode, ViewMode::Kanban);

        app.kanban_right();
        assert_eq!(app.kanban_col, 1);

        app.kanban_left();
        assert_eq!(app.kanban_col, 0);
    }

    #[test]
    fn insert_mode_types_at_cursor_not_just_at_the_end() {
        let mut app = create_test_app();
        app.open_create_modal();
        assert!(app.field_insert);
        assert_eq!(app.form_field, FormField::Title);

        for c in "helloworld".chars() {
            app.handle_text_key(KeyCode::Char(c), false);
        }
        assert_eq!(app.form_title, "helloworld");
        assert_eq!(app.text_cursor, 10);

        // Move left 5 (to right after "hello") and insert a space.
        for _ in 0..5 {
            app.handle_text_key(KeyCode::Left, false);
        }
        app.handle_text_key(KeyCode::Char(' '), false);
        assert_eq!(app.form_title, "hello world");
        assert_eq!(app.text_cursor, 6);
    }

    #[test]
    fn esc_leaves_insert_before_cancelling_the_modal() {
        let mut app = create_test_app();
        app.open_create_modal();
        app.form_title = "draft".to_string();
        app.text_cursor = 5;

        // First Esc: still Insert -> drops to field-Normal, modal stays open.
        assert!(app.handle_field_esc());
        assert!(!app.field_insert);
        assert_eq!(app.mode, AppMode::Creating);
        assert_eq!(app.form_title, "draft"); // untouched

        // Second Esc: already Normal -> caller (main.rs) closes the modal.
        assert!(!app.handle_field_esc());
    }

    #[test]
    fn dd_in_field_normal_mode_clears_a_single_line_field() {
        let mut app = create_test_app();
        app.open_create_modal();
        app.form_title = "delete me".to_string();
        app.text_cursor = 3;
        app.field_insert = false;

        app.handle_text_key(KeyCode::Char('d'), false);
        assert!(app.field_pending_d);
        app.handle_text_key(KeyCode::Char('d'), false);
        assert!(!app.field_pending_d);
        assert_eq!(app.form_title, "");
    }

    #[test]
    fn tabbing_between_fields_resets_to_insert_at_the_end() {
        let mut app = create_test_app();
        app.open_create_modal();
        app.form_title = "abc".to_string();
        app.field_insert = false;
        app.text_cursor = 0;

        app.form_field = app.form_field.next(); // -> Description
        app.focus_text_field();
        assert!(app.field_insert);
        assert_eq!(app.text_cursor, crate::textedit::char_len(&app.form_description));
    }

    #[test]
    fn command_quit_sets_should_quit() {
        let mut app = create_test_app();
        app.enter_command_mode();
        app.command_input = "q".to_string();
        app.execute_command();
        assert!(app.should_quit);
        assert_eq!(app.mode, AppMode::Normal);
    }

    #[test]
    fn command_sync_triggers_manual_sync_without_leaving_early() {
        let mut app = create_test_app();
        app.enter_command_mode();
        app.command_input = "sync".to_string();
        app.execute_command();
        assert_eq!(app.mode, AppMode::Normal);
        assert!(!app.should_quit);
    }

    #[test]
    fn unknown_command_reports_status_and_returns_to_normal() {
        let mut app = create_test_app();
        app.enter_command_mode();
        app.command_input = "bogus".to_string();
        app.execute_command();
        assert_eq!(app.mode, AppMode::Normal);
        assert!(
            app.get_status_message()
                .unwrap_or_default()
                .contains("Unknown command")
        );
    }

    #[test]
    fn visual_mode_delete_removes_selection_and_exits_visual() {
        let mut app = create_test_app();
        app.open_create_modal();
        app.form_title = "hello world".to_string();
        app.field_insert = false;
        app.text_cursor = 0;

        app.handle_text_key(KeyCode::Char('v'), false);
        assert!(app.field_visual_anchor.is_some());

        for _ in 0..4 {
            app.handle_text_key(KeyCode::Char('l'), false); // extend to "hello"
        }
        app.handle_text_key(KeyCode::Char('d'), false);

        assert_eq!(app.form_title, " world");
        assert!(app.field_visual_anchor.is_none());
        assert_eq!(app.field_clipboard, "hello");
    }

    #[test]
    fn visual_mode_yank_then_normal_mode_paste() {
        let mut app = create_test_app();
        app.open_create_modal();
        app.form_title = "hello world".to_string();
        app.field_insert = false;
        app.text_cursor = 6; // 'w' of world

        app.handle_text_key(KeyCode::Char('v'), false);
        for _ in 0..4 {
            app.handle_text_key(KeyCode::Char('l'), false); // extend to "world"
        }
        app.handle_text_key(KeyCode::Char('y'), false);
        assert_eq!(app.field_clipboard, "world");
        assert!(app.field_visual_anchor.is_none());
        assert_eq!(app.form_title, "hello world"); // yank doesn't mutate

        app.text_cursor = 4; // resting on the 'o' at the end of "hello"
        app.handle_text_key(KeyCode::Char('p'), false);
        assert_eq!(app.form_title, "helloworld world");
    }

    #[test]
    fn visual_mode_change_deletes_and_enters_insert() {
        let mut app = create_test_app();
        app.open_create_modal();
        app.form_title = "abcdef".to_string();
        app.field_insert = false;
        app.text_cursor = 1;

        app.handle_text_key(KeyCode::Char('v'), false);
        app.handle_text_key(KeyCode::Char('l'), false); // select "bc"
        app.handle_text_key(KeyCode::Char('c'), false);

        assert_eq!(app.form_title, "adef");
        assert!(app.field_insert);
        assert!(app.field_visual_anchor.is_none());
    }

    #[test]
    fn esc_exits_visual_mode_without_editing_before_cancelling_modal() {
        let mut app = create_test_app();
        app.open_create_modal();
        app.form_title = "keep me".to_string();
        app.field_insert = false;
        app.text_cursor = 0;
        app.handle_text_key(KeyCode::Char('v'), false);

        assert!(app.handle_field_esc()); // exits visual, modal stays open
        assert!(app.field_visual_anchor.is_none());
        assert_eq!(app.mode, AppMode::Creating);
        assert_eq!(app.form_title, "keep me");

        assert!(!app.handle_field_esc()); // plain Normal now -> caller cancels
    }
}
