pub mod git;

use std::{
    cell::Cell,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc::{self, Receiver, RecvTimeoutError, Sender},
    thread,
    time::{Duration, Instant},
};

#[cfg(unix)]
use std::os::unix::process::CommandExt;
#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[derive(Debug)]
pub enum SyncRequest {
    SchedulePush,
    ForceSync,
    Shutdown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SyncStatus {
    Disabled,
    Idle,
    Syncing(String),
    Synced,
    UpdatedRemoteData,
    Offline,
    Conflict(String),
    Error(String),
}

pub struct SyncEngine {
    tx: Sender<SyncRequest>,
    rx: Receiver<SyncStatus>,
    data_dir: PathBuf,
    remote: String,
    branch: String,
    dirty: Cell<bool>,
    pub enabled: bool,
}

impl SyncEngine {
    pub fn spawn(
        data_dir: PathBuf,
        remote: String,
        branch: String,
        debounce: Duration,
        pull_on_startup: bool,
        enabled: bool,
    ) -> Self {
        let (req_tx, req_rx) = mpsc::channel::<SyncRequest>();
        let (status_tx, status_rx) = mpsc::channel::<SyncStatus>();
        let (engine_dir, engine_remote, engine_branch) =
            (data_dir.clone(), remote.clone(), branch.clone());

        if !enabled || !git::is_repo(&data_dir) {
            let _ = status_tx.send(SyncStatus::Disabled);
            return Self {
                tx: req_tx,
                rx: status_rx,
                data_dir: engine_dir,
                remote: engine_remote,
                branch: engine_branch,
                dirty: Cell::new(false),
                enabled: false,
            };
        }

        thread::spawn(move || {
            worker_loop(
                data_dir,
                remote,
                branch,
                debounce,
                pull_on_startup,
                req_rx,
                status_tx,
            );
        });

        Self {
            tx: req_tx,
            rx: status_rx,
            data_dir: engine_dir,
            remote: engine_remote,
            branch: engine_branch,
            dirty: Cell::new(false),
            enabled: true,
        }
    }

    #[inline(always)]
    pub fn request_push(&self) {
        if self.enabled {
            self.dirty.set(true);
            let _ = self.tx.send(SyncRequest::SchedulePush);
        }
    }

    #[inline(always)]
    pub fn force_sync(&self) {
        if self.enabled {
            let _ = self.tx.send(SyncRequest::ForceSync);
        }
    }

    #[inline(always)]
    pub fn try_recv(&self) -> Option<SyncStatus> {
        let status = self.rx.try_recv().ok();
        if status == Some(SyncStatus::Synced) {
            self.dirty.set(false);
        }
        status
    }

