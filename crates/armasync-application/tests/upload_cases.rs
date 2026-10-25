// SPDX-License-Identifier: AGPL-3.0-only
//! Plan and apply of the upload use case against in-memory fakes of the source and the target (ADR-0011). These tests
//! pin the ordering, the change detection, the refusal rules and the cancellation behaviour of the use case itself.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::{BTreeMap, BTreeSet};
use std::io::{Cursor, Read};
use std::sync::Mutex;

use armasync_adapters::Sha256Hasher;
use armasync_application::upload::{self, UploadStep};
use armasync_core::{
    CancelToken, Error, ErrorKind, HashAlgorithm, Hasher, LocalSource, OpenedSource, RelativePath,
    RemoteTarget, SourceFile,
};

fn path(text: &str) -> RelativePath {
    RelativePath::parse(text).unwrap()
}

/// A source folder held in memory.
#[derive(Default)]
struct FakeSource {
    files: BTreeMap<String, Vec<u8>>,
}

impl FakeSource {
    fn with(files: &[(&str, &[u8])]) -> Self {
        Self {
            files: files
                .iter()
                .map(|(p, b)| ((*p).to_owned(), b.to_vec()))
                .collect(),
        }
    }
}

impl LocalSource for FakeSource {
    fn files(&self, cancel: &CancelToken) -> Result<Vec<SourceFile>, Error> {
        if cancel.is_cancelled() {
            return Err(Error::new(ErrorKind::Cancelled, "cancelled"));
        }
        Ok(self
            .files
            .iter()
            .map(|(p, bytes)| SourceFile {
                path: path(p),
                size: bytes.len() as u64,
                hash: Sha256Hasher.hash(HashAlgorithm::Sha256, bytes),
            })
            .collect())
    }

    fn open(&self, path: &RelativePath) -> Result<OpenedSource, Error> {
        let bytes = self
            .files
            .get(path.as_str())
            .ok_or_else(|| Error::new(ErrorKind::FileSystem, "the file is missing"))?;
        Ok(OpenedSource {
            size: bytes.len() as u64,
            content: Box::new(Cursor::new(bytes.clone())),
        })
    }
}

/// A target folder held in memory. It records every call, can refuse one upload, and can request cancellation while
/// an upload is in progress.
#[derive(Default)]
struct FakeTarget {
    existing: Mutex<BTreeSet<String>>,
    calls: Mutex<Vec<String>>,
    stored: Mutex<BTreeMap<String, Vec<u8>>>,
    fail_upload: Option<String>,
    cancel_on_upload: Option<CancelToken>,
}

impl FakeTarget {
    fn with_existing(paths: &[&str]) -> Self {
        Self {
            existing: Mutex::new(paths.iter().map(|p| (*p).to_owned()).collect()),
            ..Self::default()
        }
    }

    /// Calls other than the existence checks, in order.
    fn writes(&self) -> Vec<String> {
        self.calls
            .lock()
            .unwrap()
            .iter()
            .filter(|c| !c.starts_with("exists"))
            .cloned()
            .collect()
    }
}

impl RemoteTarget for FakeTarget {
    fn fetch(&self, path: &RelativePath, _limit: u64) -> Result<Option<Vec<u8>>, Error> {
        Ok(self.stored.lock().unwrap().get(path.as_str()).cloned())
    }

    fn exists(&self, path: &RelativePath) -> Result<bool, Error> {
        self.calls.lock().unwrap().push(format!("exists {path}"));
        Ok(self.existing.lock().unwrap().contains(path.as_str()))
    }

    fn make_folder(&self, path: &RelativePath) -> Result<(), Error> {
        self.calls.lock().unwrap().push(format!("mkdir {path}"));
        self.existing
            .lock()
            .unwrap()
            .insert(path.as_str().to_owned());
        Ok(())
    }

    fn upload(
        &self,
        path: &RelativePath,
        mut source: OpenedSource,
        _cancel: &CancelToken,
    ) -> Result<(), Error> {
        self.calls.lock().unwrap().push(format!("put {path}"));
        if self.fail_upload.as_deref() == Some(path.as_str()) {
            return Err(Error::new(
                ErrorKind::Network,
                "the server stopped answering",
            ));
        }
        if let Some(token) = &self.cancel_on_upload {
            token.cancel();
            return Err(Error::new(ErrorKind::Network, "the stream was cut"));
        }
        let mut bytes = Vec::new();
        source.content.read_to_end(&mut bytes).unwrap();
        self.stored
            .lock()
            .unwrap()
            .insert(path.as_str().to_owned(), bytes);
        self.existing
            .lock()
            .unwrap()
            .insert(path.as_str().to_owned());
        Ok(())
    }
}

