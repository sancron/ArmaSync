// SPDX-License-Identifier: AGPL-3.0-only
//! Resumable transfers (MIG-032R, ADR-0009) on the real local file system: the partial file survives a new handle
//! (a new process takes the lock again), cancellation leaves no file behind, damaged or foreign state is never
//! continued, stale leftovers are removed by a scan under the lock, and the target is untouched until verified.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use armasync_adapters::InMemoryRepository;
use armasync_adapters::localfs::LocalDirectory;
use armasync_application::sync::ApplyReport;
use armasync_core::{CancelToken, Error, ErrorKind, HashAlgorithm, LocalStore};
use common::{CUT, OnResume, Recording, big_file, content, run};

fn scratch(name: &str) -> PathBuf {
    let base = std::env::temp_dir().join(format!(
        "armasync-localfs-resume-{name}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
    ));
    fs::create_dir_all(&base).unwrap();
    base
}

/// One run in a fresh handle, as a new process would do it: the lock is taken and released again.
fn run_in_destination(
    source: &Recording,
    root: &Path,
    cancel: &CancelToken,
) -> Result<ApplyReport, Error> {
    let mut destination = LocalDirectory::open_exclusive(root)?;
    run(source, &mut destination, cancel)
}

/// The leftovers of resumable transfers in the destination (their names carry the partial prefix).
fn partial_files(root: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(root)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with(armasync_adapters::localfs::PARTIAL_PREFIX))
        .collect();
    names.sort();
    names
}

fn repository(data: &[u8]) -> Recording {
    Recording::new(InMemoryRepository::new("rev-1", &[("a.pbo", data)]).unwrap())
}

#[test]
fn a_partial_file_survives_a_new_process_and_the_next_run_continues_with_a_range() {
    let root = scratch("restart");
    let data = big_file();
    let source = repository(&data);

    source.cut_at.set(Some(CUT));
    let error = run_in_destination(&source, &root, &CancelToken::new()).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Network);
    assert!(
        !root.join("a.pbo").exists(),
        "the target does not exist yet"
    );
    assert_eq!(
        partial_files(&root).len(),
        2,
        "data and state of the partial file"
    );

    source.cut_at.set(None);
    run_in_destination(&source, &root, &CancelToken::new()).unwrap();
    assert_eq!(fs::read(root.join("a.pbo")).unwrap(), data);
    assert!(
        partial_files(&root).is_empty(),
        "nothing is left after a verified file"
    );
    let opens = source.opens.borrow();
    assert_eq!(opens[1].0, CUT, "the new process asked for the rest only");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_cancelled_download_leaves_no_partial_file() {
    let root = scratch("cancel");
    let data = big_file();
    let source = repository(&data);
    let cancel = CancelToken::new();
    *source.cancel_after.borrow_mut() = Some((CUT, cancel.clone()));

    let error = run_in_destination(&source, &root, &cancel).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Cancelled);
    assert!(
        partial_files(&root).is_empty(),
        "a cancellation keeps no temporary file"
    );
    assert!(!root.join("a.pbo").exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_damaged_state_is_never_continued_and_the_old_file_stays_until_the_new_one_is_verified() {
    let root = scratch("damaged");
    let data = big_file();
    let source = repository(&data);
    fs::write(root.join("a.pbo"), b"previous good content").unwrap();

    source.cut_at.set(Some(CUT));
    run_in_destination(&source, &root, &CancelToken::new()).unwrap_err();
    assert_eq!(
        fs::read(root.join("a.pbo")).unwrap(),
        b"previous good content"
    );

    let state = partial_files(&root)
        .into_iter()
        .find(|name| name.ends_with(".state"))
        .unwrap();
    fs::write(root.join(state), "not a state file\n").unwrap();

    source.cut_at.set(None);
    run_in_destination(&source, &root, &CancelToken::new()).unwrap();
    assert_eq!(fs::read(root.join("a.pbo")).unwrap(), data);
    let opens = source.opens.borrow();
    assert_eq!(opens[1].0, 0, "an unreadable state is never continued");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_partial_file_of_another_size_is_never_continued() {
    let root = scratch("size");
    let old = big_file();
    let mut source = repository(&old);

    source.cut_at.set(Some(CUT));
    run_in_destination(&source, &root, &CancelToken::new()).unwrap_err();
    assert_eq!(partial_files(&root).len(), 2);

    let new = content(old.len() + 77, 4);
    source.publish("rev-2", &[("a.pbo", &new)]);
    source.cut_at.set(None);
    run_in_destination(&source, &root, &CancelToken::new()).unwrap();
    assert_eq!(fs::read(root.join("a.pbo")).unwrap(), new);
    let opens = source.opens.borrow();
    assert_eq!(
        opens[1].0, 0,
        "the key includes the size: other bytes are not reused"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_scan_under_the_lock_removes_partial_files_older_than_the_limit() {
    let root = scratch("stale");
    let data = big_file();
    let source = repository(&data);

    source.cut_at.set(Some(CUT));
    run_in_destination(&source, &root, &CancelToken::new()).unwrap_err();
    let long_ago = SystemTime::now() - Duration::from_secs(8 * 24 * 60 * 60);
    for name in partial_files(&root) {
        OpenOptions::new()
            .write(true)
            .open(root.join(&name))
            .unwrap()
            .set_modified(long_ago)
            .unwrap();
    }

    let destination = LocalDirectory::open_exclusive(&root).unwrap();
    let listed = destination
        .list(HashAlgorithm::Sha256, &CancelToken::new())
        .unwrap();
    assert!(listed.is_empty(), "partial files are never listed");
    assert!(
        partial_files(&root).is_empty(),
        "partial files older than the limit are removed by a scan under the lock"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_refused_resume_on_disk_restarts_once_from_zero() {
    let root = scratch("refused");
    let data = big_file();
    let source = repository(&data);

    source.cut_at.set(Some(CUT));
    run_in_destination(&source, &root, &CancelToken::new()).unwrap_err();
    source.cut_at.set(None);
    source.on_resume.set(OnResume::Refuse);
    run_in_destination(&source, &root, &CancelToken::new()).unwrap();
    assert_eq!(fs::read(root.join("a.pbo")).unwrap(), data);
    let opens = source.opens.borrow();
    assert_eq!(
        opens.len(),
        3,
        "first run, refused resume, one fresh request"
    );
    assert_eq!(opens[2].0, 0);
    fs::remove_dir_all(root).unwrap();
}
