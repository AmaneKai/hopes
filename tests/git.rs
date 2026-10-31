use hopes::systems::sync::git::*;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    sync::atomic::{AtomicU32, Ordering},
};

fn run(dir: &Path, args: &[&str]) -> Result<Output, String> {
    Command::new("git")
        .args(args)
        .current_dir(dir)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| e.to_string())
}

static COUNTER: AtomicU32 = AtomicU32::new(0);

fn scratch_dir(name: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir =
        std::env::temp_dir().join(format!("hopes-git-test-{name}-{n}-{}", std::process::id()));
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
fn detects_uncommitted_and_unpushed_changes() {
    let remote = make_bare_remote_with_items("[]");
    let workdir = scratch_dir("unsynced");
    init_clone(&workdir, &remote);
    assert!(!has_unsynced_changes(&workdir, "origin", "main"));

    fs::write(workdir.join("items.json"), "[{\"title\":\"a\"}]").unwrap();
    assert!(has_unsynced_changes(&workdir, "origin", "main"));

    stage_and_commit(&workdir, "sync: update tasks").unwrap();
    assert!(has_unsynced_changes(&workdir, "origin", "main"));

    push(&workdir, "origin", "main").unwrap();
    assert!(!has_unsynced_changes(&workdir, "origin", "main"));
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

    stage_and_commit(&workdir, "sync: update tasks").unwrap();
}

#[test]
fn pull_rebase_fetches_clean_remote_changes() {
    let remote = make_bare_remote_with_items("[]");
    let machine_a = scratch_dir("machine-a");
    init_clone(&machine_a, &remote);

    let machine_b = scratch_dir("machine-b");
    init_clone(&machine_b, &remote);
    fs::write(machine_b.join("items.json"), "[{\"title\":\"from-b\"}]").unwrap();
    stage_and_commit(&machine_b, "sync: update tasks").unwrap();
    push(&machine_b, "origin", "main").unwrap();

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
