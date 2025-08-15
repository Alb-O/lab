use std::fs;
use std::thread;
use std::time::Duration;

use dot001_watcher::{WatchEvent, WatchOptions, watch};
use tempfile::TempDir;

fn touch(path: &std::path::Path) {
    fs::File::create(path).unwrap();
}

#[test]
fn pairs_delete_create_as_move_by_basename() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().to_path_buf();

    let a_dir = root.join("a");
    let b_dir = root.join("b");
    fs::create_dir_all(&a_dir).unwrap();
    fs::create_dir_all(&b_dir).unwrap();

    let a_file = a_dir.join("scene.blend");
    touch(&a_file);

    let opts = WatchOptions {
        root: root.clone(),
        debounce_ms: 100,
        move_pair_window_ms: 2000,
        follow_symlinks: false,
    };

    let (rx, _watcher) = watch(opts).expect("watcher");

    // Give watcher time to start
    thread::sleep(Duration::from_millis(200));

    // Simulate move by delete+create: remove from a, create in b
    fs::remove_file(&a_file).unwrap();
    let b_file = b_dir.join("scene.blend");
    // Small delay within pairing window
    thread::sleep(Duration::from_millis(150));
    touch(&b_file);

    // Wait for events to be paired
    let mut got_move = false;
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while std::time::Instant::now() < deadline {
        if let Ok(WatchEvent::BlendFileMoved(pair)) = rx.recv_timeout(Duration::from_millis(250)) {
            if pair.from.ends_with(&a_file) && pair.to.ends_with(&b_file) {
                got_move = true;
                break;
            }
        }
    }

    assert!(
        got_move,
        "expected BlendFileMoved pairing for delete+create"
    );
}

#[test]
fn rename_changes_basename_emits_rename() {
    env_logger::try_init().ok(); // Initialize logging for debugging
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().to_path_buf();

    fs::create_dir_all(&root).unwrap();
    let a = root.join("old.blend");
    touch(&a);

    let opts = WatchOptions {
        root: root.clone(),
        debounce_ms: 100,
        move_pair_window_ms: 2000,
        follow_symlinks: false,
    };

    let (rx, _watcher) = watch(opts).expect("watcher");

    thread::sleep(Duration::from_millis(200));

    let b = root.join("new.blend");
    fs::rename(&a, &b).unwrap();

    let mut got = false;
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while std::time::Instant::now() < deadline {
        if let Ok(WatchEvent::BlendFileRenamed { from, to, .. }) =
            rx.recv_timeout(Duration::from_millis(250))
        {
            if from.ends_with("old.blend") && to.ends_with("new.blend") {
                got = true;
                break;
            }
        }
    }

    assert!(got, "expected BlendFileRenamed on rename");
}

#[test]
fn dir_move_emits_dir_and_child_moves() {
    env_logger::try_init().ok(); // Initialize logging for debugging
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().to_path_buf();

    let d1 = root.join("d1");
    let d2 = root.join("d2");
    fs::create_dir_all(&d1).unwrap();

    let c1 = d1.join("child1.blend");
    let c2subdir = d1.join("sub");
    fs::create_dir_all(&c2subdir).unwrap();
    let c2 = c2subdir.join("child2.blend");
    touch(&c1);
    touch(&c2);

    let opts = WatchOptions {
        root: root.clone(),
        debounce_ms: 100,
        move_pair_window_ms: 2000,
        follow_symlinks: false,
    };

    let (rx, _watcher) = watch(opts).expect("watcher");

    thread::sleep(Duration::from_millis(200));

    fs::rename(&d1, &d2).unwrap();

    let mut saw_dir = false;
    let mut saw_child1 = false;
    let mut saw_child2 = false;

    let child1_to = d2.join("child1.blend");
    let child2_to = d2.join("sub").join("child2.blend");

    let deadline = std::time::Instant::now() + Duration::from_secs(6);
    while std::time::Instant::now() < deadline {
        if let Ok(ev) = rx.recv_timeout(Duration::from_millis(500)) {
            match ev {
                WatchEvent::DirRenamedOrMoved(pair) => {
                    if pair.from.ends_with("d1") && pair.to.ends_with("d2") {
                        saw_dir = true;
                    }
                }
                WatchEvent::DirBlendChildMoved(pair) => {
                    if pair.to.ends_with(&child1_to) {
                        saw_child1 = true;
                    }
                    if pair.to.ends_with(&child2_to) {
                        saw_child2 = true;
                    }
                }
                _ => {}
            }
        }
        if saw_dir && saw_child1 && saw_child2 {
            break;
        }
    }

    assert!(saw_dir, "expected DirRenamedOrMoved");
    assert!(saw_child1, "expected DirBlendChildMoved for child1");
    assert!(saw_child2, "expected DirBlendChildMoved for child2");
}
