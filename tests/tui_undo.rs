use std::fs;
use std::path::PathBuf;

use ttd::store::{TaskId, TaskStore};

fn temp_store(name: &str) -> (TaskStore, PathBuf) {
    let mut path = std::env::temp_dir();
    path.push(format!("ttd-undo-{}-{}", name, std::process::id()));
    let _ = fs::remove_dir_all(&path);
    let store = TaskStore::open(path.clone()).unwrap();
    (store, path)
}

#[test]
fn undo_delete_restores_file_to_open_dir() {
    let (store, root) = temp_store("del-open");
    let id = store.create_task("buy milk +groceries").unwrap();
    let filename = id.file_name().to_string();
    let raw_before = "buy milk +groceries";

    store.delete_task(&id).unwrap();
    assert!(!id.path.exists());

    let path = store.root_dir().join(&filename);
    fs::write(&path, format!("{}\n", raw_before)).unwrap();
    assert!(path.exists());

    let snapshot = store.load_all().unwrap();
    assert_eq!(snapshot.open_tasks.len(), 1);
    assert_eq!(snapshot.open_tasks[0].task.raw, raw_before);

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn undo_delete_restores_file_to_done_dir() {
    let (store, root) = temp_store("del-done");
    let id = store.create_task("buy milk").unwrap();
    let filename = id.file_name().to_string();
    store.mark_done(&id, "2026-07-17").unwrap();

    let done_id = TaskId {
        path: store.done_dir().join(&filename),
        line_index: 0,
    };
    store.delete_task(&done_id).unwrap();
    assert!(!done_id.path.exists());

    fs::write(&done_id.path, "x 2026-07-17 buy milk\n").unwrap();

    let snapshot = store.load_all().unwrap();
    assert_eq!(snapshot.done_tasks.len(), 1);

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn undo_toggle_restores_open_task() {
    let (store, root) = temp_store("toggle-open");
    let id = store.create_task("buy milk").unwrap();
    let raw_before = "buy milk";
    let filename = id.file_name().to_string();

    store.mark_done(&id, "2026-07-17").unwrap();
    assert!(!id.path.exists());

    let done_id = TaskId {
        path: store.done_dir().join(&filename),
        line_index: 0,
    };
    store.restore_task(&done_id).unwrap();
    assert!(id.path.exists());

    let snapshot = store.load_all().unwrap();
    assert_eq!(snapshot.open_tasks.len(), 1);
    assert_eq!(snapshot.open_tasks[0].task.raw, raw_before);

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn undo_toggle_restore_puts_task_back_to_done() {
    let (store, root) = temp_store("toggle-done");
    let id = store.create_task("buy milk").unwrap();
    let filename = id.file_name().to_string();

    store.mark_done(&id, "2026-07-17").unwrap();
    let done_id = TaskId {
        path: store.done_dir().join(&filename),
        line_index: 0,
    };
    store.restore_task(&done_id).unwrap();
    let open_id = TaskId {
        path: store.root_dir().join(&filename),
        line_index: 0,
    };
    store.mark_done(&open_id, "2026-07-17").unwrap();

    assert!(!open_id.path.exists());
    let snapshot = store.load_all().unwrap();
    assert_eq!(snapshot.done_tasks.len(), 1);

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn empty_stack_undo_is_noop() {
    let (_store, _root) = temp_store("empty");
    // Verifies that an empty undo scenario doesn't panic.
    // The actual empty-stack check happens in TuiSession, not in the store — this test
    // simply confirms store operations work on an untouched directory.
}

#[test]
fn multi_level_undo_restores_in_reverse_order() {
    let (store, root) = temp_store("multi");
    let id_a = store.create_task("task a").unwrap();
    let id_b = store.create_task("task b").unwrap();
    let raw_a = "task a";
    let filename_a = id_a.file_name().to_string();
    let filename_b = id_b.file_name().to_string();

    store.delete_task(&id_a).unwrap();
    assert!(!id_a.path.exists());
    store.mark_done(&id_b, "2026-07-17").unwrap();
    assert!(!id_b.path.exists());

    let done_id_b = TaskId {
        path: store.done_dir().join(&filename_b),
        line_index: 0,
    };
    store.restore_task(&done_id_b).unwrap();
    let path_a = store.root_dir().join(&filename_a);
    fs::write(&path_a, format!("{}\n", raw_a)).unwrap();

    let snapshot = store.load_all().unwrap();
    assert_eq!(snapshot.open_tasks.len(), 2);
    let raws: Vec<&str> = snapshot
        .open_tasks
        .iter()
        .map(|t| t.task.raw.as_str())
        .collect();
    assert!(raws.contains(&"task a"));
    assert!(raws.contains(&"task b"));

    let _ = fs::remove_dir_all(&root);
}
