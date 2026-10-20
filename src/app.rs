use crate::{
    action::{Action, NavTarget, TaskDraft},
    config::{Backend, Config},
    domain::{AppState, FocusPane, Item, NavSection, Priority, ViewMode, snippet},
    event::{Event, EventHandler},
    systems::{
        storage::{self, Storage},
        sync::{SyncEngine, SyncStatus},
        view_state::ViewState,
    },
    tui::Tui,
    ui::{
        Component, KeyResult, TaskView,
        components::{
            cmdline::CmdlineComponent,
            editor::EditorComponent,
            footer::{self, FooterCtx},
            header,
            help::HelpComponent,
            inspector::InspectorComponent,
            kanban::KanbanComponent,
            nav::NavComponent,
            table::TableComponent,
        },
        layout,
        text_input::TextInput,
    },
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Frame;
use std::{
    io,
    sync::mpsc::{self, Receiver, Sender},
    time::Duration,
};

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum AppMode {
    Normal,
    Creating,
    Editing,
    Filtering,
    Command,
    Help,
}

pub struct App {
    pub state: AppState,
    mode: AppMode,
    should_quit: bool,

    store: Box<dyn Storage>,
    sync_engine: SyncEngine,

    action_tx: Sender<Action>,
    action_rx: Receiver<Action>,

    table: TableComponent,
    kanban: KanbanComponent,
    nav: NavComponent,
    inspector: InspectorComponent,
    editor: EditorComponent,
    help: HelpComponent,
    cmdline: CmdlineComponent,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        let config = Config::load();
        let store =
            storage::open(&config.storage, config.sync.data_dir.as_ref()).unwrap_or_else(|e| {
                eprintln!("hopes: {e}");
                std::process::exit(1);
            });
        let sync_engine = SyncEngine::spawn(
            store.data_dir().to_path_buf(),
            config.sync.remote.clone(),
            config.sync.branch.clone(),
            Duration::from_secs(config.sync.debounce_seconds),
            config.sync.pull_on_startup,
            config.sync.enabled && config.storage.backend == Backend::Json,
        );
        let mut app = Self::with_store(store, sync_engine);
        if let Some(view) = ViewState::load() {
            view.apply(&mut app.state);
            app.nav.restore(&app.state);
        }
        app
    }

    pub fn save_view_state(&self) {
        ViewState::capture(&self.state).save();
    }

    pub fn with_store(store: Box<dyn Storage>, sync_engine: SyncEngine) -> Self {
        let items = store.load().unwrap_or_default();
        let sync_status = if sync_engine.enabled {
            SyncStatus::Idle
        } else {
            SyncStatus::Disabled
        };
        let (action_tx, action_rx) = mpsc::channel();

        Self {
            table: TableComponent::new(!items.is_empty()),
            state: AppState::new(items, sync_status),
            mode: AppMode::Normal,
            should_quit: false,
            store,
            sync_engine,
            action_tx,
            action_rx,
            kanban: KanbanComponent::default(),
            nav: NavComponent::default(),
            inspector: InspectorComponent::default(),
            editor: EditorComponent::default(),
            help: HelpComponent::default(),
            cmdline: CmdlineComponent::default(),
        }
    }

    pub fn run(&mut self, terminal: &mut Tui) -> io::Result<()> {
        let events = EventHandler::new(60);
        let mut needs_draw = true;

        loop {
            if self.drain_sync() {
                needs_draw = true;
            }
            self.process_actions();

            if self.should_quit {
                return Ok(());
            }

            if needs_draw || self.state.status().is_some() {
                terminal.draw(|f| self.render(f))?;
                needs_draw = false;
            }

            if let Event::Key(key) = events.next()? {
                needs_draw = true;
                self.handle_key(key);
            }
        }
    }

    pub fn shutdown_sync(&mut self) {
        self.sync_engine.shutdown();
    }

    pub fn mode(&self) -> AppMode {
        self.mode
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        let result = match self.mode {
            AppMode::Normal => self.normal_key(key),
            AppMode::Creating | AppMode::Editing => self.editor.handle_key(key, &self.state),
            AppMode::Filtering | AppMode::Command => self.cmdline.handle_key(key, &self.state),
            AppMode::Help => self.help.handle_key(key, &self.state),
        };
        if let KeyResult::Emit(action) = result {
            self.send(action);
        }
    }

    fn normal_key(&mut self, key: KeyEvent) -> KeyResult {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

        let starts_chord = matches!(key.code, KeyCode::Char('g' | 'd')) && !ctrl;
        if !starts_chord {
            self.table.clear_pending();
            self.kanban.clear_pending();
        }

        match key.code {
            KeyCode::Char('q') => return KeyResult::Emit(Action::Quit),
            KeyCode::Char('c') if ctrl => return KeyResult::Emit(Action::Quit),
            KeyCode::Tab => return KeyResult::Emit(Action::NextPane),
            KeyCode::Char('w') if ctrl => return KeyResult::Emit(Action::NextPane),
            KeyCode::BackTab => return KeyResult::Emit(Action::PrevPane),
            _ => {}
        }

        let focused = match self.state.focus {
            FocusPane::Nav => self.nav.handle_key(key, &self.state),
            FocusPane::Inspector => self.inspector.handle_key(key, &self.state),
            FocusPane::Main => KeyResult::Ignored,
        };
        if !matches!(focused, KeyResult::Ignored) {
            return focused;
        }

        let view: &mut dyn TaskView = match self.state.view_mode {
            ViewMode::Table => &mut self.table,
            ViewMode::Kanban => &mut self.kanban,
        };
        let from_view = view.handle_key(key, &self.state);
        if !matches!(from_view, KeyResult::Ignored) {
            return from_view;
        }

        Self::global_key(key)
    }

    fn global_key(key: KeyEvent) -> KeyResult {
        let action = match key.code {
            KeyCode::Char('v') => Action::ToggleViewMode,
            KeyCode::Char('s') => Action::CycleSort,
            KeyCode::Char('h') | KeyCode::Left => Action::PrevPane,
            KeyCode::Char('l') | KeyCode::Right => Action::NextPane,
            KeyCode::Char('u') => Action::UndoDelete,
            KeyCode::Char('/') => Action::OpenFind,
            KeyCode::Char(':') => Action::OpenCommand,
            KeyCode::Char('t') => Action::CycleTagFilter,
            KeyCode::Esc => Action::ClearFilter,
            KeyCode::Char('?') => Action::OpenHelp,
            KeyCode::Char('S') | KeyCode::F(5) => Action::ForceSync,
            _ => return KeyResult::Ignored,
        };
        KeyResult::Emit(action)
    }

    pub fn send(&self, action: Action) {
        let _ = self.action_tx.send(action);
    }

    pub fn process_actions(&mut self) {
        while let Ok(action) = self.action_rx.try_recv() {
            self.process_action(action);
        }
    }

    fn process_action(&mut self, action: Action) {
        if let Action::Batch(actions) = action {
            for action in actions {
                self.process_action(action);
            }
            return;
        }

        self.apply(&action);

        self.nav.update(&action, &self.state);
        self.table.update(&action, &self.state);
        self.kanban.update(&action, &self.state);
        self.inspector.update(&action, &self.state);
        self.editor.update(&action, &self.state);
        self.help.update(&action, &self.state);
        self.cmdline.update(&action, &self.state);
    }

    fn apply(&mut self, action: &Action) {
        match action {
            Action::Quit => self.should_quit = true,

            Action::NextPane => self.state.focus = self.state.focus.next(),
            Action::PrevPane => self.state.focus = self.state.focus.prev(),
            Action::SetFocus(pane) => self.state.focus = *pane,
            Action::ToggleViewMode => self.toggle_view_mode(),
            Action::CycleSort => self.cycle_sort(),

            Action::OpenCreateModal { .. } => self.mode = AppMode::Creating,
            Action::OpenEditModal(idx) => self.open_edit(*idx),
            Action::OpenFind => self.mode = AppMode::Filtering,
            Action::OpenCommand => self.mode = AppMode::Command,
            Action::OpenHelp => self.mode = AppMode::Help,
            Action::CloseModal => self.mode = AppMode::Normal,
            Action::RunCommand(command) => self.run_command(command),

            Action::ApplyNav(target) => self.apply_nav(target),
            Action::SetFilter(filter) => self.set_filter(filter.clone()),
            Action::CycleTagFilter => self.cycle_tag_filter(),
            Action::ClearFilter => self.clear_filter(),

            Action::SaveTask(draft) => self.save_task(draft),
            Action::DeleteTask(idx) => self.delete_task(*idx),
            Action::UndoDelete => self.undo_delete(),
            Action::ToggleTaskStatus(idx) => self.toggle_task_status(*idx),
            Action::ShiftTaskStatus { idx, forward } => self.shift_task_status(*idx, *forward),
            Action::SetPriority { idx, priority } => self.set_priority(*idx, *priority),
            Action::CyclePriority { idx, forward } => self.cycle_priority(*idx, *forward),
            Action::MoveTaskUp(idx) => self.move_task_up(*idx),
            Action::MoveTaskDown(idx) => self.move_task_down(*idx),

            Action::SyncRequested => self.sync_engine.request_push(),
            Action::ForceSync => self.trigger_manual_sync(),
            Action::SyncStatus(status) => self.apply_sync_status(status),

            Action::ResetNavCursor
            | Action::SelectNext
            | Action::SelectPrev
            | Action::SelectIndex(_)
            | Action::DataReloaded
            | Action::Batch(_) => {}
        }
    }

    fn persist(&mut self) {
        let _ = self.store.save(&self.state.items);
        self.send(Action::SyncRequested);
    }

    fn commit_change(&mut self, status: String, audit: String) {
        self.persist();
        self.state.refresh_cache();
        self.state.set_status(status);
        self.state.log(audit);
    }

    fn toggle_view_mode(&mut self) {
        let (view_mode, name) = match self.state.view_mode {
            ViewMode::Table => (ViewMode::Kanban, "Kanban Board"),
            ViewMode::Kanban => (ViewMode::Table, "Table Grid"),
        };
        self.state.view_mode = view_mode;
        self.state.set_status(format!("Switched to {name} view"));
    }

    fn cycle_sort(&mut self) {
        self.state.sort_mode = self.state.sort_mode.next();
        self.state.refresh_cache();
        let label = self.state.sort_mode.label();
        self.state.set_status(format!("Sort: {label}"));
    }

    fn open_edit(&mut self, idx: usize) {
        if idx < self.state.items.len() {
            self.mode = AppMode::Editing;
        }
    }

    fn apply_nav(&mut self, target: &NavTarget) {
        match target {
            NavTarget::Section(section) => {
                self.state.nav_section = *section;
                self.state.tag_filter = None;
                self.state.set_status(format!("View: {}", section.label()));
            }
            NavTarget::Tag(tag) => {
                self.state.nav_section = NavSection::All;
                self.state.tag_filter = Some(format!("#{tag}"));
                self.state.set_status(format!("Filtered by tag #{tag}"));
            }
        }
        self.filter_changed();
    }

    fn set_filter(&mut self, filter: Option<String>) {
        self.state.tag_filter = filter;
        self.filter_changed();
    }

    fn save_task(&mut self, draft: &TaskDraft) {
        match draft.id {
            Some(idx) => self.update_task(idx, draft),
            None => self.create_task(draft),
        }
        self.mode = AppMode::Normal;
    }

    fn update_task(&mut self, idx: usize, draft: &TaskDraft) {
        let Some(item) = self.state.items.get_mut(idx) else {
            return;
        };
        item.title = draft.title.clone();
        item.description = draft.description.clone();
        item.priority = draft.priority;
        item.tags = draft.tags.clone();
        let short = snippet(&item.title, 28);
        self.commit_change("Task updated".into(), format!("Edited \"{short}\""));
    }

    fn create_task(&mut self, draft: &TaskDraft) {
        let mut item = Item::new(draft.title.clone(), draft.tags.clone());
        item.description = draft.description.clone();
        item.priority = draft.priority;
        item.status = draft.status;
        let short = snippet(&draft.title, 28);
        self.state.items.push(item);
        self.commit_change("Task created".into(), format!("Created \"{short}\""));
        let last = self.state.filtered_indices().len().saturating_sub(1);
        self.send(Action::SelectIndex(last));
    }

    fn delete_task(&mut self, idx: usize) {
        if idx >= self.state.items.len() {
            return;
        }
        let deleted = self.state.items.remove(idx);
        let short = snippet(&deleted.title, 28);
        self.state.last_deleted = Some((idx, deleted));
        self.commit_change(
            format!("Deleted \"{short}\" (press 'u' to undo)"),
            format!("Deleted \"{short}\""),
        );
    }

    fn undo_delete(&mut self) {
        let Some((idx, item)) = self.state.last_deleted.take() else {
            self.state.set_status("Nothing to undo");
            return;
        };
        let insert_idx = idx.min(self.state.items.len());
        let short = snippet(&item.title, 28);
        self.state.items.insert(insert_idx, item);
        self.commit_change(
            format!("Restored \"{short}\""),
            format!("Restored \"{short}\""),
        );
        self.send(Action::SelectIndex(insert_idx));
    }

    fn toggle_task_status(&mut self, idx: usize) {
        let Some(item) = self.state.items.get_mut(idx) else {
            return;
        };
        item.status = item.status.next();
        let label = item.status.label();
        let short = snippet(&item.title, 28);
        self.commit_change(
            format!("Status changed to {label}"),
            format!("\"{short}\" status \u{2192} {label}"),
        );
    }

    fn shift_task_status(&mut self, idx: usize, forward: bool) {
        let Some(item) = self.state.items.get_mut(idx) else {
            return;
        };
        item.status = if forward {
            item.status.next()
        } else {
            item.status.prev()
        };
        let label = item.status.label();
        let short = snippet(&item.title, 28);
        self.commit_change(
            format!("Shifted task to {label}"),
            format!("Shifted \"{short}\" to {label}"),
        );
    }

    fn set_priority(&mut self, idx: usize, priority: Priority) {
        let Some(item) = self.state.items.get_mut(idx) else {
            return;
        };
        item.priority = priority;
        let label = priority.label();
        let short = snippet(&item.title, 28);
        self.commit_change(
            format!("Priority set to {label}"),
            format!("\"{short}\" priority \u{2192} {label}"),
        );
    }

    fn cycle_priority(&mut self, idx: usize, forward: bool) {
        let Some(item) = self.state.items.get(idx) else {
            return;
        };
        let next = if forward {
            item.priority.next()
        } else {
            item.priority.prev()
        };
        self.set_priority(idx, next);
    }

    fn move_task_up(&mut self, idx: usize) {
        if idx == 0 || idx >= self.state.items.len() {
            return;
        }
        self.state.items.swap(idx, idx - 1);
        let short = snippet(&self.state.items[idx - 1].title, 28);
        self.commit_change("Moved task up".into(), format!("Moved \"{short}\" up"));
        self.send(Action::SelectPrev);
    }

    fn move_task_down(&mut self, idx: usize) {
        if idx + 1 >= self.state.items.len() {
            return;
        }
        self.state.items.swap(idx, idx + 1);
        let short = snippet(&self.state.items[idx + 1].title, 28);
        self.commit_change("Moved task down".into(), format!("Moved \"{short}\" down"));
        self.send(Action::SelectNext);
    }

    fn filter_changed(&mut self) {
        self.state.refresh_cache();
        self.send(Action::SelectIndex(0));
    }

    fn cycle_tag_filter(&mut self) {
        let tags: Vec<String> = self.state.tags().iter().map(|(t, _)| t.clone()).collect();
        if tags.is_empty() {
            self.state.tag_filter = None;
            self.state.set_status("No tags available to filter");
            return;
        }

        let next = match &self.state.tag_filter {
            None => Some(&tags[0]),
            Some(curr) => {
                let curr_clean = curr.strip_prefix('#').unwrap_or(curr);
                match tags.iter().position(|t| t == curr_clean) {
                    Some(idx) => tags.get(idx + 1),
                    None => Some(&tags[0]),
                }
            }
        };
        match next {
            Some(tag) => {
                self.state.tag_filter = Some(format!("#{tag}"));
                self.state.set_status(format!("Filter: #{tag}"));
            }
            None => {
                self.state.tag_filter = None;
                self.state.set_status("Filter cleared");
            }
        }
        self.filter_changed();
    }

    fn clear_filter(&mut self) {
        let mut cleared = false;
        if self.state.tag_filter.take().is_some() {
            cleared = true;
        }
        if self.state.nav_section != NavSection::All {
            self.state.nav_section = NavSection::All;
            self.send(Action::ResetNavCursor);
            cleared = true;
        }
        if cleared {
            self.filter_changed();
            self.state.set_status("Filter reset to All");
        }
    }

    fn run_command(&mut self, command: &str) {
        match command.trim() {
            "sync" | "s" => self.trigger_manual_sync(),
            "q" | "quit" | "wq" => self.should_quit = true,
            "w" => self.state.set_status("Already saved automatically"),
            "help" | "h" => {
                self.send(Action::OpenHelp);
                return;
            }
            "" => {}
            other => self.state.set_status(format!("Unknown command: {other}")),
        }
        self.mode = AppMode::Normal;
    }

    fn trigger_manual_sync(&mut self) {
        if self.sync_engine.enabled {
            self.sync_engine.force_sync();
            self.state.set_status("Manual sync requested");
            self.state.log("Sync: manual sync requested");
        } else {
            self.state
                .set_status("Sync is disabled (no Git repo configured)");
        }
    }

    fn drain_sync(&mut self) -> bool {
        let mut received = false;
        while let Some(status) = self.sync_engine.try_recv() {
            received = true;
            self.send(Action::SyncStatus(status));
        }
        received
    }

    fn apply_sync_status(&mut self, status: &SyncStatus) {
        match status {
            SyncStatus::UpdatedRemoteData => {
                self.reload_from_disk();
                self.state.set_status("Pulled remote changes");
                self.state.log("Sync: pulled remote changes");
            }
            SyncStatus::Offline => {
                self.state
                    .set_status("Offline \u{2014} will sync when connection returns");
                self.state.log("Sync: offline, will retry");
            }
            SyncStatus::Conflict(backup) => {
                self.reload_from_disk();
                self.state.set_status(format!(
                    "Sync conflict \u{2014} local backup saved to {backup}"
                ));
                self.state
                    .log(format!("Sync: conflict, backup saved to {backup}"));
            }
            SyncStatus::Error(e) => {
                self.state.set_status(format!("Sync error: {e}"));
                self.state.log(format!("Sync: error \u{2014} {e}"));
            }
            _ => {}
        }
        self.state.sync_status = status.clone();
    }

    fn reload_from_disk(&mut self) {
        if let Ok(items) = self.store.load() {
            self.state.items = items;
            self.state.refresh_cache();
            self.send(Action::DataReloaded);
        }
    }

    pub fn selected_index(&self) -> Option<usize> {
        match self.state.view_mode {
            ViewMode::Table => self.table.selected_index(&self.state),
            ViewMode::Kanban => self.kanban.selected_index(&self.state),
        }
    }

    pub fn pending_chord(&self) -> Option<&'static str> {
        match self.state.view_mode {
            ViewMode::Table => self.table.pending_label(),
            ViewMode::Kanban => self.kanban.pending_label(),
        }
    }

    pub fn render(&mut self, f: &mut Frame) {
        self.state.selection = self.selected_index();

        let full = f.area();
        let screen = layout::split_screen(full);
        let focus = self.state.focus;

        header::draw_header(f, &self.state, screen.header);
        layout::draw_rule(f, screen.rule);

        // Panes that don't fit are never dropped: the focused one takes over the main area.
        let body = layout::split_body(screen.body);
        if let Some(area) = body.sidebar {
            self.nav
                .render(f, area, &self.state, focus == FocusPane::Nav);
        }
        match focus {
            FocusPane::Nav if body.sidebar.is_none() => {
                self.nav.render(f, body.main, &self.state, true);
            }
            FocusPane::Inspector if body.inspector.is_none() => {
                self.inspector.render(f, body.main, &self.state, true);
            }
            _ => {
                let main_focused = focus == FocusPane::Main;
                match self.state.view_mode {
                    ViewMode::Table => self.table.render(f, body.main, &self.state, main_focused),
                    ViewMode::Kanban => self.kanban.render(f, body.main, &self.state, main_focused),
                }
            }
        }
        if let Some(area) = body.inspector {
            self.inspector
                .render(f, area, &self.state, focus == FocusPane::Inspector);
        }

        let input: Option<&TextInput> = match self.mode {
            AppMode::Creating | AppMode::Editing => self.editor.text_input(),
            AppMode::Filtering | AppMode::Command => Some(self.cmdline.text_input()),
            AppMode::Normal | AppMode::Help => None,
        };
        let ctx = FooterCtx {
            mode: self.mode,
            position: self.table.position(&self.state),
            input,
            pending: self.pending_chord(),
        };
        footer::draw_status_line(f, screen.status, &self.state, &ctx);
        footer::draw_hints(f, screen.bottom, &self.state, &ctx);

        match self.mode {
            AppMode::Creating | AppMode::Editing => {
                self.editor.render(f, full, &self.state, true);
            }
            AppMode::Filtering | AppMode::Command => {
                self.cmdline.render(f, screen.bottom, &self.state, true);
            }
            AppMode::Help => self.help.render(f, full, &self.state, true),
            AppMode::Normal => {}
        }
    }
}