#[test]
fn the_plan_orders_folders_by_depth_then_files_by_path_and_flags_replacements() {
    let source = FakeSource::with(&[("z.txt", b"1"), ("a/b/c.txt", b"22"), ("a/d.txt", b"333")]);
    let target = FakeTarget::with_existing(&["a/d.txt"]);
    let plan = upload::plan(&source, &target, &CancelToken::new()).unwrap();
    assert_eq!(
        plan.steps,
        [
            UploadStep::MakeFolder { path: path("a") },
            UploadStep::MakeFolder { path: path("a/b") },
            UploadStep::Upload {
                path: path("a/b/c.txt"),
                size: 2,
                replace: false,
            },
            UploadStep::Upload {
                path: path("a/d.txt"),
                size: 3,
                replace: true,
            },
            UploadStep::Upload {
                path: path("z.txt"),
                size: 1,
                replace: false,
            },
        ]
    );
    assert_eq!(
        (plan.folders, plan.files, plan.replacements, plan.bytes),
        (2, 3, 1, 6)
    );
    assert!(target.writes().is_empty(), "planning must not write");
}

#[test]
fn apply_runs_the_steps_in_order_and_reports_what_it_did() {
    let source = FakeSource::with(&[("z.txt", b"1"), ("a/b/c.txt", b"22"), ("a/d.txt", b"333")]);
    let target = FakeTarget::with_existing(&["a/d.txt"]);
    let cancel = CancelToken::new();
    let plan = upload::plan(&source, &target, &cancel).unwrap();
    let report = upload::apply(&plan, &source, &target, &cancel).unwrap();
    assert_eq!(report.folders_created, 2);
    assert_eq!(report.files_created, 2);
    assert_eq!(report.files_replaced, 1);
    assert_eq!(report.bytes, 6);
    assert_eq!(
        target.writes(),
        [
            "mkdir a",
            "mkdir a/b",
            "put a/b/c.txt",
            "put a/d.txt",
            "put z.txt"
        ]
    );
    assert_eq!(
        target.stored.lock().unwrap().get("a/d.txt").unwrap(),
        b"333"
    );
}

#[test]
fn a_file_whose_size_changed_after_the_plan_is_refused_before_anything_is_sent() {
    let planned = FakeSource::with(&[("x.txt", b"abc")]);
    let changed = FakeSource::with(&[("x.txt", b"abcdef")]);
    let target = FakeTarget::default();
    let cancel = CancelToken::new();
    let plan = upload::plan(&planned, &target, &cancel).unwrap();
    let error = upload::apply(&plan, &changed, &target, &cancel).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Integrity, "{error}");
    assert!(target.writes().is_empty());
}

#[test]
fn cancelling_before_apply_sends_nothing() {
    let source = FakeSource::with(&[("a/x.txt", b"x")]);
    let target = FakeTarget::default();
    let cancel = CancelToken::new();
    let plan = upload::plan(&source, &target, &cancel).unwrap();
    cancel.cancel();
    let error = upload::apply(&plan, &source, &target, &cancel).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Cancelled);
    assert!(target.writes().is_empty());
}

#[test]
fn a_remote_failure_stops_the_run_and_keeps_the_completed_steps() {
    let source = FakeSource::with(&[("z.txt", b"1"), ("a/b/c.txt", b"22"), ("a/d.txt", b"333")]);
    let target = FakeTarget {
        fail_upload: Some("a/d.txt".to_owned()),
        ..FakeTarget::default()
    };
    let cancel = CancelToken::new();
    let plan = upload::plan(&source, &target, &cancel).unwrap();
    let error = upload::apply(&plan, &source, &target, &cancel).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Network);
    assert!(target.stored.lock().unwrap().contains_key("a/b/c.txt"));
    assert!(!target.stored.lock().unwrap().contains_key("z.txt"));
    assert!(target.writes().iter().all(|c| c != "put z.txt"));
}

#[test]
fn a_cancellation_during_an_upload_is_reported_as_cancelled() {
    let source = FakeSource::with(&[("x.txt", b"x")]);
    let cancel = CancelToken::new();
    let target = FakeTarget {
        cancel_on_upload: Some(cancel.clone()),
        ..FakeTarget::default()
    };
    let plan = upload::plan(&source, &target, &cancel).unwrap();
    let error = upload::apply(&plan, &source, &target, &cancel).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Cancelled, "{error}");
}

#[test]
fn an_empty_source_has_an_empty_plan_and_an_empty_report() {
    let source = FakeSource::default();
    let target = FakeTarget::default();
    let cancel = CancelToken::new();
    let plan = upload::plan(&source, &target, &cancel).unwrap();
    assert!(plan.steps.is_empty());
    let report = upload::apply(&plan, &source, &target, &cancel).unwrap();
    assert_eq!(report, upload::UploadReport::default());
}
