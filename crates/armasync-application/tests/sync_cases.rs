// SPDX-License-Identifier: AGPL-3.0-only
//! Use-case tests derived from realistic repository situations (written before the implementation was finished):
//! first sync, repeat sync, changed mod, extra files, corrupted transfer, outage, full disk, stale plan, cancel.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;

use armasync_adapters::{InMemoryRepository, InMemoryStore, MemoryEventSink, Sha256Hasher};
use armasync_application::sync::{ApplyReport, SyncContext, apply, plan};
use armasync_core::{
    CancelToken, EVENT_SCHEMA_VERSION, ErrorKind, EventSink, HashAlgorithm, JobState, LocalStore,
    OperationEvent, OperationId, PlanOptions, RelativePath, SyncStep,
};

fn path(text: &str) -> RelativePath {
    RelativePath::parse(text).unwrap()
}

fn repo() -> InMemoryRepository {
    InMemoryRepository::new(
        "rev-1",
        &[
            ("@cba/addons/main.pbo", b"cba main".as_slice()),
            ("@cba/addons/xeh.pbo", b"cba xeh".as_slice()),
            ("@ace/addons/medical.pbo", b"ace medical".as_slice()),
            ("keys/cba.bikey", b"key".as_slice()),
        ],
    )
    .unwrap()
}

struct Outcome {
    result: Result<ApplyReport, armasync_core::Error>,
    events: Vec<OperationEvent>,
}

fn run_apply(
    plan_to_apply: &armasync_core::SyncPlan,
    options: &PlanOptions,
    source: &InMemoryRepository,
    store: &mut InMemoryStore,
    cancel: &CancelToken,
) -> Outcome {
    let mut sink = MemoryEventSink::default();
    let hasher = Sha256Hasher;
    let result = {
        let mut context = SyncContext {
            source,
            store,
            hasher: &hasher,
            unpacker: &armasync_adapters::archive::ZipUnpacker,
            sink: &mut sink,
            cancel,
        };
        apply(OperationId::new(1), plan_to_apply, options, &mut context)
    };
    Outcome {
        result,
        events: sink.events,
    }
}

fn sync(source: &InMemoryRepository, store: &mut InMemoryStore, options: &PlanOptions) -> Outcome {
    let planned = plan(source, store, options).unwrap();
    run_apply(&planned, options, source, store, &CancelToken::new())
}

fn assert_event_contract(events: &[OperationEvent]) {
    assert!(!events.is_empty());
    for (index, event) in events.iter().enumerate() {
        assert_eq!(event.schema_version, EVENT_SCHEMA_VERSION);
        assert_eq!(
            event.sequence, index as u64,
            "sequence must start at 0 and increase by 1"
        );
        assert_eq!(event.operation, OperationId::new(1));
    }
    let last = events.last().unwrap();
    assert!(
        last.state.is_terminal(),
        "last event must be terminal, got {:?}",
        last.state
    );
    assert!(
        events[..events.len() - 1]
            .iter()
            .all(|event| !event.state.is_terminal())
    );
}

#[test]
fn first_sync_downloads_everything_and_the_second_plan_is_a_noop() {
    let source = repo();
    let mut store = InMemoryStore::new();
    let outcome = sync(&source, &mut store, &PlanOptions::default());
    let report = outcome.result.unwrap();
    assert_eq!(report.state, JobState::Succeeded);
    assert_eq!(
        (report.downloaded, report.replaced, report.deleted),
        (4, 0, 0)
    );
    assert_eq!(
        store.content("@cba/addons/main.pbo"),
        Some(b"cba main".as_slice())
    );
    assert_event_contract(&outcome.events);
    assert_eq!(outcome.events.last().unwrap().state, JobState::Succeeded);
    assert_eq!(outcome.events.last().unwrap().progress.files_done, 4);
    assert_eq!(outcome.events.last().unwrap().error, None);

    let again = plan(&source, &store, &PlanOptions::default()).unwrap();
    assert!(again.is_noop());
    assert_eq!(again.summary.keep_files, 4);
}

#[test]
fn updated_mod_is_replaced_and_unchanged_files_are_not_touched() {
    let mut source = repo();
    let mut store = InMemoryStore::new();
    sync(&source, &mut store, &PlanOptions::default())
        .result
        .unwrap();

    let mut files = BTreeMap::new();
    for (name, data) in [
        ("@cba/addons/main.pbo", b"cba main v2".as_slice()),
        ("@cba/addons/xeh.pbo", b"cba xeh".as_slice()),
        ("@ace/addons/medical.pbo", b"ace medical".as_slice()),
        ("keys/cba.bikey", b"key".as_slice()),
        ("@ace/addons/new.pbo", b"new".as_slice()),
    ] {
        files.insert(path(name), data.to_vec());
    }
    source.publish("rev-2", files);

    let planned = plan(&source, &store, &PlanOptions::default()).unwrap();
    assert_eq!(planned.revision, "rev-2");
    assert_eq!(planned.summary.replace_files, 1);
    assert_eq!(planned.summary.download_files, 1);
    assert_eq!(planned.summary.keep_files, 3);
    let report = run_apply(
        &planned,
        &PlanOptions::default(),
        &source,
        &mut store,
        &CancelToken::new(),
    )
    .result
    .unwrap();
    assert_eq!((report.downloaded, report.replaced, report.kept), (1, 1, 3));
    assert_eq!(
        store.content("@cba/addons/main.pbo"),
        Some(b"cba main v2".as_slice())
    );
}

