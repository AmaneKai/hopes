# Hopes

A keyboard-centric terminal task tracker and Kanban board built in Rust with Ratatui and Crossterm. 
Designed with native Vim motions, zero memory overhead during rendering, and responsive multi-pane 
layout management.

---

## Features

- **Responsive 3-Pane Dashboard**: Automatically scales from compact single-pane to a 3-pane power 
    workspace (`>= 115` columns) featuring Navigator, Center Data Grid / Kanban, 
    and Inspector & Analytics.
- **Dual View Modes (`v`)**: Seamlessly switch between a high-density **Table Grid** and 
    a 3-column **Kanban Board** (`[ ] Todo`, `[-] In Progress`, `[x] Completed`).
- **Interactive Pane Switching (`Tab` / `<C-w>`)**: Cycle focus between Navigator (`[NAV]`), 
    Main Workspace (`[MAIN]`), and Task Inspector (`[INSPECTOR]`).
- **Native Vim Motions**: `j`/`k` navigation, `gg`/`G` bounds jump, `<C-d>`/`<C-u>` half-page 
    jumps, `dd` deletion, `u` undo stack, and `J`/`K` task reordering.
- **Live Filtering & Tags (`/`, `t`)**: Instant substring search across titles, descriptions, and 
    tags with zero-allocation byte-level matching.
- **Task Notes & Descriptions**: Multi-line description support visible in the right-hand Inspector 
    pane with vertical scrolling.
- **Dynamic Sorting (`s`)**: Sort on the fly by Natural order, Priority, Status, or Alphabetical 
    title.
- **Atomic Local Persistence**: Thread-safe, crash-resilient JSON storage written atomically via 
    temporary staging files.

---

## Installation

### Prerequisites

- Rust 1.80+ (2024 Edition)
- Cargo

### Building from Source

```bash
# Clone the repository
git clone https://github.com/AmaneKai/hopes.git
cd hopes

# Build optimized release binary
cargo build --release

# Run
./target/release/hopes
```

#### Smallest binary

`cargo build --release` already uses size-optimized settings. For roughly 30% less on top of that,
`scripts/build-small.sh` rebuilds the standard library for size. It needs nightly with `rust-src`
(`rustup toolchain install nightly --component rust-src`) and keeps panic messages.

### Installation via Cargo

```bash
cargo install --path .
```

## Keybindings Reference

### Navigation & Focus

| Key | Context | Action |
| :--- | :--- | :--- |
| `j` / `Down` | Global | Move selection / scroll down |
| `k` / `Up` | Global | Move selection / scroll up |
| `Tab` / `<C-w>` | Normal | Switch focus to next pane (`Nav` -> `Main` -> `Inspector`) |
| `BackTab` | Normal | Switch focus to previous pane |
| `gg` / `Home` | Normal | Jump to top |
| `G` / `End` | Normal | Jump to bottom |
| `<C-d>` / `PageDown` | Normal | Jump down 5 items |
| `<C-u>` / `PageUp` | Normal | Jump up 5 items |

### Task Management

| Key | Context | Action |
| :--- | :--- | :--- |
| `Space` | Main Pane | Advance status (`Todo` -> `In Progress` -> `Complete`) |
| `a` / `o` / `i` | Normal | Open modal to create new task |
| `e` / `c` / `Enter` | Normal | Edit selected task |
| `dd` / `x` | Normal | Delete selected task |
| `u` | Normal | Undo last deletion |
| `J` / `K` | Table View | Reorder task down / up |
| `p` / `P` | Normal | Cycle priority next / previous |
| `1` / `2` / `3` / `4` | Main Pane | Set priority (`Urgent`, `High`, `Medium`, `Low`) |

### Kanban Controls

| Key | Context | Action |
| :--- | :--- | :--- |
| `v` | Normal | Toggle View Mode (Table Grid <-> Kanban Board) |
| `h` / `Left` | Kanban View | Move cursor to column on the left |
| `l` / `Right` | Kanban View | Move cursor to column on the right |
| `H` | Kanban View | Shift selected task one column left |
| `L` / `Space` | Kanban View | Shift selected task one column right |

### Filtering & Presets

