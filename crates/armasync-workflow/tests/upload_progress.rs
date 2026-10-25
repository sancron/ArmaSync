// SPDX-License-Identifier: AGPL-3.0-only
//! Upload events of the shared wiring (MIG-042, step 4c; ADR-0016) against the loopback WebDAV server of the test kit,
//! over anonymous HTTP. Order and sequence, counters, byte progress, cancellation inside a file, a failing target, and
//! that no event carries the password, the address or the local folder. Synthetic data only.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use armasync_core::{
    CancelToken, ErrorKind, OperationId, UploadEvent, UploadEventKind, UploadEventSink,
};
use armasync_testkit::webdav::{Server, start};
use armasync_workflow::upload::{UploadRequest, publish_observed};

/// Collects the events of one operation; cloning shares the list.
#[derive(Clone, Default)]
struct Recorder(Arc<Mutex<Vec<UploadEvent>>>);

impl Recorder {
    fn events(&self) -> Vec<UploadEvent> {
        self.0.lock().unwrap().clone()
    }
}

impl UploadEventSink for Recorder {
    fn emit(&mut self, event: UploadEvent) {
        self.0.lock().unwrap().push(event);
    }
}

/// Records the events and cancels the operation as soon as bytes of a file were read.
struct CancelOnBytes {
    recorder: Recorder,
    cancel: CancelToken,
}

impl UploadEventSink for CancelOnBytes {
    fn emit(&mut self, event: UploadEvent) {
        if matches!(event.kind, UploadEventKind::FileBytes { .. }) {
            self.cancel.cancel();
        }
        self.recorder.emit(event);
    }
}

fn tree(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "armasync-wf-progress-{}-{name}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    for (relative, bytes) in [
        ("a.txt", b"alpha".as_slice()),
        ("docs/b.txt", b"beta"),
        (".armasync-lock", b""),
    ] {
        let target = root.join(relative);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(target, bytes).unwrap();
    }
    root
}

fn request(root: &Path, target: String) -> UploadRequest {
    UploadRequest {
        source: root.to_path_buf(),
        target,
        user: None,
        password: None,
        exclusions: Vec::new(),
        http: armasync_adapters::http::HttpConfig::default(),
    }
}

fn anonymous_target(server: &Server) -> String {
    format!("http://{}/dav", server.addr)
}

/// Asserts the shape every operation has: sequence 0, 1, 2, … and the schema version and operation of the run.
fn assert_sequence(events: &[UploadEvent], operation: u64) {
    for (index, event) in events.iter().enumerate() {
        assert_eq!(event.sequence, index as u64, "{events:?}");
        assert_eq!(event.schema_version, 1);
        assert_eq!(event.operation, OperationId::new(operation));
    }
}

fn terminal_count(events: &[UploadEvent]) -> usize {
    events.iter().filter(|e| e.kind.is_terminal()).count()
}

#[test]
fn events_follow_the_plan_order_and_count_every_byte() {
    let root = tree("order");
    let server = start(false, false, "");
    let recorder = Recorder::default();
    let report = publish_observed(
        &request(&root, anonymous_target(&server)),
        &CancelToken::new(),
        OperationId::new(7),
        Box::new(recorder.clone()),
    )
    .unwrap();
    assert_eq!(report.files_uploaded, 2);

    let events = recorder.events();
    assert_sequence(&events, 7);
    assert!(matches!(events[0].kind, UploadEventKind::Started));
    assert!(matches!(
        events.last().unwrap().kind,
        UploadEventKind::Finished
    ));
    assert_eq!(terminal_count(&events), 1, "{events:?}");

    // Start totals come from the plan: two files, nine bytes; the internal file is not counted.
    assert_eq!(events[0].progress.files_total, 2);
    assert_eq!(events[0].progress.bytes_total, 9);

    // Each file: started with its size, bytes strictly growing up to that size, then finished, in plan order.
    let body = &events[1..events.len() - 1];
    let mut index = 0;
    for (path, size) in [("a.txt", 5_u64), ("docs/b.txt", 4)] {
        let base = index;
        match &body[index].kind {
            UploadEventKind::FileStarted { path: p, size: s } => {
                assert_eq!((p.as_str(), *s), (path, size));
            }
            other => panic!("expected file start, got {other:?}"),
        }
        index += 1;
        let mut last = 0;
        while let UploadEventKind::FileBytes {
            path: p,
            bytes_done,
        } = &body[index].kind
        {
            assert_eq!(p.as_str(), path);
            assert!(
                *bytes_done > last,
                "bytes must grow: {bytes_done} after {last}"
            );
            last = *bytes_done;
            index += 1;
        }
        assert_eq!(last, size, "the last block reaches the file size");
        match &body[index].kind {
            UploadEventKind::FileFinished { path: p } => assert_eq!(p.as_str(), path),
            other => panic!("expected file end, got {other:?} after {base}"),
        }
        index += 1;
    }
    assert_eq!(index, body.len(), "no event between the files");

    // The counters: after the first file five bytes; the total never decreases; the end is complete.
    let finished_first = body
        .iter()
        .find(|e| matches!(&e.kind, UploadEventKind::FileFinished { path } if path.as_str() == "a.txt"))
        .unwrap();
    assert_eq!(finished_first.progress.files_done, 1);
    assert_eq!(finished_first.progress.bytes_done, 5);
    let mut previous = 0;
    for event in &events {
        assert!(event.progress.bytes_done >= previous, "{events:?}");
        previous = event.progress.bytes_done;
        assert_eq!(event.progress.bytes_total, 9);
    }
    let end = events.last().unwrap().progress;
    assert_eq!((end.files_done, end.files_total), (2, 2));
    assert_eq!((end.bytes_done, end.bytes_total), (9, 9));
    assert!(server.file("current.json").is_some());

    // The events do not show where the data came from or went to.
    let text = format!("{events:?}");
    let root_name = root.file_name().unwrap().to_string_lossy().into_owned();
    assert!(
        !text.contains(&root_name),
        "a local folder appears in an event"
    );
    assert!(
        !text.contains(&server.addr.to_string()),
        "an address appears in an event"
    );
    fs::remove_dir_all(root).ok();
}

