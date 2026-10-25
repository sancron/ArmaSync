// SPDX-License-Identifier: AGPL-3.0-only
//! End-to-end tests of the upload use case (MIG-042, step 1; ADR-0011): a real local source folder and the WebDAV
//! client against a loopback server. Synthetic data and credentials only.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;
use support::trust;

use std::fs;
use std::io::Read;
use std::path::PathBuf;

use armasync_adapters::localfs::LocalDirectory;
use armasync_adapters::webdav::WebDavClient;
use armasync_application::upload::{self, UploadStep};
use armasync_core::{CancelToken, Error, ErrorKind, LocalSource, OpenedSource, RelativePath};
use support::{credentials, path, start};

/// A fresh source folder in the system temp directory with the given files.
fn tree(name: &str, files: &[(&str, &[u8])]) -> PathBuf {
    let root = std::env::temp_dir().join(format!("armasync-upload-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    for (relative, bytes) in files {
        let target = root.join(relative);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(target, bytes).unwrap();
    }
    root
}

fn bytes_of(size: usize, seed: u8) -> Vec<u8> {
    (0..size)
        .map(|i| (i.wrapping_mul(7).wrapping_add(seed as usize) % 251) as u8)
        .collect()
}

fn client(server: &support::Server) -> WebDavClient {
    WebDavClient::new(&server.https(), trust(server), Some(credentials())).unwrap()
}

/// Wraps a source and requests cancellation once `after` bytes of one file were read: a cancellation during the
/// transfer, not before it.
struct CancelAfter<'a> {
    inner: &'a LocalDirectory,
    cancel: CancelToken,
    after: u64,
}

impl LocalSource for CancelAfter<'_> {
    fn inventory(&self, cancel: &CancelToken) -> Result<Vec<armasync_core::InventoryEntry>, Error> {
        self.inner.inventory(cancel)
    }

    fn files(&self, cancel: &CancelToken) -> Result<Vec<armasync_core::SourceFile>, Error> {
        self.inner.files(cancel)
    }

    fn open(&self, path: &RelativePath) -> Result<OpenedSource, Error> {
        let opened = self.inner.open(path)?;
        Ok(OpenedSource {
            size: opened.size,
            content: Box::new(Tripwire {
                inner: opened.content,
                cancel: self.cancel.clone(),
                read: 0,
                after: self.after,
            }),
        })
    }
}

struct Tripwire {
    inner: Box<dyn Read + Send>,
    cancel: CancelToken,
    read: u64,
    after: u64,
}

impl Read for Tripwire {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self.inner.read(buf)?;
        self.read += n as u64;
        if self.read >= self.after {
            self.cancel.cancel();
        }
        Ok(n)
    }
}

#[test]
fn a_local_tree_is_uploaded_and_a_second_run_replaces_it() {
    let one = bytes_of(100_000, 1);
    let root = tree(
        "tree",
        &[
            ("top.txt", b"top"),
            ("a/one.bin", &one),
            ("a/b/two.txt", b"two!"),
        ],
    );
    let source = LocalDirectory::new(&root).unwrap();
    let server = start(true, true, "");
    let target = client(&server);
    let cancel = CancelToken::new();

    let first = upload::plan(&source, &target, &cancel).unwrap();
    assert_eq!(first.folders, 2, "{first:?}");
    assert_eq!(first.files, 3);
    assert_eq!(first.replacements, 0);
    assert_eq!(first.bytes, 3 + 100_000 + 4);
    assert_eq!(
        first.steps[..2],
        [
            UploadStep::MakeFolder { path: path("a") },
            UploadStep::MakeFolder { path: path("a/b") },
        ]
    );
    let report = upload::apply(&first, &source, &target, &cancel).unwrap();
    assert_eq!(report.folders_created, 2);
    assert_eq!(report.files_created, 3);
    assert_eq!(report.files_replaced, 0);
    assert_eq!(server.file("a/one.bin").unwrap(), one);
    assert_eq!(server.file("a/b/two.txt").unwrap(), b"two!");
    assert_eq!(server.file("top.txt").unwrap(), b"top");

    // The second run finds everything in place: no folders, every file is a replacement.
    let second = upload::plan(&source, &target, &cancel).unwrap();
    assert_eq!(second.folders, 0, "{second:?}");
    assert_eq!(second.replacements, 3);
    let again = upload::apply(&second, &source, &target, &cancel).unwrap();
    assert_eq!(again.files_replaced, 3);
    assert_eq!(again.files_created, 0);
    fs::remove_dir_all(root).ok();
}

#[test]
fn planning_changes_nothing_on_the_target() {
    let root = tree("plan-only", &[("docs/x.txt", b"x")]);
    let source = LocalDirectory::new(&root).unwrap();
    let server = start(true, true, "");
    let target = client(&server);
    let plan = upload::plan(&source, &target, &CancelToken::new()).unwrap();
    assert_eq!(plan.folders, 1);
    let methods: Vec<String> = server.seen().into_iter().map(|s| s.method).collect();
    assert!(
        methods.iter().all(|m| m == "PROPFIND" || m == "OPTIONS"),
        "{methods:?}"
    );
    assert!(server.file("docs/x.txt").is_none());
    fs::remove_dir_all(root).ok();
}