| Key | Context | Action |
| :--- | :--- | :--- |
| `/` | Normal | Open live search / filter prompt |
| `t` | Normal | Cycle through active tag filters |
| `1` ..= `5` | Nav Pane | Filter by View (`All`, `Todo`, `WIP`, `Done`, `Urgent`) |
| `<Esc>` | Normal | Clear active filter and reset view |
| `?` | Normal | Open keybindings cheatsheet overlay |
| `q` / `<C-c>` | Normal | Exit application |

### Modal Editing

| Key | Context | Action |
| :--- | :--- | :--- |
| `Tab` / `<C-j>` | Modal | Focus next form field |
| `BackTab` / `<C-k>` | Modal | Focus previous form field |
| `<Enter>` / `<C-s>` | Modal | Save and commit changes |
| `<C-w>` | Modal | Delete preceding word |
| `<C-u>` | Modal | Clear current input field |
| `<Esc>` | Modal | Cancel without saving |

---

## Storage & Configuration

Task items are stored locally in JSON format at:

- **Linux**: `~/.local/share/hopes/items.json`
- **macOS**: `~/Library/Application Support/hopes/items.json`
- **Windows**: `%APPDATA%\hopes\items.json`

Writes are atomic: updates serialize to a temporary file (`.tmp`) and sync to disk before being renamed to prevent data corruption during unexpected shutdowns.

---

## Architecture

Components own their own UI state and talk to the rest of the app through
messages. Keys become `Action`s; `App::process_action` is the only code that
changes the task list, persists it, or touches sync. After each action every
component gets an `update` call so it can react (clamp a cursor, reset a scroll).

```
key press --> Component::handle_key --> Action --> App::process_action
                                                        |-- mutates AppState, saves, requests sync
                                                        '-- Component::update(&action) on every component
```

```
src/
├── lib.rs               # The whole app as a library, so tests can drive it
├── main.rs              # Thin binary: terminal setup/teardown, detached sync flush
├── app.rs               # Event loop, action queue, key routing, process_action
├── action.rs            # The Action enum: everything that can happen
├── domain/              # Pure data, no UI knowledge
│   ├── mod.rs           # AppState (items, filter/sort, derived caches) and view enums
│   ├── item.rs          # Task model with multi-line notes
│   ├── priority.rs      # Priority levels (Urgent, High, Med, Low)
│   ├── status.rs        # Status tracking (Todo, WIP, Done)
│   └── audit.rs         # In-memory activity log
├── ui/
│   ├── mod.rs           # Component and TaskView traits
│   ├── layout.rs        # Screen and pane geometry
│   ├── text_input.rs    # Vim-style editing engine shared by every text field
│   ├── popup.rs, field.rs, theme.rs
│   └── components/      # Each owns its own state
│       ├── table.rs     # Row cursor, gg / dd chords
│       ├── kanban.rs    # Active column and per-column card cursors
│       ├── nav.rs       # Navigator cursor
│       ├── inspector.rs # Details scroll
│       ├── editor.rs    # New/edit task form and its draft text
│       ├── cmdline.rs   # `:` command and `/` find input row
│       ├── help.rs      # Shortcut popup and its scroll
│       └── header.rs, footer.rs
├── systems/             # IO, never touches UI state
│   ├── storage/         # json, sqlite and exec backends behind one trait
│   └── sync/            # Git sync worker thread and detached quit-time push
├── config.rs, event.rs, tui.rs
└── utils/textedit.rs    # Pure vim-motion string math
tests/                   # All tests live here and use only the public API
├── user_flows.rs        # Key presses in, task state / disk / rendered screen out
├── editor.rs            # The task form as a component
├── text_input.rs        # Vim field editing engine
├── textedit.rs          # Vim motion string math
├── git.rs, sync_engine.rs        # Git wrappers and the sync worker, on real repos
└── storage_exec.rs, storage_sqlite.rs
```

Adding a feature is usually one new `Action` variant, one arm in `process_action`,
and a key binding in the component that should own it.

---

## Development

```bash
# Check formatting
cargo fmt --check

# Run linter
cargo clippy --all-targets -- -D warnings

# Execute test suite
cargo test --all-targets
```

---

## License

MIT License.
