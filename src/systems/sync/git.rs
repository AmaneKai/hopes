use crate::utils::proc;
use std::{
    fmt, fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const NETWORK_TIMEOUT: Duration = Duration::from_secs(10);
const OUTPUT_LIMIT: usize = 1 << 20;

#[derive(Debug)]
pub enum GitError {
    Offline,
    Conflict,
    RejectedNonFastForward,
    Failed(String),
}

impl fmt::Display for GitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Offline => write!(f, "network unreachable"),
            Self::Conflict => write!(f, "merge conflict"),
            Self::RejectedNonFastForward => write!(f, "remote has newer commits"),
            Self::Failed(msg) => write!(f, "{msg}"),
        }
    }
}

#[inline(always)]
pub fn is_repo(dir: &Path) -> bool {
    dir.join(".git").exists()
}

fn run(dir: &Path, args: &[&str]) -> Result<Output, GitError> {
    let run = proc::run(
        Command::new("git").args(args).current_dir(dir),
        None,
        OUTPUT_LIMIT,
        Some(NETWORK_TIMEOUT),
    )
    .map_err(|e| GitError::Failed(e.to_string()))?;
    match run.status {
        Some(status) => Ok(Output {
            status,
            stdout: run.stdout,
            stderr: run.stderr,
        }),
        None => Err(GitError::Offline),
    }
}

fn is_offline_stderr(stderr: &str) -> bool {
    const OFFLINE_MESSAGES: [&str; 6] = [
        "could not resolve host",
        "unable to access",
        "connection timed out",
        "network is unreachable",
        "could not read from remote repository",
        "no route to host",
    ];
    let stderr = stderr.to_lowercase();
    for message in OFFLINE_MESSAGES {
        if stderr.contains(message) {
            return true;
        }
    }
    false
}

pub fn has_unsynced_changes(dir: &Path, remote: &str, branch: &str) -> bool {
    if let Ok(status) = run(dir, &["status", "--porcelain", "--", "items.json"])
        && !status.stdout.is_empty()
    {
        return true;
    }

    let range = format!("{remote}/{branch}..HEAD");
    let Ok(output) = run(dir, &["rev-list", "--count", &range]) else {
        return false;
    };
    if !output.status.success() {
        return false;
    }
    let count = String::from_utf8_lossy(&output.stdout);
    match count.trim().parse::<u32>() {
        Ok(ahead) => ahead > 0,
        Err(_) => false,
    }
}

pub fn pull_rebase(dir: &Path, remote: &str, branch: &str) -> Result<(), GitError> {
    let output = run(dir, &["pull", "--rebase", "--autostash", remote, branch])?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    if is_offline_stderr(&stderr) {
        return Err(GitError::Offline);
    }
    if stderr.contains("CONFLICT") || stderr.contains("could not apply") {
        return Err(GitError::Conflict);
    }
    Err(GitError::Failed(stderr.trim().to_string()))
}

pub fn stage_and_commit(dir: &Path, message: &str) -> Result<(), GitError> {
    run(dir, &["add", "items.json"])?;
    let output = run(dir, &["commit", "-m", message])?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.contains("nothing to commit") || stderr.contains("nothing to commit") {
        return Ok(());
    }
    Err(GitError::Failed(stderr.trim().to_string()))
}

pub fn push(dir: &Path, remote: &str, branch: &str) -> Result<(), GitError> {
    let output = run(dir, &["push", remote, branch])?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    if is_offline_stderr(&stderr) {
        return Err(GitError::Offline);
    }
    if stderr.contains("[rejected]")
        || stderr.contains("non-fast-forward")
        || stderr.contains("fetch first")
    {
        return Err(GitError::RejectedNonFastForward);
    }
    Err(GitError::Failed(stderr.trim().to_string()))
}

pub fn resolve_conflict_keep_remote(
    dir: &Path,
    remote: &str,
    branch: &str,
) -> Result<PathBuf, GitError> {
    let _ = run(dir, &["rebase", "--abort"]);

    let items_path = dir.join("items.json");
    let timestamp = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(elapsed) => elapsed.as_secs(),
        Err(_) => 0,
    };
    let backup_path = dir.join(format!("items.json.local-backup-{timestamp}"));
    let _ = fs::copy(&items_path, &backup_path);

    run(dir, &["fetch", remote, branch])?;
    run(dir, &["reset", "--hard", &format!("{remote}/{branch}")])?;
    Ok(backup_path)
}
