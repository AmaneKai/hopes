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

pub const FLUSH_ARG: &str = "--sync-flush";

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

fn report(tx: &Sender<SyncStatus>, status: SyncStatus) {
    let _ = tx.send(status);
}

fn push_outcome(result: Result<(), git::GitError>, tx: &Sender<SyncStatus>) -> bool {
    match result {
        Ok(()) => {
            report(tx, SyncStatus::Synced);
            false
        }
        Err(git::GitError::Offline) => {
            report(tx, SyncStatus::Offline);
            true
        }
        Err(e) => {
            report(tx, SyncStatus::Error(e.to_string()));
            false
        }
    }
}

fn rebase_and_push(dir: &Path, remote: &str, branch: &str, tx: &Sender<SyncStatus>) -> bool {
    report(tx, SyncStatus::Syncing("Pulling...".into()));
    match git::pull_rebase(dir, remote, branch) {
        Ok(()) => push_outcome(git::push(dir, remote, branch), tx),
        Err(git::GitError::Conflict) => {
            handle_conflict(dir, remote, branch, tx);
            false
        }
        Err(e) => push_outcome(Err(e), tx),
    }
}

fn do_push(dir: &Path, remote: &str, branch: &str, tx: &Sender<SyncStatus>) -> bool {
    report(tx, SyncStatus::Syncing("Committing...".into()));
    if let Err(e) = git::stage_and_commit(dir, "sync: update tasks") {
        report(tx, SyncStatus::Error(e.to_string()));
        return false;
    }

    report(tx, SyncStatus::Syncing("Pushing...".into()));
    match git::push(dir, remote, branch) {
        Err(git::GitError::RejectedNonFastForward) => rebase_and_push(dir, remote, branch, tx),
        other => push_outcome(other, tx),
    }
}

fn retry_deadline(retry: bool) -> Option<Instant> {
    retry.then(Instant::now)
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

    let mut pending_since: Option<Instant> = git::has_unsynced_changes(&data_dir, &remote, &branch)
        .then(|| {
            let _ = tx.send(SyncStatus::Syncing("Pending sync...".into()));
            Instant::now()
        });

    loop {
        let due = pending_since.is_some_and(|started| started.elapsed() >= debounce);
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(SyncRequest::Shutdown) | Err(RecvTimeoutError::Disconnected) => break,
            Ok(SyncRequest::SchedulePush) => {
                pending_since = Some(Instant::now());
                report(&tx, SyncStatus::Syncing("Pending sync...".into()));
            }
            Ok(SyncRequest::ForceSync) => {
                pending_since = retry_deadline(do_push(&data_dir, &remote, &branch, &tx));
            }
            Err(RecvTimeoutError::Timeout) if due => {
                pending_since = retry_deadline(do_push(&data_dir, &remote, &branch, &tx));
            }
            Err(RecvTimeoutError::Timeout) => {}
        }
    }
}
