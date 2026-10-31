#![cfg(unix)]

use hopes::{
    domain::Item,
    systems::storage::{ExecStore, Storage},
};

fn script_store(script: &str) -> ExecStore {
    ExecStore::new(
        std::env::temp_dir(),
        vec!["sh".into(), "-c".into(), script.into(), "hopes".into()],
    )
    .unwrap()
}

#[test]
fn round_trips_through_an_external_program() {
    let file = std::env::temp_dir().join(format!("hopes-exec-{}.json", std::process::id()));
    let _ = std::fs::remove_file(&file);
    let path = file.display();
    let store = script_store(&format!(
        "case $1 in load) cat {path} 2>/dev/null || true;; save) cat > {path};; esac"
    ));

    assert!(store.load().unwrap().is_empty());
    let items = vec![Item::new("a".into(), vec!["x".into()])];
    store.save(&items).unwrap();
    assert_eq!(store.load().unwrap(), items);
    let _ = std::fs::remove_file(&file);
}

#[test]
fn nonzero_exit_is_an_error() {
    let store = script_store("echo boom >&2; exit 3");
    let err = store.load().unwrap_err().to_string();
    assert!(err.contains("boom"), "{err}");
    assert!(store.save(&[]).is_err());
}

#[test]
fn missing_command_is_rejected() {
    assert!(ExecStore::new(std::env::temp_dir(), vec![]).is_err());
}