#[test]
fn extra_files_survive_by_default_and_are_deleted_only_when_requested_inside_managed_roots() {
    let source = repo();
    let mut store = InMemoryStore::new()
        .with_file("@cba/addons/old.pbo", b"stale")
        .unwrap()
        .with_file("userdata/profile.vars", b"mine")
        .unwrap();
    sync(&source, &mut store, &PlanOptions::default())
        .result
        .unwrap();
    assert!(
        store.content("@cba/addons/old.pbo").is_some(),
        "no deletion without the flag"
    );

    let options = PlanOptions {
        delete_extra: true,
        managed_roots: vec![path("@cba")],
        ..PlanOptions::default()
    };
    let planned = plan(&source, &store, &options).unwrap();
    assert!(planned.steps.iter().any(
        |step| matches!(step, SyncStep::Delete { path } if path.as_str() == "@cba/addons/old.pbo")
    ));
    assert_eq!(
        planned.warnings.len(),
        1,
        "userdata is outside the managed roots and must be reported, not deleted"
    );
    let report = run_apply(&planned, &options, &source, &mut store, &CancelToken::new())
        .result
        .unwrap();
    assert_eq!(report.deleted, 1);
    assert!(store.content("@cba/addons/old.pbo").is_none());
    assert_eq!(
        store.content("userdata/profile.vars"),
        Some(b"mine".as_slice())
    );
}

#[test]
fn corrupted_transfer_never_replaces_a_good_file() {
    let mut source = repo();
    let mut store = InMemoryStore::new()
        .with_file("@cba/addons/main.pbo", b"previous good content")
        .unwrap();
    source.serve_corrupted(&path("@cba/addons/main.pbo"), b"truncat");
    let outcome = sync(&source, &mut store, &PlanOptions::default());
    let error = outcome.result.unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Integrity);
    assert_eq!(
        store.content("@cba/addons/main.pbo"),
        Some(b"previous good content".as_slice())
    );
    assert_event_contract(&outcome.events);
    let last = outcome.events.last().unwrap();
    assert_eq!(
        (last.state, last.error),
        (JobState::Failed, Some(ErrorKind::Integrity))
    );
}

#[test]
fn same_size_corruption_is_caught_by_the_hash() {
    let mut source = repo();
    let mut store = InMemoryStore::new()
        .with_file("@cba/addons/main.pbo", b"previous good content")
        .unwrap();
    // "cba main" has 8 bytes; the corrupted copy has the same length, so only the hash can reveal it.
    source.serve_corrupted(&path("@cba/addons/main.pbo"), b"cba maiX");
    let error = sync(&source, &mut store, &PlanOptions::default())
        .result
        .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Integrity);
    assert_eq!(
        store.content("@cba/addons/main.pbo"),
        Some(b"previous good content".as_slice())
    );
}

#[test]
fn repository_outage_is_a_network_error_at_planning_and_at_apply() {
    let mut source = repo();
    let mut store = InMemoryStore::new();
    let planned = plan(&source, &store, &PlanOptions::default()).unwrap();
    source.set_unreachable(true);
    assert_eq!(
        plan(&source, &store, &PlanOptions::default())
            .unwrap_err()
            .kind(),
        ErrorKind::Network
    );
    let outcome = run_apply(
        &planned,
        &PlanOptions::default(),
        &source,
        &mut store,
        &CancelToken::new(),
    );
    assert_eq!(outcome.result.unwrap_err().kind(), ErrorKind::Network);
    assert!(store.paths().is_empty());
    assert_eq!(
        outcome.events.last().unwrap().error,
        Some(ErrorKind::Network)
    );
}

#[test]
fn full_disk_fails_with_filesystem_error_and_keeps_old_content() {
    let source = repo();
    let mut store = InMemoryStore::new()
        .with_file("keys/cba.bikey", b"old key")
        .unwrap();
    store.fail_writes_to(&path("keys/cba.bikey"));
    let outcome = sync(&source, &mut store, &PlanOptions::default());
    assert_eq!(outcome.result.unwrap_err().kind(), ErrorKind::FileSystem);
    assert_eq!(store.content("keys/cba.bikey"), Some(b"old key".as_slice()));
    assert_eq!(outcome.events.last().unwrap().state, JobState::Failed);
}

