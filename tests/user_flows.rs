use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use hopes::{
    action::Action,
    app::{App, AppMode},
    domain::{FocusPane, Item, NavSection, Priority, Status, ViewMode},
    systems::{
        storage::{JsonStore, Storage},
        sync::{SyncEngine, SyncStatus},
    },
};
use ratatui::{Terminal, backend::TestBackend};
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicU32, Ordering},
    time::Duration,
};

static COUNTER: AtomicU32 = AtomicU32::new(0);

struct Session {
    app: App,
    dir: PathBuf,
}

fn seed() -> Vec<Item> {
    let mut items = vec![
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
    items[0].priority = Priority::Urgent;
    items[1].priority = Priority::High;
    items[2].priority = Priority::Medium;
    items
}

fn fresh_dir() -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("hopes-flow-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn open_app(dir: &Path) -> App {
    let store: Box<dyn Storage> = Box::new(JsonStore::at_dir(dir.to_path_buf()));
    let sync = SyncEngine::spawn(
        dir.to_path_buf(),
        "origin".to_string(),
        "main".to_string(),
        Duration::from_secs(5),
        false,
        false,
    );
    App::with_store(store, sync)
}

impl Session {
    fn with_items(items: Vec<Item>) -> Self {
        let dir = fresh_dir();
        JsonStore::at_dir(dir.clone()).save(&items).unwrap();
        Self {
            app: open_app(&dir),
            dir,
        }
    }

    fn new() -> Self {
        Self::with_items(seed())
    }

    fn press(&mut self, code: KeyCode) {
        self.press_with(code, KeyModifiers::NONE);
    }

    fn press_with(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        self.app.handle_key(KeyEvent::new_with_kind(
            code,
            modifiers,
            KeyEventKind::Press,
        ));
        self.app.process_actions();
    }

    fn type_str(&mut self, text: &str) {
        for c in text.chars() {
            self.press(KeyCode::Char(c));
        }
    }

    fn dispatch(&mut self, action: Action) {
        self.app.send(action);
        self.app.process_actions();
    }

    fn items(&self) -> &[Item] {
        &self.app.state.items
    }

    fn cursor(&self) -> Option<usize> {
        self.app.selected_index()
    }

    fn saved(&self) -> Vec<Item> {
        JsonStore::at_dir(self.dir.clone()).load().unwrap()
    }

    fn screen(&mut self, width: u16, height: u16) -> String {
        render(&mut self.app, width, height)
    }
}

fn render(app: &mut App, width: u16, height: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|f| app.render(f)).unwrap();
    let buffer = terminal.backend().buffer();
    let mut out = String::new();
    for y in 0..height {
        for x in 0..width {
            out.push_str(buffer[(x, y)].symbol());
        }
        out.push('\n');
    }
    out
}

#[test]
fn vim_motions_move_the_cursor() {
    let mut s = Session::new();
    assert_eq!(s.cursor(), Some(0));

    s.press(KeyCode::Char('j'));
    assert_eq!(s.cursor(), Some(1));

    s.press(KeyCode::Char('G'));
    assert_eq!(s.cursor(), Some(2));

    s.press(KeyCode::Char('j'));
    assert_eq!(s.cursor(), Some(0));

    s.press(KeyCode::Char('k'));
    assert_eq!(s.cursor(), Some(2));

    s.type_str("gg");
    assert_eq!(s.cursor(), Some(0));
}

#[test]
fn tab_cycles_panes() {
    let mut s = Session::new();
    assert_eq!(s.app.state.focus, FocusPane::Main);
    s.press(KeyCode::Tab);
    assert_eq!(s.app.state.focus, FocusPane::Inspector);
    s.press(KeyCode::Tab);
    assert_eq!(s.app.state.focus, FocusPane::Nav);
    s.press(KeyCode::BackTab);
    assert_eq!(s.app.state.focus, FocusPane::Inspector);
}

#[test]
fn a_stray_key_abandons_a_half_typed_chord() {
    let mut s = Session::new();
    s.press(KeyCode::Char('d'));
    assert_eq!(s.app.pending_chord(), Some("d..."));

    s.press(KeyCode::Char('j'));
    assert_eq!(s.app.pending_chord(), None);

    s.press(KeyCode::Char('d'));
    assert_eq!(s.items().len(), 3);
}

