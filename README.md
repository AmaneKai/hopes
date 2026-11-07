# Hopes

A keyboard-centric terminal task tracker and Kanban board built in Go with Bubble Tea.
Designed around workspaces, native Vim motions, and a quiet two-pane layout.

---

## Features

- **Workspaces**: Every task lives in exactly one workspace, and you only ever see one workspace
    at a time. `Tab` / `Shift-Tab` cycle through them; the sidebar lists them with their open
    task counts. `:ws name` opens or creates one, `:rename name` renames the current one.
- **Two-Pane Layout**: A fixed workspace sidebar beside the task list once the terminal is
    `>= 72` columns wide; below that the list takes the whole width and the header shows which
    workspace you are in.
- **Dual View Modes (`v`)**: Switch between a dense **Table** and a 3-column **Kanban Board**
    (`To Do`, `In Progress`, `Completed`), both scoped to the current workspace.
- **Native Vim Motions**: `j`/`k` navigation, `gg`/`G` bounds jump, `<C-d>`/`<C-u>` half-page
    jumps, `dd` deletion, `u` undo, and `J`/`K` task reordering. Lists stop at either end rather
    than wrapping, and keep a few rows of context around the cursor while scrolling.
- **Search (`/`)**: Instant substring search across titles, notes, priority and status within
    the current workspace.
- **Task Notes**: Multi-line notes, edited in the task form and previewed on Kanban cards.
- **Dynamic Sorting (`s`)**: Sort on the fly by Natural order, Priority, Status, or Title.
- **Atomic Local Persistence**: Crash-resilient JSON storage written atomically via temporary
    staging files.

---

## Installation

### Prerequisites

- Go 1.26+

### Building from Source

```bash
git clone https://github.com/AmaneKai/hopes.git
cd hopes
go build -trimpath -ldflags "-s -w" .
./hopes
```

### Installation via Go

```bash
go install github.com/AmaneKai/hopes@latest
```

The only dependency is Bubble Tea, which handles the terminal and keyboard on macOS, Linux and
Windows. Drawing, layout, the config parser and the vim editing engine are all in this repo.

## Keybindings Reference

### Workspaces

| Key | Context | Action |
| :--- | :--- | :--- |
| `Tab` / `BackTab` | Normal | Next / previous workspace (wraps around) |
| `h` / `<C-w>` | Normal | Focus the workspace sidebar |
| `j` / `k` | Sidebar | Switch to the workspace below / above |
| `1` ..= `9` | Sidebar | Jump to a workspace by position |
| `Enter` / `l` | Sidebar | Back to the task list |
| `:ws name` | Command | Open a workspace, creating it if it does not exist |
| `:rename name` | Command | Rename the current workspace |

A new workspace stays in the sidebar while it is open; once it holds a task it stays for good.
The task form's **Workspace** field moves a task to another workspace (with completion).

### Navigation

| Key | Context | Action |
| :--- | :--- | :--- |
| `j` / `Down` | Global | Move selection down |
| `k` / `Up` | Global | Move selection up |
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
| `J` / `K` | Table View | Reorder task down / up within the workspace |
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

### Search & General

| Key | Context | Action |
| :--- | :--- | :--- |
| `/` | Normal | Search the current workspace |
| `<Esc>` | Normal | Clear the search |
| `s` | Normal | Cycle sort order |
| `?` | Normal | Open keybindings cheatsheet overlay |
| `q` / `<C-c>` | Normal | Exit application |

### Modal Editing

| Key | Context | Action |
| :--- | :--- | :--- |
| `Tab` / `<C-j>` | Modal | Focus next form field (completes a workspace name first) |
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
- **Windows**: `%LOCALAPPDATA%\hopes\items.json`

Writes are atomic: updates serialize to a temporary file (`.tmp`) and sync to disk before being renamed to prevent data corruption during unexpected shutdowns.

Each task stores its `workspace` by name. Files written before workspaces existed still load:
a task's first tag becomes its workspace, and tasks without tags go to `Inbox`. Old due dates
and repeat rules are ignored and dropped on the next save.

---

## Configuration

`config.toml` lives in `~/.config/hopes/` on Linux, `~/Library/Application Support/hopes/` on
macOS and `%APPDATA%\hopes\` on Windows. Every key is optional:

```toml
[sync]
enabled = true
debounce_seconds = 5
data_dir = "/path/to/hopes-data"
remote = "origin"
branch = "main"
pull_on_startup = true

[storage]
backend = "json"            # or "exec"
dir = "/path/to/data"
command = ["my-store"]      # exec only: called with `load` or `save`, JSON on stdout / stdin
```

A config that does not parse, or holds a value of the wrong type, is ignored as a whole and the
defaults apply. The `exec` backend can wrap any store, such as a `sqlite3` script.

---

## Architecture

Components own their own UI state and talk to the rest of the app through actions. Keys become
`Action`s; `App.apply` is the only code that changes the task list, persists it, or touches sync.
After each action every component gets an `update` call so it can react (clamp a cursor, reset a
scroll). Bubble Tea delivers keys, pastes and sync updates to `Model.Update`, and `Model.View`
draws the app onto a cell canvas that is turned into one frame.

```
key press --> component.handleKey --> Action --> App.apply
                                                   |-- mutates AppState, saves, requests sync
                                                   '-- component.update(action) on every component
```

```
main.go                    Starts the program, or runs the detached quit-time sync flush
internal/
├── domain/                Tasks, priority, status, and the workspace-scoped AppState
├── ui/
│   ├── program.go         Bubble Tea model: keys, pastes and sync updates in, frames out
│   ├── app.go, apply.go   Key routing and every state change
│   ├── tasks.go           Task create, edit, delete, undo, reorder
│   ├── canvas.go          Cell grid with styles, boxes and wide-character handling
│   ├── table.go           Row cursor, scroll padding, gg / dd chords
│   ├── board.go           Columns of cards, per-column cursors and scroll
│   ├── sidebar.go         Workspace list
│   ├── editor.go          Task form; editor_view.go draws it
│   ├── input.go           Vim-style editing shared by every text field
│   └── chrome.go, cmdline.go, help.go, render.go, theme.go
├── textedit/              Pure vim-motion string math
├── storage/               JSON and exec backends behind one interface
├── gitsync/               Git wrappers and the debounced sync worker
├── config/                config.toml loading with a small TOML-subset parser
├── proc/                  Child processes with capped output and deadlines
├── viewstate/, paths/, textfile/
```

Tests sit next to the code they cover. The UI tests drive the app with key presses and check
task state, the file on disk, and the rendered screen as plain text.

---

## Development

```bash
gofmt -l .        # formatting
go vet ./...      # static checks
go test ./...     # everything, including git sync against real temporary repos
```

---

## License

MIT License.