#[test]
fn a_repeated_publication_reports_only_its_start_and_its_end() {
    let root = tree("repeat");
    let server = start(false, false, "");
    let target = anonymous_target(&server);
    publish_observed(
        &request(&root, target.clone()),
        &CancelToken::new(),
        OperationId::new(1),
        Box::new(Recorder::default()),
    )
    .unwrap();

    let recorder = Recorder::default();
    publish_observed(
        &request(&root, target),
        &CancelToken::new(),
        OperationId::new(2),
        Box::new(recorder.clone()),
    )
    .unwrap();
    let events = recorder.events();
    assert_sequence(&events, 2);
    assert_eq!(events.len(), 2, "{events:?}");
    assert!(matches!(events[0].kind, UploadEventKind::Started));
    assert!(matches!(events[1].kind, UploadEventKind::Finished));
    assert_eq!(events[1].progress.files_total, 0);
    fs::remove_dir_all(root).ok();
}

#[test]
fn cancelling_inside_a_file_ends_with_cancelled_and_publishes_no_pointer() {
    let root = tree("cancel");
    let server = start(false, false, "");
    let cancel = CancelToken::new();
    let recorder = Recorder::default();
    let sink = CancelOnBytes {
        recorder: recorder.clone(),
        cancel: cancel.clone(),
    };
    let result = publish_observed(
        &request(&root, anonymous_target(&server)),
        &cancel,
        OperationId::new(3),
        Box::new(sink),
    );
    assert_eq!(result.unwrap_err().kind(), ErrorKind::Cancelled);

    let events = recorder.events();
    assert_sequence(&events, 3);
    assert_eq!(terminal_count(&events), 1, "{events:?}");
    assert!(matches!(
        events.last().unwrap().kind,
        UploadEventKind::Cancelled
    ));
    assert!(
        events
            .iter()
            .any(|e| matches!(e.kind, UploadEventKind::FileBytes { .. })),
        "the cancel happened while bytes were sent"
    );
    assert!(server.file("current.json").is_none());
    fs::remove_dir_all(root).ok();
}

#[test]
fn a_failing_target_ends_with_failed_and_only_its_category() {
    let root = tree("fail");
    // The revision identifier depends only on the content: a probe run on another server reveals it, so the failing
    // PUT can name the exact path (the test kit matches whole paths).
    let probe = start(false, false, "");
    publish_observed(
        &request(&root, anonymous_target(&probe)),
        &CancelToken::new(),
        OperationId::new(9),
        Box::new(Recorder::default()),
    )
    .unwrap();
    let revision = probe
        .file_names()
        .into_iter()
        .find_map(|name| {
            name.strip_prefix("revisions/")
                .and_then(|rest| rest.split('/').next())
                .filter(|id| id.starts_with("r-"))
                .map(str::to_owned)
        })
        .expect("the probe published a revision");

    let server = start(false, false, "");
    server.fail_puts_to(&format!("revisions/{revision}/files/docs/b.txt"));
    let recorder = Recorder::default();
    let result = publish_observed(
        &request(&root, anonymous_target(&server)),
        &CancelToken::new(),
        OperationId::new(4),
        Box::new(recorder.clone()),
    );
    let error = result.unwrap_err();

    let events = recorder.events();
    assert_sequence(&events, 4);
    assert_eq!(terminal_count(&events), 1, "{events:?}");
    match &events.last().unwrap().kind {
        UploadEventKind::Failed { kind } => assert_eq!(*kind, error.kind()),
        other => panic!("expected a failure, got {other:?}"),
    }
    // The first file was complete before the second one failed.
    assert!(events.iter().any(|e| matches!(
        &e.kind,
        UploadEventKind::FileFinished { path } if path.as_str() == "a.txt"
    )));
    assert!(!events.iter().any(|e| matches!(
        &e.kind,
        UploadEventKind::FileFinished { path } if path.as_str() == "docs/b.txt"
    )));
    assert!(server.file("current.json").is_none());
    fs::remove_dir_all(root).ok();
}

#[test]
fn a_refusal_before_connecting_is_a_failed_event_and_keeps_the_password_out() {
    let root = tree("refusal");
    let server = start(false, false, "");
    let recorder = Recorder::default();
    let request = UploadRequest {
        user: Some("alice".to_owned()),
        password: Some("pw-very-secret".to_owned()),
        ..request(&root, anonymous_target(&server))
    };
    let result = publish_observed(
        &request,
        &CancelToken::new(),
        OperationId::new(5),
        Box::new(recorder.clone()),
    );
    assert_eq!(result.unwrap_err().kind(), ErrorKind::Config);
    assert!(server.seen().is_empty(), "no connection is made");

    let events = recorder.events();
    assert_eq!(events.len(), 2, "{events:?}");
    assert!(matches!(events[0].kind, UploadEventKind::Started));
    assert!(matches!(
        events[1].kind,
        UploadEventKind::Failed {
            kind: ErrorKind::Config
        }
    ));
    let text = format!("{events:?}");
    assert!(!text.contains("pw-very-secret"));
    assert!(!text.contains("alice"));
    assert!(!text.contains(&server.addr.to_string()));
    fs::remove_dir_all(root).ok();
}
