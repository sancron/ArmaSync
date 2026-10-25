// SPDX-License-Identifier: AGPL-3.0-only
//! Hardening of the partial files (MIG-032R2): a file outside the destination, linked to the name of a partial data
//! or state file (hard link, or symbolic link where the platform allows it), must never be truncated, appended to or
//! overwritten. Each case keeps a sentinel file outside the destination and checks its bytes afterwards. The data is
//! synthetic and the sentinel is created by the test itself.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use armasync_adapters::localfs::{LocalDirectory, PARTIAL_PREFIX};
use armasync_core::{LocalStore, PartialKey, RelativePath};

const SIZE: u64 = 4000;

fn scratch(name: &str) -> PathBuf {
    let base = std::env::temp_dir().join(format!(
        "armasync-partial-sentinel-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
    ));
    fs::create_dir_all(base.join("dest")).unwrap();
    base
}

fn key() -> PartialKey {
    PartialKey {
        source: "sentinel-source".to_owned(),
        path: RelativePath::parse("a.pbo").unwrap(),
        size: SIZE,
    }
}

/// The partial file names in the destination, with their full path.
fn partial_entries(root: &Path) -> Vec<PathBuf> {
    fs::read_dir(root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(PARTIAL_PREFIX))
        })
        .collect()
}

fn entry_with(root: &Path, suffix: &str) -> PathBuf {
    partial_entries(root)
        .into_iter()
        .find(|path| path.to_string_lossy().ends_with(suffix))
        .unwrap()
}

/// Leaves a kept partial file behind, as an interrupted run does.
fn leave_partial(root: &Path, bytes: &[u8]) {
    let mut destination = LocalDirectory::open_exclusive(root).unwrap();
    let mut partial = destination.partial(&key()).unwrap();
    partial.restart(Some("\"v1\"")).unwrap();
    partial.write_all(bytes).unwrap();
    partial.keep();
}

/// Makes `name` a link to `target`: a hard link, or a symbolic link where the platform allows creating one.
/// Returns `false` if the platform refused the link (then the case is skipped).
fn link_to(target: &Path, name: &Path) -> bool {
    fs::remove_file(name).unwrap();
    if fs::hard_link(target, name).is_ok() {
        return true;
    }
    #[cfg(windows)]
    let made = std::os::windows::fs::symlink_file(target, name);
    #[cfg(unix)]
    let made = std::os::unix::fs::symlink(target, name);
    made.is_ok()
}

#[test]
fn a_linked_data_file_is_never_truncated_or_appended_to() {
    let base = scratch("data");
    let root = base.join("dest");
    leave_partial(&root, &[7; 1000]);
    let sentinel = base.join("sentinel.bin");
    let original = vec![0x5A; 2000];
    fs::write(&sentinel, &original).unwrap();
    if !link_to(&sentinel, &entry_with(&root, ".part")) {
        eprintln!("skipped: the platform refused a link");
        return;
    }

    let mut destination = LocalDirectory::open_exclusive(&root).unwrap();
    let mut partial = destination.partial(&key()).unwrap();
    partial.write_all(&[1; 100]).unwrap();
    partial.discard();
    drop(destination);

    assert_eq!(
        fs::read(&sentinel).unwrap(),
        original,
        "the file outside the destination is untouched"
    );
    fs::remove_dir_all(base).unwrap();
}

#[test]
fn a_linked_state_file_is_replaced_and_its_target_is_never_written() {
    let base = scratch("state");
    let root = base.join("dest");
    leave_partial(&root, &[7; 1000]);
    let sentinel = base.join("sentinel.txt");
    fs::write(&sentinel, b"not a state file, do not touch").unwrap();
    // The state is unreadable as a state, so the next run starts fresh and writes a new state.
    if !link_to(&sentinel, &entry_with(&root, ".state")) {
        eprintln!("skipped: the platform refused a link");
        return;
    }

    let mut destination = LocalDirectory::open_exclusive(&root).unwrap();
    let mut partial = destination.partial(&key()).unwrap();
    assert!(partial.stored().is_none());
    partial.restart(Some("\"v2\"")).unwrap();
    partial.discard();
    drop(destination);

    assert_eq!(
        fs::read(&sentinel).unwrap(),
        b"not a state file, do not touch",
        "a link at the state name is replaced, its target keeps its bytes"
    );
    fs::remove_dir_all(base).unwrap();
}

#[test]
fn a_linked_data_file_on_a_fresh_start_is_removed_as_a_name_only() {
    let base = scratch("fresh");
    let root = base.join("dest");
    leave_partial(&root, &[7; 1000]);
    // Without a valid state the data is never continued: the name is cleared and recreated.
    let state = entry_with(&root, ".state");
    fs::remove_file(&state).unwrap();
    let sentinel = base.join("sentinel.bin");
    let original = vec![0x33; 3000];
    fs::write(&sentinel, &original).unwrap();
    if !link_to(&sentinel, &entry_with(&root, ".part")) {
        eprintln!("skipped: the platform refused a link");
        return;
    }

    let mut destination = LocalDirectory::open_exclusive(&root).unwrap();
    let mut partial = destination.partial(&key()).unwrap();
    assert!(partial.stored().is_none());
    partial.write_all(&[2; 200]).unwrap();
    partial.discard();
    drop(destination);

    assert_eq!(fs::read(&sentinel).unwrap(), original);
    fs::remove_dir_all(base).unwrap();
}
