// SPDX-License-Identifier: AGPL-3.0-only
//! Hardening tests of the local file-system adapter (MIG-030H): one writer per destination (also across processes
//! and after a crash), read-only handles, leftover cleanup under the lock, and (Windows) verified folders that
//! cannot be swapped for links while in use. Synthetic temporary folders only.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use armasync_adapters::localfs::LocalDirectory;
use armasync_core::{CancelToken, ErrorKind, HashAlgorithm, LocalStore, RelativePath};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "armasync-hardening-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn path(text: &str) -> RelativePath {
    RelativePath::parse(text).unwrap()
}

#[test]
fn a_second_exclusive_handle_fails_clearly_until_the_first_is_dropped() {
    let root = scratch("lock");
    let first = LocalDirectory::open_exclusive(&root).unwrap();
    let error = LocalDirectory::open_exclusive(&root).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Config);
    assert!(error.message().contains("another ArmaSync run"), "{error}");
    // a clone shares the lock: it is still held while any clone lives
    let clone = first.clone();
    drop(first);
    assert_eq!(
        LocalDirectory::open_exclusive(&root).unwrap_err().kind(),
        ErrorKind::Config
    );
    drop(clone);
    LocalDirectory::open_exclusive(&root).unwrap();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn read_only_handles_scan_but_cannot_change_anything() {
    let root = scratch("readonly");
    fs::write(root.join("a.pbo"), b"abc").unwrap();
    let held = LocalDirectory::open_exclusive(&root).unwrap();
    // scanning is allowed even while a writer is active
    let reader = LocalDirectory::new(&root).unwrap();
    let listed = reader
        .list(HashAlgorithm::Sha1, &CancelToken::new())
        .unwrap();
    assert_eq!(listed.len(), 1, "the lock file is not listed");
    let mut reader = reader;
    assert_eq!(
        reader
            .begin(&path("b.pbo"), 1)
            .map(|_| ())
            .unwrap_err()
            .kind(),
        ErrorKind::Config
    );
    assert_eq!(
        reader.write(&path("b.pbo"), b"x").unwrap_err().kind(),
        ErrorKind::Config
    );
    assert_eq!(
        reader.delete(&path("a.pbo")).unwrap_err().kind(),
        ErrorKind::Config
    );
    assert_eq!(fs::read(root.join("a.pbo")).unwrap(), b"abc");
    drop(held);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn the_lock_file_is_reserved_and_never_listed_or_replaceable() {
    let root = scratch("lockname");
    let mut store = LocalDirectory::open_exclusive(&root).unwrap();
    assert!(
        store
            .list(HashAlgorithm::Sha1, &CancelToken::new())
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        store
            .begin(&path(".armasync-lock"), 1)
            .map(|_| ())
            .unwrap_err()
            .kind(),
        ErrorKind::FileSystem
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn leftovers_are_removed_at_once_when_the_exclusive_lock_is_held() {
    let root = scratch("leftovers");
    fs::create_dir_all(root.join("sub")).unwrap();
    fs::write(root.join(".armasync-1-1-1.tmp"), b"crash leftover").unwrap();
    fs::write(root.join("sub/.armasync-2-2-2.tmp"), b"crash leftover").unwrap();
    fs::write(root.join("keep.pbo"), b"keep").unwrap();
    // a read-only scan leaves fresh files alone (it cannot know whether another run owns them)
    let reader = LocalDirectory::new(&root).unwrap();
    reader
        .list(HashAlgorithm::Sha1, &CancelToken::new())
        .unwrap();
    assert!(root.join(".armasync-1-1-1.tmp").exists());
    // the exclusive owner knows nobody else can own them
    let writer = LocalDirectory::open_exclusive(&root).unwrap();
    let listed = writer
        .list(HashAlgorithm::Sha1, &CancelToken::new())
        .unwrap();
    assert_eq!(listed.len(), 1);
    assert!(!root.join(".armasync-1-1-1.tmp").exists());
    assert!(!root.join("sub/.armasync-2-2-2.tmp").exists());
    fs::remove_dir_all(root).unwrap();
}

/// Helper "program": holds the lock of the folder named in the environment until its input is closed. It only does
/// anything when started by `the_lock_is_held_across_processes_and_released_when_the_holder_dies`.
#[test]
fn lock_holder_child() {
    let Ok(dir) = std::env::var("ARMASYNC_TEST_LOCK_DIR") else {
        return;
    };
    let _held = LocalDirectory::open_exclusive(Path::new(&dir)).unwrap();
    println!("LOCKED");
    std::io::stdout().flush().unwrap();
    let mut line = String::new();
    let _ = std::io::stdin().read_line(&mut line);
}

#[test]
fn the_lock_is_held_across_processes_and_released_when_the_holder_dies() {
    let root = scratch("process-lock");
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "lock_holder_child",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("ARMASYNC_TEST_LOCK_DIR", &root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
    loop {
        let line = lines.next().expect("the child must report").unwrap();
        if line.contains("LOCKED") {
            break;
        }
    }
    // another process holds the destination
    let error = LocalDirectory::open_exclusive(&root).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Config, "{error}");
    // the holder is killed (a crash): the operating system releases the lock, nothing stale remains
    child.kill().unwrap();
    child.wait().unwrap();
    LocalDirectory::open_exclusive(&root).unwrap();
    fs::remove_dir_all(root).unwrap();
}

#[cfg(windows)]
mod windows_swap {
    use super::*;
    use std::io::Write;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::thread;
    use std::time::Duration;

    #[test]
    fn a_verified_folder_cannot_be_renamed_while_a_staged_file_exists() {
        let root = scratch("pin");
        fs::create_dir_all(root.join("@m/addons")).unwrap();
        let mut store = LocalDirectory::open_exclusive(&root).unwrap();
        {
            let mut staged = store.begin(&path("@m/addons/a.pbo"), 3).unwrap();
            staged.write_all(b"abc").unwrap();
            // a competing process tries to move the verified folders away (to put a link in their place)
            assert!(fs::rename(root.join("@m/addons"), root.join("@m/addons_old")).is_err());
            assert!(fs::rename(root.join("@m"), root.join("@m_old")).is_err());
            assert!(fs::remove_dir(root.join("@m/addons")).is_err());
            staged.commit().unwrap();
        }
        // once the file is committed the folders are free again
        fs::rename(root.join("@m/addons"), root.join("@m/addons_old")).unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn dropping_a_staged_file_releases_the_folders_too() {
        let root = scratch("pin-drop");
        let mut store = LocalDirectory::open_exclusive(&root).unwrap();
        drop(store.begin(&path("@m/a.pbo"), 1).unwrap());
        fs::rename(root.join("@m"), root.join("@m_old")).unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    /// An attacker repeatedly tries to replace `@m` by a junction to an outside folder while 400 files are written.
    #[test]
    fn a_racing_attacker_never_gets_a_file_written_outside_the_destination() {
        let root = scratch("race-root");
        let outside = scratch("race-outside");
        fs::create_dir_all(root.join("@m")).unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let swaps = Arc::new(AtomicUsize::new(0));
        let attacker = {
            let (root, outside, stop, swaps) =
                (root.clone(), outside.clone(), stop.clone(), swaps.clone());
            thread::spawn(move || {
                while !stop.load(Ordering::SeqCst) {
                    if fs::rename(root.join("@m"), root.join("@m_away")).is_ok() {
                        let linked = Command::new("cmd")
                            .args(["/C", "mklink", "/J"])
                            .arg(root.join("@m"))
                            .arg(&outside)
                            .output()
                            .is_ok_and(|o| o.status.success());
                        if linked {
                            swaps.fetch_add(1, Ordering::SeqCst);
                            thread::sleep(Duration::from_millis(5));
                            let _ = fs::remove_dir(root.join("@m"));
                        }
                        let _ = fs::rename(root.join("@m_away"), root.join("@m"));
                    }
                    thread::sleep(Duration::from_micros(200));
                }
            })
        };
        // Control: with nothing pinned the attacker does succeed, so the test is sensitive to the attack.
        thread::sleep(Duration::from_millis(1500));
        assert!(
            swaps.load(Ordering::SeqCst) > 0,
            "the attacker must be able to swap an unpinned folder"
        );
        let swaps_before = swaps.load(Ordering::SeqCst);
        let mut store = LocalDirectory::open_exclusive(&root).unwrap();
        let (mut committed, mut refused) = (0, 0);
        for i in 0..400 {
            let target = path(&format!("@m/f{i}.pbo"));
            match store.begin(&target, 4) {
                Ok(mut staged) => {
                    if staged.write_all(b"data").is_ok() && staged.commit().is_ok() {
                        committed += 1;
                    }
                }
                Err(_) => refused += 1,
            }
            // give the attacker the gaps between operations, where no folder is pinned
            thread::sleep(Duration::from_millis(1));
        }
        stop.store(true, Ordering::SeqCst);
        attacker.join().unwrap();
        // restore a clean state for the cleanup below
        let _ = fs::remove_dir(root.join("@m"));
        let outside_files: Vec<_> = fs::read_dir(&outside).unwrap().collect();
        assert!(
            outside_files.is_empty(),
            "a file was written through the swapped link: {outside_files:?}"
        );
        println!(
            "control swaps {swaps_before}, committed {committed}, refused {refused}, swaps during the run {}",
            swaps.load(Ordering::SeqCst) - swaps_before
        );
        assert!(committed + refused > 0);
        let _ = fs::remove_dir_all(root);
        fs::remove_dir_all(outside).unwrap();
    }
}