#[test]
fn a_file_changed_after_the_plan_is_refused_before_it_is_sent() {
    let root = tree("changed", &[("notes.txt", b"short")]);
    let source = LocalDirectory::new(&root).unwrap();
    let server = start(true, true, "");
    let target = client(&server);
    let cancel = CancelToken::new();
    let plan = upload::plan(&source, &target, &cancel).unwrap();
    fs::write(root.join("notes.txt"), b"much longer content than planned").unwrap();
    let error = upload::apply(&plan, &source, &target, &cancel).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Integrity, "{error}");
    assert!(server.seen().iter().all(|s| s.method != "PUT"));
    fs::remove_dir_all(root).ok();
}

#[test]
fn cancelling_before_the_apply_sends_nothing() {
    let root = tree("cancel-before", &[("a/x.txt", b"x")]);
    let source = LocalDirectory::new(&root).unwrap();
    let server = start(true, true, "");
    let target = client(&server);
    let cancel = CancelToken::new();
    let plan = upload::plan(&source, &target, &cancel).unwrap();
    cancel.cancel();
    let error = upload::apply(&plan, &source, &target, &cancel).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Cancelled);
    assert!(
        server
            .seen()
            .iter()
            .all(|s| s.method != "MKCOL" && s.method != "PUT")
    );
    fs::remove_dir_all(root).ok();
}

#[test]
fn cancelling_during_a_transfer_stops_it_and_stores_no_partial_file() {
    let big = bytes_of(2 * 1024 * 1024, 9);
    let root = tree("cancel-during", &[("big.bin", &big)]);
    let source = LocalDirectory::new(&root).unwrap();
    let server = start(true, true, "");
    let target = client(&server);
    let plan = upload::plan(&source, &target, &CancelToken::new()).unwrap();
    let cancel = CancelToken::new();
    let cancelling = CancelAfter {
        inner: &source,
        cancel: cancel.clone(),
        after: 64 * 1024,
    };
    let error = upload::apply(&plan, &cancelling, &target, &cancel).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Cancelled, "{error}");
    assert!(
        server.file("big.bin").is_none(),
        "a partial upload must not appear as a file"
    );
    fs::remove_dir_all(root).ok();
}

#[test]
fn a_redirect_during_the_upload_is_not_followed_and_credentials_stay_on_the_origin() {
    let root = tree("redirect", &[("redirect/same/docs/x.txt", b"data")]);
    let source = LocalDirectory::new(&root).unwrap();
    let server = start(true, true, "");
    let target = client(&server);
    let cancel = CancelToken::new();
    let plan = upload::plan(&source, &target, &cancel).unwrap();
    let error = upload::apply(&plan, &source, &target, &cancel).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Network, "{error}");
    assert!(error.message().contains("not followed"), "{error}");
    let seen = server.seen();
    assert!(seen.iter().all(|s| s.method != "PUT"), "{seen:?}");
    // Reads may follow the same-origin redirect (with credentials); writes must never reach the redirect target.
    assert!(
        seen.iter()
            .filter(|s| s.method == "MKCOL" || s.method == "PUT")
            .all(|s| s.target != "/dav/docs" && s.target != "/dav/docs/x.txt"),
        "{seen:?}"
    );
    assert!(
        seen.iter()
            .filter(|s| s.method == "MKCOL")
            .all(|s| s.authorization.as_deref() == Some(support::EXPECTED_AUTH))
    );
    fs::remove_dir_all(root).ok();
}

#[test]
fn opening_a_missing_source_file_is_a_file_system_error() {
    let root = tree("missing", &[("here.txt", b"x")]);
    let source = LocalDirectory::new(&root).unwrap();
    let missing = RelativePath::parse("gone.txt").unwrap();
    let error = source.open(&missing).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::FileSystem, "{error}");
    fs::remove_dir_all(root).ok();
}

#[test]
fn the_source_root_is_not_modified_by_an_upload() {
    let root = tree("untouched", &[("keep/a.txt", b"alpha")]);
    let source = LocalDirectory::new(&root).unwrap();
    let server = start(true, true, "");
    let target = client(&server);
    let cancel = CancelToken::new();
    let plan = upload::plan(&source, &target, &cancel).unwrap();
    upload::apply(&plan, &source, &target, &cancel).unwrap();
    assert_eq!(fs::read(root.join("keep/a.txt")).unwrap(), b"alpha");
    let mut names: Vec<String> = fs::read_dir(root.join("keep"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(names, ["a.txt"]);
    assert!(target.exists(&path("keep")).unwrap());
    fs::remove_dir_all(root).ok();
}
