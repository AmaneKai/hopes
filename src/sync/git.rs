use std::{
    fmt, fs,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
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

fn run(dir: &Path, args: &[&str]) -> Result<Output, GitError> {
    let mut child = Command::new("git")
        .args(args)
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| GitError::Failed(e.to_string()))?;

    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {
                if start.elapsed() > NETWORK_TIMEOUT {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(GitError::Offline);
                }
                thread::sleep(Duration::from_millis(50));
            }
            Err(e) => return Err(GitError::Failed(e.to_string())),
        }
    }
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

/// Aborts an in-progress rebase, backs up the local `items.json`, then hard-resets
/// to the remote branch so the app always has a consistent (if not-yet-merged) state.
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    fn scratch_dir(name: &str) -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("hopes-git-test-{name}-{n}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn init_clone(dir: &Path, remote_path: &Path) {
        fs::create_dir_all(dir).unwrap();
        run(dir, &["clone", remote_path.to_str().unwrap(), "."]).unwrap();
        run(dir, &["config", "user.email", "test@example.com"]).unwrap();
        run(dir, &["config", "user.name", "Test"]).unwrap();
    }

    fn make_bare_remote_with_items(items_json: &str) -> PathBuf {
        let remote = scratch_dir("remote");
        run(&remote, &["init", "--bare", "-b", "main"]).unwrap();

        let seed = scratch_dir("seed");
        init_clone(&seed, &remote);
        fs::write(seed.join("items.json"), items_json).unwrap();
        run(&seed, &["add", "items.json"]).unwrap();
        run(&seed, &["commit", "-m", "seed"]).unwrap();
        run(&seed, &["push", "origin", "main"]).unwrap();
        remote
    }

    #[test]
    fn is_repo_detects_dot_git() {
        let dir = scratch_dir("isrepo");
        assert!(!is_repo(&dir));
        run(&dir, &["init", "-b", "main"]).unwrap();
        assert!(is_repo(&dir));
    }

    #[test]
    fn stage_commit_and_push_round_trips() {
        let remote = make_bare_remote_with_items("[]");
        let workdir = scratch_dir("work");
        init_clone(&workdir, &remote);

        fs::write(workdir.join("items.json"), "[{\"title\":\"a\"}]").unwrap();
        stage_and_commit(&workdir, "sync: update tasks").unwrap();
        push(&workdir, "origin", "main").unwrap();

        let verify = scratch_dir("verify");
        init_clone(&verify, &remote);
        let contents = fs::read_to_string(verify.join("items.json")).unwrap();
        assert_eq!(contents, "[{\"title\":\"a\"}]");
    }

    #[test]
    fn stage_and_commit_is_noop_when_nothing_changed() {
        let remote = make_bare_remote_with_items("[]");
        let workdir = scratch_dir("work-noop");
        init_clone(&workdir, &remote);

        // No file changes made — should succeed without creating a commit.
        stage_and_commit(&workdir, "sync: update tasks").unwrap();
    }

    #[test]
    fn pull_rebase_fetches_clean_remote_changes() {
        let remote = make_bare_remote_with_items("[]");
        let machine_a = scratch_dir("machine-a");
        init_clone(&machine_a, &remote);

        // Machine B pushes a change.
        let machine_b = scratch_dir("machine-b");
        init_clone(&machine_b, &remote);
        fs::write(machine_b.join("items.json"), "[{\"title\":\"from-b\"}]").unwrap();
        stage_and_commit(&machine_b, "sync: update tasks").unwrap();
        push(&machine_b, "origin", "main").unwrap();

        // Machine A pulls and should see it.
        pull_rebase(&machine_a, "origin", "main").unwrap();
        let contents = fs::read_to_string(machine_a.join("items.json")).unwrap();
        assert_eq!(contents, "[{\"title\":\"from-b\"}]");
    }

    #[test]
    fn push_reports_rejected_non_fast_forward_when_remote_moved_on() {
        let remote = make_bare_remote_with_items("[]");
        let machine_a = scratch_dir("nff-a");
        init_clone(&machine_a, &remote);
        let machine_b = scratch_dir("nff-b");
        init_clone(&machine_b, &remote);

        fs::write(machine_b.join("items.json"), "[{\"title\":\"from-b\"}]").unwrap();
        stage_and_commit(&machine_b, "sync: update tasks").unwrap();
        push(&machine_b, "origin", "main").unwrap();

        fs::write(machine_a.join("items.json"), "[{\"title\":\"from-a\"}]").unwrap();
        stage_and_commit(&machine_a, "sync: update tasks").unwrap();
        let result = push(&machine_a, "origin", "main");
        assert!(matches!(result, Err(GitError::RejectedNonFastForward)));
    }

    #[test]
    fn resolve_conflict_keep_remote_backs_up_and_matches_remote() {
        let remote = make_bare_remote_with_items("[]");
        let machine_a = scratch_dir("conflict-a");
        init_clone(&machine_a, &remote);
        let machine_b = scratch_dir("conflict-b");
        init_clone(&machine_b, &remote);

        // Both machines edit the same line of items.json differently, then B pushes first.
        fs::write(machine_b.join("items.json"), "[{\"title\":\"from-b\"}]").unwrap();
        stage_and_commit(&machine_b, "sync: update tasks").unwrap();
        push(&machine_b, "origin", "main").unwrap();

        fs::write(machine_a.join("items.json"), "[{\"title\":\"from-a\"}]").unwrap();
        stage_and_commit(&machine_a, "sync: update tasks").unwrap();

        let result = pull_rebase(&machine_a, "origin", "main");
        assert!(matches!(result, Err(GitError::Conflict)));

        let backup = resolve_conflict_keep_remote(&machine_a, "origin", "main").unwrap();
        assert!(backup.exists());
        let backup_contents = fs::read_to_string(&backup).unwrap();
        assert_eq!(backup_contents, "[{\"title\":\"from-a\"}]");

        let items_contents = fs::read_to_string(machine_a.join("items.json")).unwrap();
        assert_eq!(items_contents, "[{\"title\":\"from-b\"}]");
    }
}
