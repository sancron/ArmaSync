// SPDX-License-Identifier: AGPL-3.0-only
//! Read-back verification before publication (MIG-042, step 3a; ADR-0012 addendum). Remote files that are corrupted,
//! shortened, missing or unreadable, and a cancellation during the read-back, must keep `current.json` on the
//! previous revision. Synthetic data and loopback server only.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;
use support::trust;

use std::cell::Cell;
use std::fs;
use std::path::PathBuf;

use armasync_adapters::Sha256Hasher;
use armasync_adapters::localfs::LocalDirectory;
use armasync_adapters::webdav::WebDavClient;
use armasync_application::revision::{self, PublishReport};
use armasync_core::{CancelToken, Error, ErrorKind, OpenedSource, RelativePath, RemoteTarget};
use support::{FaultKind, credentials, start};

fn tree(name: &str, files: &[(&str, &[u8])]) -> PathBuf {
    let root =
        std::env::temp_dir().join(format!("armasync-readback-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    for (relative, bytes) in files {
        let target = root.join(relative);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(target, bytes).unwrap();
    }
    root
}

fn publish(
    source: &LocalDirectory,
    target: &dyn RemoteTarget,
    cancel: &CancelToken,
) -> Result<PublishReport, Error> {
    revision::publish(source, target, &Sha256Hasher, cancel)
}

/// Counts the read-backs (`fetch` calls) and requests cancellation at the `cancel_at`-th one.
struct CancelAtFetch<'a> {
    inner: &'a WebDavClient,
    cancel: CancelToken,
    fetches: Cell<usize>,
    cancel_at: usize,
}

impl RemoteTarget for CancelAtFetch<'_> {
    fn exists(&self, path: &RelativePath) -> Result<bool, Error> {
        self.inner.exists(path)
    }

    fn make_folder(&self, path: &RelativePath) -> Result<(), Error> {
        self.inner.make_folder(path)
    }

    fn upload(
        &self,
        path: &RelativePath,
        source: OpenedSource,
        cancel: &CancelToken,
    ) -> Result<(), Error> {
        self.inner.upload(path, source, cancel)
    }

    fn fetch(&self, path: &RelativePath, limit: u64) -> Result<Option<Vec<u8>>, Error> {
        let count = self.fetches.get() + 1;
        self.fetches.set(count);
        if count == self.cancel_at {
            self.cancel.cancel();
        }
        self.inner.fetch(path, limit)
    }
}

/// Publishes the current content of `root` and returns the report and the pointer bytes of that publication.
fn published_first_revision(
    server: &support::Server,
    root: &PathBuf,
    target: &WebDavClient,
) -> (PublishReport, Vec<u8>) {
    let source = LocalDirectory::new(root).unwrap();
    let first = publish(&source, target, &CancelToken::new()).unwrap();
    let pointer = server.file("current.json").unwrap();
    (first, pointer)
}

#[test]
fn a_corrupted_remote_file_blocks_the_pointer_and_the_next_run_recovers() {
    let root = tree("corrupt", &[("x.txt", b"version one")]);
    let server = start(true, true, "");
    let target = WebDavClient::new(&server.https(), trust(&server), Some(credentials())).unwrap();
    let (_, old_pointer) = published_first_revision(&server, &root, &target);

    fs::write(root.join("x.txt"), b"version two, longer").unwrap();
    server.inject("files/x.txt", FaultKind::CorruptBody);
    let error = publish(
        &LocalDirectory::new(&root).unwrap(),
        &target,
        &CancelToken::new(),
    )
    .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Integrity, "{error}");
    assert!(error.message().contains("content"), "{error}");
    assert_eq!(server.file("current.json").unwrap(), old_pointer);

    server.clear_faults();
    let next = publish(
        &LocalDirectory::new(&root).unwrap(),
        &target,
        &CancelToken::new(),
    )
    .unwrap();
    assert!(next.pointer_changed);
    fs::remove_dir_all(root).ok();
}

