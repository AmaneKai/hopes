use std::{
    fmt, fs,
    path::{Path, PathBuf},
    process::{Child, Command, Output, Stdio},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const NETWORK_TIMEOUT: Duration = Duration::from_secs(10);

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

fn wait_for_exit(child: &mut Child) -> Result<(), GitError> {
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return Ok(()),
            Ok(None) if start.elapsed() > NETWORK_TIMEOUT => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(GitError::Offline);
            }
            Ok(None) => thread::sleep(Duration::from_millis(50)),
            Err(e) => return Err(GitError::Failed(e.to_string())),
        }
    }
}

fn run(dir: &Path, args: &[&str]) -> Result<Output, GitError> {
    let mut child = Command::new("git")
        .args(args)
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| GitError::Failed(e.to_string()))?;

    wait_for_exit(&mut child)?;
    child
        .wait_with_output()
        .map_err(|e| GitError::Failed(e.to_string()))
}

fn is_offline_stderr(stderr: &str) -> bool {
    let s = stderr.to_lowercase();
    s.contains("could not resolve host")
        || s.contains("unable to access")
        || s.contains("connection timed out")
        || s.contains("network is unreachable")
        || s.contains("could not read from remote repository")
        || s.contains("no route to host")
}

pub fn has_unsynced_changes(dir: &Path, remote: &str, branch: &str) -> bool {
    let dirty = run(dir, &["status", "--porcelain", "--", "items.json"])
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false);
    if dirty {
        return true;
    }
    run(
        dir,
        &["rev-list", "--count", &format!("{remote}/{branch}..HEAD")],
    )
    .ok()
    .filter(|o| o.status.success())
    .and_then(|o| {
        String::from_utf8_lossy(&o.stdout)
            .trim()
            .parse::<u32>()
            .ok()
    })
    .is_some_and(|n| n > 0)
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
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let backup_path = dir.join(format!("items.json.local-backup-{timestamp}"));
    let _ = fs::copy(&items_path, &backup_path);

    run(dir, &["fetch", remote, branch])?;
    run(dir, &["reset", "--hard", &format!("{remote}/{branch}")])?;
    Ok(backup_path)
}
