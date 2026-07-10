use std::sync::{Arc, Barrier};

use meld_events::error::StorageError;
use meld_events::EventCursor;

const CURSOR_TREE: &str = "consumer_meta";

#[test]
fn independent_cursor_handles_preserve_the_global_maximum() {
    let dir = tempfile::TempDir::new().unwrap();
    let db = sled::open(dir.path()).unwrap();
    let contenders = 64_u64;

    for round in 0..16 {
        let name = format!("graph-{round}");
        let barrier = Arc::new(Barrier::new(contenders as usize));
        let mut handles = Vec::with_capacity(contenders as usize);

        for seq in 1..=contenders {
            let cursor = EventCursor::new(db.open_tree(CURSOR_TREE).unwrap(), &name);
            let barrier = Arc::clone(&barrier);
            handles.push(std::thread::spawn(move || {
                barrier.wait();
                cursor.advance(seq).unwrap()
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let cursor = EventCursor::new(db.open_tree(CURSOR_TREE).unwrap(), &name);
        assert_eq!(cursor.get().unwrap(), contenders);
    }
}

#[test]
fn concurrent_maximum_survives_drop_and_reopen() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().to_path_buf();
    let db = sled::open(&path).unwrap();
    let contenders = 64_u64;
    let barrier = Arc::new(Barrier::new(contenders as usize));
    let mut handles = Vec::with_capacity(contenders as usize);

    for seq in 1..=contenders {
        let cursor = EventCursor::new(db.open_tree(CURSOR_TREE).unwrap(), "graph");
        let barrier = Arc::clone(&barrier);
        handles.push(std::thread::spawn(move || {
            barrier.wait();
            cursor.advance(seq).unwrap()
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }
    assert_eq!(
        EventCursor::new(db.open_tree(CURSOR_TREE).unwrap(), "graph")
            .get()
            .unwrap(),
        contenders
    );

    drop(db);

    let reopened = sled::open(&path).unwrap();
    let cursor = EventCursor::new(reopened.open_tree(CURSOR_TREE).unwrap(), "graph");
    assert_eq!(cursor.get().unwrap(), contenders);
}

#[test]
fn malformed_cursor_payload_remains_an_error() {
    let dir = tempfile::TempDir::new().unwrap();
    let db = sled::open(dir.path()).unwrap();
    let tree = db.open_tree(CURSOR_TREE).unwrap();
    tree.insert(b"event_cursor::graph", b"invalid".as_slice())
        .unwrap();
    let cursor = EventCursor::new(tree.clone(), "graph");

    assert_invalid_data(cursor.get().unwrap_err());
    assert_invalid_data(cursor.advance(42).unwrap_err());
    assert_eq!(
        tree.get(b"event_cursor::graph").unwrap().unwrap().as_ref(),
        b"invalid"
    );
}

fn assert_invalid_data(error: StorageError) {
    match error {
        StorageError::IoError(error) => {
            assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
            assert_eq!(error.to_string(), "invalid ledger cursor payload");
        }
        other => panic!("expected invalid cursor I/O error, got {other}"),
    }
}
