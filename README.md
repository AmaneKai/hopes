# hopes-C

C + ncurses port of [hopes](https://github.com/AmaneKai/hopes): a keyboard-driven task tracker and
Kanban board. Same features, same `items.json` format, same keybindings (press `?`).

Built to be small and idle: ~90 KB binary, no timers, no threads. The main loop sleeps in
`poll()` until a key, a sync message or a deadline arrives, so an idle session uses 0% CPU and ~2 MB.

## Build

    make                 # build/hopes
    make SQLITE=1        # also compile the sqlite storage backend
    make test            # ASan/UBSan test run, no ncurses needed
    make check           # everything CI runs: lint, warnings-as-errors, tests, size budget, git sync
    make install         # PREFIX=/usr/local

Needs a C11 compiler and a curses library:

| OS | Curses | Build |
| --- | --- | --- |
| macOS | system ncurses | `make` |
| Linux | `libncursesw` (`apt install libncurses-dev`) | `make` |
| Windows | PDCursesMod via MSYS2: `pacman -S make mingw-w64-x86_64-gcc mingw-w64-x86_64-pdcurses` | `make` in a MinGW64 shell |

Windows builds use a thread for sync and a re-launched `hopes.exe --sync-flush` to push on quit; macOS and
Linux fork instead. Config and data live in `%APPDATA%` and `%LOCALAPPDATA%` on Windows.
`compile_commands.json` is generated for clangd (`make compile_commands.json`).

## Layout

stb style: one translation unit. `src/main.c` defines `HOPES_IMPLEMENTATION` and `src/hopes.h`
pulls in the module headers below.

    amane.h      arrays, string builder, UTF-8 (your single-header lib)
    util.h       memory, growable buffers, UTF-8
    dates.h      calendar math, natural-language due dates, repeat rules
    textedit.h   vim motions and the modal text field
    json.h       reader/writer, byte-compatible with serde_json
    config.h     config.toml subset and platform paths
    store.h      json / exec / sqlite backends
    sync.h       git sync in a forked child, reporting over a pipe
    app.h        state, actions and key handling (no terminal I/O)
    view.h       persisted view state (view, sort, nav section, tag filter)
    ui.h         ncurses rendering and input

## Config

Same `config.toml` as the Rust version (`~/.config/hopes/` on Linux,
`~/Library/Application Support/hopes/` on macOS). `HOPES_CONFIG=/path/to/file` overrides the location.
See the original repo's `docs/SYNC_SETUP.md` for git sync.

## CI

`make check` is exactly what CI runs, so green locally means green there.

- `make lint` enforces the house rules: no comments, at most 3 levels of nesting (`scripts/lint.c`).
- `make strict` builds with `-Wall -Wextra -Werror` (CI runs it with gcc and clang).
- `make test` runs the unit and storage tests under ASan and UBSan (plus LeakSanitizer on Linux).
- `make size` fails if the binary outgrows its budget (90 KB on macOS, 79 KB on Linux).
- `make sync-test` drives two real instances through a bare git remote in tmux (`tests/sync.sh`).

Pushing a `c-v*.*.*` tag builds, verifies and publishes Linux, macOS (arm64 and Intel) and Windows archives with checksums.