    /// Hands any unsynced changes to a detached copy of this binary (see
    /// [`flush`]) so the app can exit immediately while the push finishes in
    /// the background. Anything that still fails (offline) is picked up by the
    /// startup check in `worker_loop` next time.
    pub fn shutdown(&mut self) {
        if !self.enabled {
            return;
        }
        let _ = self.tx.send(SyncRequest::Shutdown);
        if !self.dirty.get() {
            return;
        }
        let Ok(exe) = std::env::current_exe() else {
            return;
        };
        let mut cmd = Command::new(exe);
        cmd.arg(FLUSH_ARG)
            .arg(&self.data_dir)
            .arg(&self.remote)
            .arg(&self.branch)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        #[cfg(unix)]
        cmd.process_group(0);
        #[cfg(windows)]
        {
            const DETACHED_PROCESS: u32 = 0x0000_0008;
            const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
            cmd.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
        }
        let _ = cmd.spawn();
    }
}

/// Hidden first argument that turns the binary into the detached quit-time pusher.
pub const FLUSH_ARG: &str = "--sync-flush";

/// Entry point of the detached pusher: one commit + push (pulling and
/// resolving conflicts like the in-app worker does), then exit.
pub fn flush(dir: &Path, remote: &str, branch: &str) {
    let (tx, _rx) = mpsc::channel::<SyncStatus>();
    do_push(dir, remote, branch, &tx);
}

fn do_pull(dir: &Path, remote: &str, branch: &str, tx: &Sender<SyncStatus>) {
    let _ = tx.send(SyncStatus::Syncing("Pulling...".into()));
    match git::pull_rebase(dir, remote, branch) {
        Ok(()) => {
            let _ = tx.send(SyncStatus::UpdatedRemoteData);
        }
        Err(git::GitError::Offline) => {
            let _ = tx.send(SyncStatus::Offline);
        }
        Err(git::GitError::Conflict) => {
            handle_conflict(dir, remote, branch, tx);
        }
        Err(e) => {
            let _ = tx.send(SyncStatus::Error(e.to_string()));
        }
    }
}

fn handle_conflict(dir: &Path, remote: &str, branch: &str, tx: &Sender<SyncStatus>) {
    match git::resolve_conflict_keep_remote(dir, remote, branch) {
        Ok(backup) => {
            let _ = tx.send(SyncStatus::Conflict(backup.display().to_string()));
        }
        Err(e) => {
            let _ = tx.send(SyncStatus::Error(e.to_string()));
        }
    }
}

/// Commits and pushes local changes, pulling first if the remote has moved on.
/// Returns `true` if the push should be retried later (currently offline).
fn do_push(dir: &Path, remote: &str, branch: &str, tx: &Sender<SyncStatus>) -> bool {
    let _ = tx.send(SyncStatus::Syncing("Committing...".into()));
    if let Err(e) = git::stage_and_commit(dir, "sync: update tasks") {
        let _ = tx.send(SyncStatus::Error(e.to_string()));
        return false;
    }

    let _ = tx.send(SyncStatus::Syncing("Pushing...".into()));
    match git::push(dir, remote, branch) {
        Ok(()) => {
            let _ = tx.send(SyncStatus::Synced);
            false
        }
        Err(git::GitError::Offline) => {
            let _ = tx.send(SyncStatus::Offline);
            true
        }
        Err(git::GitError::RejectedNonFastForward) => {
            let _ = tx.send(SyncStatus::Syncing("Pulling...".into()));
            match git::pull_rebase(dir, remote, branch) {
                Ok(()) => match git::push(dir, remote, branch) {
                    Ok(()) => {
                        let _ = tx.send(SyncStatus::Synced);
                        false
                    }
                    Err(git::GitError::Offline) => {
                        let _ = tx.send(SyncStatus::Offline);
                        true
                    }
                    Err(e) => {
                        let _ = tx.send(SyncStatus::Error(e.to_string()));
                        false
                    }
                },
                Err(git::GitError::Conflict) => {
                    handle_conflict(dir, remote, branch, tx);
                    false
                }
                Err(git::GitError::Offline) => {
                    let _ = tx.send(SyncStatus::Offline);
                    true
                }
                Err(e) => {
                    let _ = tx.send(SyncStatus::Error(e.to_string()));
                    false
                }
            }
        }
        Err(e) => {
            let _ = tx.send(SyncStatus::Error(e.to_string()));
            false
        }
    }
}

fn worker_loop(
    data_dir: PathBuf,
    remote: String,
    branch: String,
    debounce: Duration,
    pull_on_startup: bool,
    rx: Receiver<SyncRequest>,
    tx: Sender<SyncStatus>,
) {
    if pull_on_startup {
        do_pull(&data_dir, &remote, &branch, &tx);
    } else {
        let _ = tx.send(SyncStatus::Idle);
    }

    // Recover anything a previous run couldn't push (offline at quit, crash).
    let mut pending_since: Option<Instant> = git::has_unsynced_changes(&data_dir, &remote, &branch)
        .then(|| {
            let _ = tx.send(SyncStatus::Syncing("Pending sync...".into()));
            Instant::now()
        });

    loop {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(SyncRequest::SchedulePush) => {
                pending_since = Some(Instant::now());
                let _ = tx.send(SyncStatus::Syncing("Pending sync...".into()));
            }
            Ok(SyncRequest::ForceSync) => {
                let retry = do_push(&data_dir, &remote, &branch, &tx);
                pending_since = retry.then(Instant::now);
            }
            Ok(SyncRequest::Shutdown) => break,
            Err(RecvTimeoutError::Timeout) => {
                if let Some(started) = pending_since
                    && started.elapsed() >= debounce
                {
                    let retry = do_push(&data_dir, &remote, &branch, &tx);
                    pending_since = retry.then(Instant::now);
                }
            }
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        sync::atomic::{AtomicU32, Ordering},
    };

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    fn scratch_dir(name: &str) -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("hopes-sync-test-{name}-{n}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn git(dir: &Path, args: &[&str]) {
        let status = std::process::Command::new("git")
            .args(args)
            .current_dir(dir)
            .status()
            .unwrap();
        assert!(status.success(), "git {args:?} failed in {dir:?}");
    }

    fn commit_count(dir: &Path, branch: &str) -> u32 {
        let out = std::process::Command::new("git")
            .args(["rev-list", "--count", branch])
            .current_dir(dir)
            .output()
            .unwrap();
        String::from_utf8_lossy(&out.stdout).trim().parse().unwrap()
    }

    fn setup_repo() -> (PathBuf, PathBuf) {
        let remote = scratch_dir("remote");
        git(&remote, &["init", "--bare", "-b", "main"]);

        let workdir = scratch_dir("work");
        git(&workdir, &["clone", remote.to_str().unwrap(), "."]);
        git(&workdir, &["config", "user.email", "test@example.com"]);
        git(&workdir, &["config", "user.name", "Test"]);
        fs::write(workdir.join("items.json"), "[]").unwrap();
        git(&workdir, &["add", "items.json"]);
        git(&workdir, &["commit", "-m", "seed"]);
        git(&workdir, &["push", "origin", "main"]);

        (workdir, remote)
    }

    #[test]
    fn rapid_edits_collapse_into_a_single_commit() {
        let (workdir, remote) = setup_repo();
        let before = commit_count(&remote, "main");

        let engine = SyncEngine::spawn(
            workdir.clone(),
            "origin".into(),
            "main".into(),
            Duration::from_millis(300),
            false,
            true,
        );
        assert!(engine.enabled);

        for i in 0..5 {
            fs::write(
                workdir.join("items.json"),
                format!("[{{\"title\":\"edit-{i}\"}}]"),
            )
            .unwrap();
            engine.request_push();
            thread::sleep(Duration::from_millis(50));
        }

        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if commit_count(&remote, "main") > before {
                break;
            }
            assert!(Instant::now() < deadline, "push never landed in time");
            thread::sleep(Duration::from_millis(50));
        }

        assert_eq!(commit_count(&remote, "main"), before + 1);

        let verify = scratch_dir("verify");
        git(&verify, &["clone", remote.to_str().unwrap(), "."]);
        let contents = fs::read_to_string(verify.join("items.json")).unwrap();
        assert_eq!(contents, "[{\"title\":\"edit-4\"}]");
    }

