// SPDX-License-Identifier: AGPL-3.0-only
//! Resumable transfers (MIG-032R, ADR-0009) against the in-memory reference: a continued transfer, a changed or
//! missing validator, an ignored range, a refused resume, wrong bytes after a resume, cancellation. Nothing here
//! touches the file system or the network; see `localfs_resume.rs` and the loopback tests for those.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use armasync_adapters::{InMemoryRepository, InMemoryStore, ValidatorMode};
use armasync_core::{CancelToken, ErrorKind, is_strong_validator};
use common::{CUT, OnResume, Recording, big_file, content, run};

fn repository(data: &[u8]) -> Recording {
    Recording::new(InMemoryRepository::new("rev-1", &[("a.pbo", data)]).unwrap())
}

fn validator_of(opens: &[(u64, Option<String>)], index: usize) -> String {
    opens[index]
        .1
        .clone()
        .expect("the second request carries If-Range")
}

#[test]
fn an_interrupted_download_keeps_its_bytes_and_the_next_run_continues_with_a_range() {
    let data = big_file();
    let source = repository(&data);
    let mut store = InMemoryStore::new();

    source.cut_at.set(Some(CUT));
    let error = run(&source, &mut store, &CancelToken::new()).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Network);
    assert!(store.content("a.pbo").is_none(), "nothing is installed");
    assert_eq!(
        store.partial_count(),
        1,
        "the received part is kept for a resume"
    );

    source.cut_at.set(None);
    let report = run(&source, &mut store, &CancelToken::new()).unwrap();
    assert_eq!(report.downloaded, 1);
    assert_eq!(store.content("a.pbo"), Some(data.as_slice()));
    assert_eq!(
        store.partial_count(),
        0,
        "a verified file leaves no partial file"
    );

    let opens = source.opens.borrow();
    assert_eq!(opens[0].0, 0);
    assert_eq!(opens[1].0, CUT, "the second run asks for the rest only");
    assert!(is_strong_validator(&validator_of(&opens, 1)));
}

#[test]
fn a_changed_file_restarts_and_never_mixes_old_and_new_bytes() {
    let old = big_file();
    let new = content(old.len(), 9);
    let mut source = repository(&old);
    let mut store = InMemoryStore::new();

    source.cut_at.set(Some(CUT));
    run(&source, &mut store, &CancelToken::new()).unwrap_err();
    source.publish("rev-2", &[("a.pbo", &new)]);
    source.cut_at.set(None);

    let report = run(&source, &mut store, &CancelToken::new()).unwrap();
    assert_eq!(report.downloaded, 1);
    assert_eq!(
        store.content("a.pbo"),
        Some(new.as_slice()),
        "the stored bytes of the old revision are not continued"
    );
    let opens = source.opens.borrow();
    assert_eq!(opens[1].0, CUT, "the run asked to continue...");
    assert_eq!(store.partial_count(), 0);
}

#[test]
fn a_weak_or_missing_validator_never_resumes() {
    for mode in [ValidatorMode::Weak, ValidatorMode::None] {
        let data = big_file();
        let source = Recording::new(
            InMemoryRepository::new("rev-1", &[("a.pbo", &data)])
                .unwrap()
                .with_validator(mode),
        );
        let mut store = InMemoryStore::new();

        source.cut_at.set(Some(CUT));
        run(&source, &mut store, &CancelToken::new()).unwrap_err();
        assert_eq!(
            store.partial_count(),
            0,
            "{mode:?}: without a strong validator nothing is kept"
        );

        source.cut_at.set(None);
        run(&source, &mut store, &CancelToken::new()).unwrap();
        assert_eq!(store.content("a.pbo"), Some(data.as_slice()));
        let opens = source.opens.borrow();
        assert!(
            opens
                .iter()
                .all(|(offset, if_range)| *offset == 0 && if_range.is_none()),
            "{mode:?}: every request starts from zero without If-Range"
        );
    }
}

#[test]
fn an_ignored_range_restarts_from_zero_and_installs_the_whole_file() {
    let data = big_file();
    let source = Recording::new(
        InMemoryRepository::new("rev-1", &[("a.pbo", &data)])
            .unwrap()
            .ignoring_ranges(),
    );
    let mut store = InMemoryStore::new();

    source.cut_at.set(Some(CUT));
    run(&source, &mut store, &CancelToken::new()).unwrap_err();
    assert_eq!(store.partial_count(), 1);

    source.cut_at.set(None);
    run(&source, &mut store, &CancelToken::new()).unwrap();
    assert_eq!(store.content("a.pbo"), Some(data.as_slice()));
    let opens = source.opens.borrow();
    assert_eq!(opens[1].0, CUT, "the run asked for the rest...");
    assert_eq!(store.partial_count(), 0);
}

#[test]
fn a_refused_resume_is_retried_once_from_zero() {
    let data = big_file();
    let source = repository(&data);
    let mut store = InMemoryStore::new();

    source.cut_at.set(Some(CUT));
    run(&source, &mut store, &CancelToken::new()).unwrap_err();
    source.cut_at.set(None);
    source.on_resume.set(OnResume::Refuse);

    run(&source, &mut store, &CancelToken::new()).unwrap();
    assert_eq!(store.content("a.pbo"), Some(data.as_slice()));
    let opens = source.opens.borrow();
    assert_eq!(
        opens.len(),
        3,
        "first run, refused resume, one fresh request"
    );
    assert_eq!(opens[1].0, CUT);
    assert_eq!(opens[2].0, 0);
}

#[test]
fn wrong_bytes_after_a_resume_are_rejected_and_the_target_keeps_its_old_content() {
    let data = big_file();
    let source = repository(&data);
    let mut store = InMemoryStore::new()
        .with_file("a.pbo", b"previous good content")
        .unwrap();

    source.cut_at.set(Some(CUT));
    run(&source, &mut store, &CancelToken::new()).unwrap_err();
    assert_eq!(
        store.content("a.pbo"),
        Some(b"previous good content".as_slice())
    );
    source.cut_at.set(None);
    source.on_resume.set(OnResume::LieAboutBytes);

    let error = run(&source, &mut store, &CancelToken::new()).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Integrity);
    assert_eq!(
        store.content("a.pbo"),
        Some(b"previous good content".as_slice()),
        "a failed verification never replaces the target"
    );
    assert_eq!(
        store.partial_count(),
        0,
        "bytes that failed verification are discarded, not resumed again"
    );
}

#[test]
fn cancelling_a_resumable_download_discards_its_bytes() {
    let data = big_file();
    let source = repository(&data);
    let mut store = InMemoryStore::new();
    let cancel = CancelToken::new();

    *source.cancel_after.borrow_mut() = Some((CUT, cancel.clone()));
    let error = run(&source, &mut store, &cancel).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Cancelled);
    assert_eq!(
        store.partial_count(),
        0,
        "a cooperative cancellation leaves no temporary file (ADR-0009, section 5)"
    );
    assert!(store.content("a.pbo").is_none());

    *source.cancel_after.borrow_mut() = None;
    run(&source, &mut store, &CancelToken::new()).unwrap();
    let opens = source.opens.borrow();
    assert_eq!(opens[1].0, 0, "nothing is continued after a cancellation");
    assert_eq!(store.content("a.pbo"), Some(data.as_slice()));
}