#[test]
fn space_cycles_status() {
    let mut s = Session::new();
    assert_eq!(s.items()[0].status, Status::Todo);

    s.press(KeyCode::Char(' '));
    assert_eq!(s.items()[0].status, Status::InProgress);
    s.press(KeyCode::Char(' '));
    assert_eq!(s.items()[0].status, Status::Complete);
    s.press(KeyCode::Char(' '));
    assert_eq!(s.items()[0].status, Status::Todo);
}

#[test]
fn priority_keys_cycle_and_set() {
    let mut s = Session::new();
    assert_eq!(s.items()[0].priority, Priority::Urgent);

    s.press(KeyCode::Char('p'));
    assert_eq!(s.items()[0].priority, Priority::High);

    s.press(KeyCode::Char('P'));
    assert_eq!(s.items()[0].priority, Priority::Urgent);

    s.press(KeyCode::Char('4'));
    assert_eq!(s.items()[0].priority, Priority::Low);
}

#[test]
fn shift_j_and_k_reorder_and_the_cursor_follows() {
    let mut s = Session::new();
    s.press(KeyCode::Char('J'));
    assert_eq!(s.items()[0].title, "Design responsive TUI");
    assert_eq!(s.items()[1].title, "Write compiler backend");
    assert_eq!(s.cursor(), Some(1));

    s.press(KeyCode::Char('K'));
    assert_eq!(s.items()[0].title, "Write compiler backend");
    assert_eq!(s.cursor(), Some(0));
}

#[test]
fn dd_deletes_and_u_restores_in_place() {
    let mut s = Session::new();
    s.type_str("dd");
    assert_eq!(s.items().len(), 2);
    assert_eq!(s.items()[0].title, "Design responsive TUI");

    s.press(KeyCode::Char('u'));
    assert_eq!(s.items().len(), 3);
    assert_eq!(s.items()[0].title, "Write compiler backend");
    assert_eq!(s.cursor(), Some(0));
}

#[test]
fn deleting_the_last_row_keeps_the_cursor_on_a_task() {
    let mut s = Session::new();
    s.press(KeyCode::Char('G'));
    s.press(KeyCode::Char('x'));
    assert_eq!(s.items().len(), 2);
    assert_eq!(s.cursor(), Some(1));
}

#[test]
fn deleting_everything_leaves_no_selection() {
    let mut s = Session::new();
    for _ in 0..3 {
        s.press(KeyCode::Char('x'));
    }
    assert!(s.items().is_empty());
    assert_eq!(s.cursor(), None);

    s.press(KeyCode::Char('x'));
    assert!(s.items().is_empty());
}

#[test]
fn creating_a_task_adds_selects_and_saves_it() {
    let mut s = Session::new();
    s.press(KeyCode::Char('a'));
    assert_eq!(s.app.mode(), AppMode::Creating);
    s.type_str("fresh");
    s.press(KeyCode::Enter);

    assert_eq!(s.app.mode(), AppMode::Normal);
    assert_eq!(s.items().len(), 4);
    assert_eq!(s.cursor(), Some(3));
    assert_eq!(s.saved().last().unwrap().title, "fresh");
}

#[test]
fn ctrl_s_saves_from_any_field() {
    let mut s = Session::new();
    s.press(KeyCode::Char('a'));
    s.type_str("tagged");
    s.press(KeyCode::Tab);
    s.press(KeyCode::Tab);
    s.press(KeyCode::Tab);
    s.type_str("a, b");
    s.press_with(KeyCode::Char('s'), KeyModifiers::CONTROL);

    let created = s.items().last().unwrap();
    assert_eq!(created.title, "tagged");
    assert_eq!(created.tags, vec!["a", "b"]);
}

#[test]
fn an_empty_title_keeps_the_form_open() {
    let mut s = Session::new();
    s.press(KeyCode::Char('a'));
    s.press(KeyCode::Enter);
    assert_eq!(s.app.mode(), AppMode::Creating);
    assert_eq!(s.items().len(), 3);
}

#[test]
fn editing_loads_and_saves_the_selected_task() {
    let mut s = Session::new();
    s.press(KeyCode::Char('e'));
    assert_eq!(s.app.mode(), AppMode::Editing);

    s.type_str("!");
    s.press(KeyCode::Enter);

    assert_eq!(s.app.mode(), AppMode::Normal);
    assert_eq!(s.items()[0].title, "Write compiler backend!");
    assert_eq!(s.items().len(), 3);
}