    #[test]
    fn flush_pushes_uncommitted_edits() {
        let (workdir, remote) = setup_repo();
        let before = commit_count(&remote, "main");

        fs::write(workdir.join("items.json"), "[{\"title\":\"quit-time\"}]").unwrap();
        flush(&workdir, "origin", "main");

        assert_eq!(commit_count(&remote, "main"), before + 1);
    }

    #[test]
    fn engine_pushes_leftover_changes_on_startup() {
        let (workdir, remote) = setup_repo();
        let before = commit_count(&remote, "main");

        // Simulates a quit-time push that never made it out.
        fs::write(workdir.join("items.json"), "[{\"title\":\"left-behind\"}]").unwrap();
        let _engine = SyncEngine::spawn(
            workdir,
            "origin".into(),
            "main".into(),
            Duration::from_millis(100),
            false,
            true,
        );

        let deadline = Instant::now() + Duration::from_secs(5);
        while commit_count(&remote, "main") == before {
            assert!(Instant::now() < deadline, "leftover change never pushed");
            thread::sleep(Duration::from_millis(50));
        }
    }

    #[test]
    fn disabled_when_directory_is_not_a_git_repo() {
        let dir = scratch_dir("not-a-repo");
        let engine = SyncEngine::spawn(
            dir,
            "origin".into(),
            "main".into(),
            Duration::from_millis(100),
            false,
            true,
        );
        assert!(!engine.enabled);
        std::thread::sleep(Duration::from_millis(20));
        assert_eq!(engine.try_recv(), Some(SyncStatus::Disabled));
    }
}
