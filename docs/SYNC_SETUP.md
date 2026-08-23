# Multi-Device Sync Setup

`hopes` can keep `items.json` in sync across machines by treating your data
directory as a git repository and pushing/pulling it in the background. This
is opt-in and fully automatic once set up: if the data directory isn't a git
repo, sync just stays disabled and nothing changes about how the app behaves.

---

## How it works, briefly

- Every task mutation (add, edit, delete, status/priority change, reorder)
  saves `items.json` to disk immediately, then schedules a background push.
- The push is debounced: rapid edits collapse into a single commit after
  `debounce_seconds` of quiet (default `5`).
- On launch, `hopes` does a `git pull --rebase --autostash` first (if
  `pull_on_startup = true`), so you start with the latest data.
- A badge in the top-right of the header shows the current state: `☁ Idle`,
  `☁ Syncing...`, `☁ Synced`, `☁ Offline`, `☁ Conflict`, `☁ Sync Error`.
- Press `S` or `F5` any time to force an immediate sync instead of waiting
  for the debounce window.
- If a genuine merge conflict happens (both devices edited before either
  synced), `hopes` keeps the remote version, backs your local copy up as
  `items.json.local-backup-<timestamp>` next to `items.json`, and shows a
  red `Conflict` status with the backup path.

---

## One-time GitHub setup (do this once, on any machine)

You need a **private** GitHub repo dedicated to holding your `items.json`.
It's separate from the `hopes` source code repo.

```bash
gh repo create hopes-data --private
```

(No `gh`? Create it manually at github.com — private, empty, no README.)

---

## Linux

`hopes` defaults to `~/.local/share/hopes/items.json` with no config file
needed — the default data directory *is* where you'll put the git repo, so
there's nothing to point anywhere.

```bash
# Build the app
git clone git@github.com:AmaneKai/hopes.git ~/Github/hopes
cd ~/Github/hopes
cargo build --release

# Wire up sync in the default data directory
cd ~/.local/share/hopes        # created automatically on first run of hopes,
                                # or: mkdir -p ~/.local/share/hopes
git init -b main
git remote add origin git@github.com:AmaneKai/hopes-data.git
git add items.json
git commit -m "Initial commit"
git push -u origin main

# Run it
~/Github/hopes/target/release/hopes
```

No `~/.config/hopes/config.toml` is required — the defaults already match.

**If this is a second/third Linux machine** (data dir doesn't exist yet),
just clone straight into place instead of `git init`:

```bash
git clone git@github.com:AmaneKai/hopes-data.git ~/.local/share/hopes
```

---

## macOS

macOS's default data directory (`~/Library/Application Support/hopes`) is
awkward to `cd` into by hand, so these steps clone the sync repo somewhere
normal (`~/hopes-data`) and use a config file to point `hopes` at it.

```bash
# 1. Homebrew (skip if already installed)
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# 2. git, gh, Rust
brew install git gh
curl https://sh.rustup.rs -sSf | sh
# restart Terminal, or: source "$HOME/.cargo/env"

# 3. Authenticate GitHub CLI (offers to generate + upload an SSH key)
gh auth login -p ssh

# 4. Build the app
git clone git@github.com:AmaneKai/hopes.git ~/Github/hopes
cd ~/Github/hopes
cargo build --release

# 5. Clone the sync data
git clone git@github.com:AmaneKai/hopes-data.git ~/hopes-data

# 6. Point hopes at it
mkdir -p ~/Library/Application\ Support/hopes
cat > ~/Library/Application\ Support/hopes/config.toml <<EOF
[sync]
enabled = true
debounce_seconds = 5
data_dir = "$HOME/hopes-data"
remote = "origin"
branch = "main"
pull_on_startup = true
EOF

# 7. Run it
~/Github/hopes/target/release/hopes
```

Optional: put `hopes` on your `PATH` so you can just type `hopes`:

```bash
ln -s ~/Github/hopes/target/release/hopes /usr/local/bin/hopes
```

You should see your task list and a `☁ Synced` badge top-right.

---

## `config.toml` reference

Location: `~/.config/hopes/config.toml` (Linux) or
`~/Library/Application Support/hopes/config.toml` (macOS). Entirely
optional — every field below is already the default, so a missing file
behaves identically to a file containing these values:

```toml
[sync]
enabled = true              # turn background git sync on/off entirely
debounce_seconds = 5        # seconds of quiet after an edit before committing+pushing
# data_dir = "/path/to/dir" # override where hopes looks for items.json
remote = "origin"           # git remote name to push/pull
branch = "main"             # git branch to push/pull
pull_on_startup = true      # git pull --rebase --autostash on launch
```

Only set `data_dir` when the OS default data directory isn't where you
cloned `hopes-data` — see the macOS section above.

---

## Troubleshooting

- **Badge never appears / stays blank** — the data directory isn't a git
  repo (no `.git` folder), so sync auto-disabled itself. Nothing is broken;
  just finish the one-time setup above.
- **`☁ Offline`** — a git command couldn't reach GitHub (no network, or SSH
  auth failing). Local edits still save fine; it'll retry and catch up once
  connectivity is back, or press `S` to force a retry.
- **`☁ Conflict`** — check the status line for the backup file path
  (`items.json.local-backup-<timestamp>`), diff it against the now-current
  `items.json` if you want to hand-merge anything, then delete the backup.
- **SSH permission denied** — confirm `gh auth status` shows you're logged
  in with the `ssh` protocol, and that `ssh -T git@github.com` succeeds.