#[test]
fn stale_plan_is_rejected_when_the_revision_changes_before_apply() {
    let mut source = repo();
    let mut store = InMemoryStore::new();
    let planned = plan(&source, &store, &PlanOptions::default()).unwrap();
    // Same file set, one file has new content: without revalidation the new bytes would verify against the new
    // manifest and silently be applied although the user reviewed a plan for rev-1.
    let mut files = BTreeMap::new();
    for (name, data) in [
        ("@cba/addons/main.pbo", b"cba main CHANGED".as_slice()),
        ("@cba/addons/xeh.pbo", b"cba xeh".as_slice()),
        ("@ace/addons/medical.pbo", b"ace medical".as_slice()),
        ("keys/cba.bikey", b"key".as_slice()),
    ] {
        files.insert(path(name), data.to_vec());
    }
    source.publish("rev-2", files);
    let outcome = run_apply(
        &planned,
        &PlanOptions::default(),
        &source,
        &mut store,
        &CancelToken::new(),
    );
    assert_eq!(outcome.result.unwrap_err().kind(), ErrorKind::Integrity);
    assert!(
        store.paths().is_empty(),
        "nothing may be written for a stale plan"
    );
}

#[test]
fn stale_plan_is_rejected_when_local_files_change_before_apply() {
    let source = repo();
    let mut store = InMemoryStore::new();
    let planned = plan(&source, &store, &PlanOptions::default()).unwrap();
    store
        .write(&path("keys/cba.bikey"), b"changed by user meanwhile")
        .unwrap();
    let outcome = run_apply(
        &planned,
        &PlanOptions::default(),
        &source,
        &mut store,
        &CancelToken::new(),
    );
    assert_eq!(outcome.result.unwrap_err().kind(), ErrorKind::Integrity);
    assert_eq!(store.paths(), ["keys/cba.bikey"]);
    assert_eq!(
        store.content("keys/cba.bikey"),
        Some(b"changed by user meanwhile".as_slice())
    );
}

#[test]
fn cancel_before_start_writes_nothing_and_reports_cancelled_not_success() {
    let source = repo();
    let mut store = InMemoryStore::new();
    let planned = plan(&source, &store, &PlanOptions::default()).unwrap();
    let cancel = CancelToken::new();
    cancel.cancel();
    let outcome = run_apply(
        &planned,
        &PlanOptions::default(),
        &source,
        &mut store,
        &cancel,
    );
    assert_eq!(outcome.result.unwrap_err().kind(), ErrorKind::Cancelled);
    assert!(store.paths().is_empty());
    let last = outcome.events.last().unwrap();
    assert_eq!(
        (last.state, last.error),
        (JobState::Cancelled, Some(ErrorKind::Cancelled))
    );
    assert_event_contract(&outcome.events);
}

/// Cancels the operation as soon as the first file has been completed.
struct CancelAfterFirstFile {
    token: CancelToken,
    events: Vec<OperationEvent>,
}

impl EventSink for CancelAfterFirstFile {
    fn emit(&mut self, event: OperationEvent) {
        if event.progress.files_done >= 1 {
            self.token.cancel();
        }
        self.events.push(event);
    }
}

#[test]
fn cancel_in_the_middle_leaves_only_complete_files_and_a_cancelled_state() {
    let source = repo();
    let mut store = InMemoryStore::new();
    let options = PlanOptions::default();
    let planned = plan(&source, &store, &options).unwrap();
    let token = CancelToken::new();
    let mut sink = CancelAfterFirstFile {
        token: token.clone(),
        events: Vec::new(),
    };
    let hasher = Sha256Hasher;
    let result = {
        let mut context = SyncContext {
            source: &source,
            store: &mut store,
            hasher: &hasher,
            unpacker: &armasync_adapters::archive::ZipUnpacker,
            sink: &mut sink,
            cancel: &token,
        };
        apply(OperationId::new(1), &planned, &options, &mut context)
    };
    assert_eq!(result.unwrap_err().kind(), ErrorKind::Cancelled);
    assert_eq!(
        store.paths().len(),
        1,
        "exactly the file completed before cancellation exists"
    );
    for stored in store
        .list(HashAlgorithm::Sha256, &CancelToken::new())
        .unwrap()
    {
        let expected = source.manifest().unwrap();
        let entry = expected.get(&stored.path).unwrap();
        assert_eq!(stored.hash.as_ref(), Some(&entry.hash), "no partial file");
    }
    assert_event_contract(&sink.events);
    assert_eq!(sink.events.last().unwrap().state, JobState::Cancelled);

    // A later run continues where the cancelled one stopped.
    let resumed = sync(&source, &mut store, &options).result.unwrap();
    assert_eq!(resumed.downloaded, 3);
    assert_eq!(resumed.kept, 1);
}

use armasync_core::RepositorySource;