#[test]
fn escaping_the_form_twice_cancels_without_saving() {
    let mut s = Session::new();
    s.press(KeyCode::Char('a'));
    s.type_str("draft");

    s.press(KeyCode::Esc);
    assert_eq!(s.app.mode(), AppMode::Creating);
    s.press(KeyCode::Esc);
    assert_eq!(s.app.mode(), AppMode::Normal);
    assert_eq!(s.items().len(), 3);
}

#[test]
fn find_prompt_filters_and_puts_the_cursor_on_the_match() {
    let mut s = Session::new();
    s.press(KeyCode::Char('/'));
    assert_eq!(s.app.mode(), AppMode::Filtering);

    s.type_str("responsive");
    s.press(KeyCode::Enter);

    assert_eq!(s.app.mode(), AppMode::Normal);
    assert_eq!(s.app.state.tag_filter.as_deref(), Some("responsive"));
    assert_eq!(s.app.state.filtered_indices().len(), 1);
    assert_eq!(s.cursor(), Some(1));
}

#[test]
fn escape_cancels_find_without_filtering() {
    let mut s = Session::new();
    s.press(KeyCode::Char('/'));
    s.type_str("zzz");
    s.press(KeyCode::Esc);
    s.press(KeyCode::Esc);
    assert_eq!(s.app.mode(), AppMode::Normal);
    assert!(s.app.state.tag_filter.is_none());
    assert_eq!(s.app.state.filtered_indices().len(), 3);
}

#[test]
fn escape_in_normal_mode_clears_the_filter() {
    let mut s = Session::new();
    s.dispatch(Action::SetFilter(Some("#rust".to_string())));
    assert_eq!(s.app.state.filtered_indices().len(), 2);

    s.press(KeyCode::Esc);
    assert!(s.app.state.tag_filter.is_none());
    assert_eq!(s.app.state.filtered_indices().len(), 3);
}

#[test]
fn tag_filter_cycles_through_every_tag_then_clears() {
    let mut s = Session::new();
    let tag_count = s.app.state.tags().len();
    assert!(tag_count > 1);

    for _ in 0..tag_count {
        s.press(KeyCode::Char('t'));
        assert!(s.app.state.tag_filter.is_some());
    }
    s.press(KeyCode::Char('t'));
    assert!(s.app.state.tag_filter.is_none());
}

#[test]
fn filters_and_stats_reflect_the_tasks() {
    let mut s = Session::new();
    let stats = s.app.state.stats();
    assert_eq!(stats.total, 3);
    assert_eq!(stats.todo, 3);
    assert_eq!(stats.urgent, 1);
    assert_eq!(stats.high, 1);

    s.dispatch(Action::SetFilter(Some("#rust".to_string())));
    assert_eq!(s.app.state.filtered_indices().len(), 2);
    s.dispatch(Action::SetFilter(Some("responsive".to_string())));
    assert_eq!(s.app.state.filtered_indices().len(), 1);
}

#[test]
fn navigator_applies_views_and_enter_returns_to_the_list() {
    let mut s = Session::new();
    s.press(KeyCode::BackTab);
    assert_eq!(s.app.state.focus, FocusPane::Nav);

    s.press(KeyCode::Char('j'));
    assert_eq!(s.app.state.nav_section, NavSection::Todo);

    s.press(KeyCode::Char('5'));
    assert_eq!(s.app.state.nav_section, NavSection::Urgent);
    assert_eq!(s.app.state.filtered_indices().len(), 1);

    s.press(KeyCode::Enter);
    assert_eq!(s.app.state.focus, FocusPane::Main);
}

#[test]
fn task_commands_still_work_while_the_navigator_has_focus() {
    let mut s = Session::new();
    s.press(KeyCode::BackTab);
    assert_eq!(s.app.state.focus, FocusPane::Nav);

    s.press(KeyCode::Char('e'));
    assert_eq!(s.app.mode(), AppMode::Editing);
}

#[test]
fn v_toggles_between_table_and_board() {
    let mut s = Session::new();
    assert_eq!(s.app.state.view_mode, ViewMode::Table);
    s.press(KeyCode::Char('v'));
    assert_eq!(s.app.state.view_mode, ViewMode::Kanban);
    s.press(KeyCode::Char('v'));
    assert_eq!(s.app.state.view_mode, ViewMode::Table);
}

