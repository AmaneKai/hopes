use hopes::systems::sync::{SyncEngine, SyncStatus, flush};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU32, Ordering},
    thread,
    time::{Duration, Instant},
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
