#![cfg(feature = "sqlite")]

use hopes::{
    domain::Item,
    systems::storage::{SqliteStore, Storage},
};

#[test]
fn save_load_round_trip_keeps_order_and_drops_removed() {
    let dir = std::env::temp_dir().join(format!("hopes-sqlite-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let store = SqliteStore::at_dir(dir.clone()).unwrap();
    let (a, b, c) = (
        Item::new("a".into(), vec![]),
        Item::new("b".into(), vec![]),
        Item::new("c".into(), vec![]),
    );
    store.save(&[a.clone(), b, c.clone()]).unwrap();
    store.save(&[c.clone(), a.clone()]).unwrap();
    assert_eq!(store.load().unwrap(), vec![c, a]);
    let _ = std::fs::remove_dir_all(&dir);
}