#[test]
fn shifting_a_card_moves_it_and_the_cursor_follows() {
    let mut s = Session::new();
    s.press(KeyCode::Char('v'));

    s.press(KeyCode::Char('L'));
    assert_eq!(s.items()[0].status, Status::InProgress);
    assert_eq!(s.cursor(), Some(0));

    s.press(KeyCode::Char('H'));
    assert_eq!(s.items()[0].status, Status::Todo);
}

#[test]
fn h_and_l_switch_board_columns() {
    let mut s = Session::new();
    s.press(KeyCode::Char('v'));
    assert!(s.cursor().is_some());

    s.press(KeyCode::Char('l'));
    assert_eq!(s.cursor(), None);

    s.press(KeyCode::Char('h'));
    assert!(s.cursor().is_some());
}

#[test]
fn tasks_added_from_the_board_start_in_that_column() {
    let mut s = Session::new();
    s.press(KeyCode::Char('v'));
    s.press(KeyCode::Char('l'));

    s.press(KeyCode::Char('a'));
    s.type_str("board task");
    s.press_with(KeyCode::Char('s'), KeyModifiers::CONTROL);

    assert_eq!(s.app.mode(), AppMode::Normal);
    let created = s.items().last().unwrap();
    assert_eq!(created.title, "board task");
    assert_eq!(created.status, Status::InProgress);
}

#[test]
fn table_paging_keys_do_nothing_on_the_board() {
    let mut s = Session::new();
    s.press(KeyCode::Char('v'));
    let before = s.cursor();
    s.press_with(KeyCode::Char('d'), KeyModifiers::CONTROL);
    s.press(KeyCode::PageDown);
    assert_eq!(s.cursor(), before);
    assert_eq!(s.items().len(), 3);
}

#[test]
fn colon_q_quits() {
    let mut s = Session::new();
    s.press(KeyCode::Char(':'));
    assert_eq!(s.app.mode(), AppMode::Command);
    s.type_str("q");
    s.press(KeyCode::Enter);
    assert!(s.app.should_quit());
}

#[test]
fn q_quits_from_normal_mode() {
    let mut s = Session::new();
    s.press(KeyCode::Char('q'));
    assert!(s.app.should_quit());
}

#[test]
fn unknown_commands_report_and_return_to_normal() {
    let mut s = Session::new();
    s.press(KeyCode::Char(':'));
    s.type_str("bogus");
    s.press(KeyCode::Enter);
    assert_eq!(s.app.mode(), AppMode::Normal);
    assert!(
        s.app
            .state
            .status()
            .unwrap_or_default()
            .contains("Unknown command")
    );
}

#[test]
fn sync_command_without_a_repo_just_reports() {
    let mut s = Session::new();
    s.press(KeyCode::Char(':'));
    s.type_str("sync");
    s.press(KeyCode::Enter);
    assert_eq!(s.app.mode(), AppMode::Normal);
    assert!(!s.app.should_quit());
    assert!(
        s.app
            .state
            .status()
            .unwrap_or_default()
            .contains("disabled")
    );
}

#[test]
fn help_opens_from_a_command_and_closes_with_q() {
    let mut s = Session::new();
    s.press(KeyCode::Char(':'));
    s.type_str("help");
    s.press(KeyCode::Enter);
    assert_eq!(s.app.mode(), AppMode::Help);

    s.press(KeyCode::Char('q'));
    assert_eq!(s.app.mode(), AppMode::Normal);
    assert!(!s.app.should_quit());
}

#[test]
fn every_change_is_saved_and_survives_a_restart() {
    let mut s = Session::new();
    s.press(KeyCode::Char(' '));
    s.press(KeyCode::Char('a'));
    s.type_str("later");
    s.press(KeyCode::Enter);

    assert_eq!(s.saved()[0].status, Status::InProgress);
    assert_eq!(s.saved().len(), 4);

    let reopened = open_app(&s.dir);
    assert_eq!(reopened.state.items.len(), 4);
    assert_eq!(reopened.state.items[3].title, "later");
}