#[test]
fn a_truncated_remote_file_is_reported_as_a_size_difference() {
    let root = tree("truncated", &[("x.txt", b"version one")]);
    let server = start(true, true, "");
    let target = WebDavClient::new(&server.https(), trust(&server), Some(credentials())).unwrap();
    let (_, old_pointer) = published_first_revision(&server, &root, &target);

    fs::write(root.join("x.txt"), b"version two, longer").unwrap();
    server.inject("files/x.txt", FaultKind::TruncateBody);
    let error = publish(
        &LocalDirectory::new(&root).unwrap(),
        &target,
        &CancelToken::new(),
    )
    .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Integrity, "{error}");
    assert!(error.message().contains("size"), "{error}");
    assert_eq!(server.file("current.json").unwrap(), old_pointer);
    fs::remove_dir_all(root).ok();
}

#[test]
fn a_missing_remote_file_blocks_the_pointer_and_no_manifest_is_written() {
    let root = tree("missing", &[("x.txt", b"version one")]);
    let server = start(true, true, "");
    let target = WebDavClient::new(&server.https(), trust(&server), Some(credentials())).unwrap();
    let (first, old_pointer) = published_first_revision(&server, &root, &target);

    fs::write(root.join("x.txt"), b"version two, longer").unwrap();
    server.inject("files/x.txt", FaultKind::DropBody);
    let error = publish(
        &LocalDirectory::new(&root).unwrap(),
        &target,
        &CancelToken::new(),
    )
    .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Integrity, "{error}");
    assert!(error.message().contains("missing"), "{error}");
    assert_eq!(server.file("current.json").unwrap(), old_pointer);
    // The manifest of the new, unverified revision was never written.
    let names = server.file_names();
    assert!(
        names
            .iter()
            .filter(|n| n.ends_with("manifest.json"))
            .all(|n| n.contains(&first.revision)),
        "{names:?}"
    );
    server.clear_faults();
    fs::remove_dir_all(root).ok();
}

#[test]
fn a_read_error_during_the_read_back_is_a_transport_error_and_keeps_the_pointer() {
    let root = tree("read-error", &[("x.txt", b"version one")]);
    let server = start(true, true, "");
    let target = WebDavClient::new(&server.https(), trust(&server), Some(credentials())).unwrap();
    let (_, old_pointer) = published_first_revision(&server, &root, &target);

    fs::write(root.join("x.txt"), b"version two, longer").unwrap();
    server.inject("files/x.txt", FaultKind::FailGet);
    let error = publish(
        &LocalDirectory::new(&root).unwrap(),
        &target,
        &CancelToken::new(),
    )
    .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Network, "{error}");
    assert_eq!(server.file("current.json").unwrap(), old_pointer);
    server.clear_faults();
    fs::remove_dir_all(root).ok();
}

#[test]
fn a_cancellation_during_the_read_back_keeps_the_pointer() {
    let root = tree(
        "cancel-readback",
        &[("a.txt", b"alpha"), ("b.txt", b"beta")],
    );
    let server = start(true, true, "");
    let target = WebDavClient::new(&server.https(), trust(&server), Some(credentials())).unwrap();
    let (_, old_pointer) = published_first_revision(&server, &root, &target);

    fs::write(root.join("a.txt"), b"alpha, changed").unwrap();
    let cancel = CancelToken::new();
    // Fetch 1 is the manifest check, fetch 2 the first file of the read-back: cancel there.
    let cancelling = CancelAtFetch {
        inner: &target,
        cancel: cancel.clone(),
        fetches: Cell::new(0),
        cancel_at: 2,
    };
    let error = publish(&LocalDirectory::new(&root).unwrap(), &cancelling, &cancel).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Cancelled, "{error}");
    assert_eq!(server.file("current.json").unwrap(), old_pointer);
    fs::remove_dir_all(root).ok();
}

#[test]
fn a_damaged_stored_file_of_a_complete_revision_blocks_the_pointer_switch() {
    let root = tree("damaged-complete", &[("x.txt", b"stable content")]);
    let server = start(true, true, "");
    let target = WebDavClient::new(&server.https(), trust(&server), Some(credentials())).unwrap();
    let (first, pointer) = published_first_revision(&server, &root, &target);

    // The revision is complete, but a stored file was altered afterwards.
    server.set_file(
        &format!("revisions/{}/files/x.txt", first.revision),
        b"tampered!!!!!!",
    );
    fs::write(root.join("x.txt"), b"stable content").unwrap();
    let error = publish(
        &LocalDirectory::new(&root).unwrap(),
        &target,
        &CancelToken::new(),
    )
    .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Integrity, "{error}");
    assert_eq!(server.file("current.json").unwrap(), pointer);
    fs::remove_dir_all(root).ok();
}