#[test]
fn a_pull_that_shrinks_the_list_keeps_the_cursor_on_a_real_row() {
    let mut s = Session::new();
    s.press(KeyCode::Char('G'));
    assert_eq!(s.cursor(), Some(2));

    JsonStore::at_dir(s.dir.clone()).save(&seed()[..1]).unwrap();
    s.dispatch(Action::SyncStatus(SyncStatus::UpdatedRemoteData));

    assert_eq!(s.items().len(), 1);
    assert_eq!(s.cursor(), Some(0));
    assert_eq!(s.app.state.sync_status, SyncStatus::UpdatedRemoteData);
}

#[test]
fn the_main_screen_shows_tasks_status_line_and_hints() {
    let mut s = Session::new();
    let screen = s.screen(120, 32);
    assert!(screen.contains("Write compiler backend"), "{screen}");
    assert!(screen.contains("NORMAL"), "{screen}");
    assert!(screen.contains("1 / 3"), "{screen}");
    assert!(screen.contains("help"), "{screen}");
    assert!(screen.contains("Navigator"), "{screen}");
    assert!(screen.contains("Inspector"), "{screen}");
}

#[test]
fn the_find_prompt_shows_what_you_type_on_the_bottom_row() {
    let mut s = Session::new();
    s.press(KeyCode::Char('/'));
    s.type_str("tui");

    let screen = s.screen(120, 32);
    let last_line = screen.lines().last().unwrap();
    assert!(last_line.starts_with("/tui"), "{last_line:?}");
    assert!(screen.contains("FIND"), "{screen}");
}

#[test]
fn the_command_line_shows_on_the_bottom_row() {
    let mut s = Session::new();
    s.press(KeyCode::Char(':'));
    s.type_str("sy");

    let screen = s.screen(120, 32);
    assert!(screen.lines().last().unwrap().starts_with(":sy"));
    assert!(screen.contains("COMMAND"));
}

#[test]
fn the_form_and_help_popups_render() {
    let mut s = Session::new();
    s.press(KeyCode::Char('a'));
    let screen = s.screen(120, 32);
    assert!(screen.contains("New task"), "{screen}");

    s.press(KeyCode::Esc);
    s.press(KeyCode::Esc);
    s.press(KeyCode::Char('?'));
    let screen = s.screen(120, 32);
    assert!(screen.contains("Keyboard shortcuts"), "{screen}");
}

#[test]
fn the_board_renders_three_columns() {
    let mut s = Session::new();
    s.press(KeyCode::Char('v'));
    let screen = s.screen(120, 32);
    assert!(screen.contains("To Do"), "{screen}");
    assert!(screen.contains("In Progress"), "{screen}");
    assert!(screen.contains("Completed"), "{screen}");
}

#[test]
fn every_mode_renders_at_every_size() {
    let sizes = [
        (120, 32),
        (90, 24),
        (60, 20),
        (30, 8),
        (12, 5),
        (3, 2),
        (1, 1),
    ];
    let setups: [(&str, &[KeyCode]); 8] = [
        ("table", &[]),
        ("board", &[KeyCode::Char('v')]),
        ("new task", &[KeyCode::Char('a')]),
        ("edit task", &[KeyCode::Char('e')]),
        ("find", &[KeyCode::Char('/')]),
        ("command", &[KeyCode::Char(':')]),
        ("help", &[KeyCode::Char('?')]),
        ("navigator", &[KeyCode::BackTab]),
    ];

    for (name, keys) in setups {
        for (width, height) in sizes {
            let mut s = Session::new();
            for key in keys {
                s.press(*key);
            }
            let result =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| s.screen(width, height)));
            assert!(
                result.is_ok(),
                "rendering {name} at {width}x{height} panicked"
            );
        }
    }
}

#[test]
fn an_empty_workspace_renders_and_accepts_input() {
    let mut s = Session::with_items(Vec::new());
    assert_eq!(s.cursor(), None);

    let screen = s.screen(120, 32);
    assert!(screen.contains("Nothing here yet"), "{screen}");

    for key in [
        KeyCode::Char('j'),
        KeyCode::Char('G'),
        KeyCode::Char(' '),
        KeyCode::Char('e'),
        KeyCode::Char('p'),
    ] {
        s.press(key);
    }
    assert!(s.items().is_empty());
    assert_eq!(s.app.mode(), AppMode::Normal);
}
