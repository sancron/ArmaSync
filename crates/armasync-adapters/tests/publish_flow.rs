// SPDX-License-Identifier: AGPL-3.0-only
//! End-to-end tests of publication (MIG-042, step 2; ADR-0012): a real local source folder, the WebDAV client and a
//! loopback server. Success, cancellation, a failure while the pointer is written, and an existing revision with
//! different content. Synthetic data and credentials only.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;
use support::trust;

use std::fs;
use std::io::Read;
use std::path::PathBuf;

use armasync_adapters::Sha256Hasher;
use armasync_adapters::localfs::LocalDirectory;
use armasync_adapters::webdav::WebDavClient;
use armasync_application::revision::{self, PublishReport};
use armasync_core::{
    CancelToken, Error, ErrorKind, HashAlgorithm, Hasher, InventoryEntry, LocalSource,
    OpenedSource, RelativePath, SourceFile,
};
use support::{credentials, start};

fn tree(name: &str, files: &[(&str, &[u8])]) -> PathBuf {
    let root = std::env::temp_dir().join(format!("armasync-publish-{}-{name}", std::process::id()));
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

fn publish(
    source: &dyn LocalSource,
    target: &WebDavClient,
    cancel: &CancelToken,
) -> Result<PublishReport, Error> {
    revision::publish(source, target, &Sha256Hasher, cancel)
}

/// The `manifest_sha256` value named in a pointer.
fn pointer_manifest_hash(pointer: &[u8]) -> String {
    let text = String::from_utf8(pointer.to_vec()).unwrap();
    let key = "\"manifest_sha256\":\"";
    let start = text.find(key).unwrap() + key.len();
    text[start..start + 64].to_owned()
}

fn pointer_revision(pointer: &[u8]) -> String {
    let text = String::from_utf8(pointer.to_vec()).unwrap();
    let key = "\"revision\":\"";
    let start = text.find(key).unwrap() + key.len();
    let end = start + text[start..].find('"').unwrap();
    text[start..end].to_owned()
}

/// Wraps a source and requests cancellation once `after` bytes of a file were read.
struct CancelAfter<'a> {
    inner: &'a LocalDirectory,
    cancel: CancelToken,
    after: u64,
}

impl LocalSource for CancelAfter<'_> {
    fn inventory(&self, cancel: &CancelToken) -> Result<Vec<InventoryEntry>, Error> {
        self.inner.inventory(cancel)
    }

    fn files(&self, cancel: &CancelToken) -> Result<Vec<SourceFile>, Error> {
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
fn a_publication_writes_files_manifest_and_pointer_and_the_pointer_verifies() {
    let root = tree(
        "success",
        &[("top.txt", b"top"), ("a/one.bin", &bytes_of(50_000, 1))],
    );
    let source = LocalDirectory::new(&root).unwrap();
    let server = start(true, true, "");
    let target = WebDavClient::new(&server.https(), trust(&server), Some(credentials())).unwrap();
    let report = publish(&source, &target, &CancelToken::new()).unwrap();

    assert!(!report.revision_was_complete);
    assert_eq!(report.files_uploaded, 2);
    assert!(report.pointer_changed);
    let pointer = server.file("current.json").expect("pointer published");
    assert_eq!(pointer_revision(&pointer), report.revision);
    let manifest = server
        .file(&format!("revisions/{}/manifest.json", report.revision))
        .expect("manifest published");
    assert_eq!(
        pointer_manifest_hash(&pointer),
        Sha256Hasher.hash(HashAlgorithm::Sha256, &manifest).hex()
    );
    assert_eq!(
        server
            .file(&format!("revisions/{}/files/a/one.bin", report.revision))
            .unwrap(),
        bytes_of(50_000, 1)
    );
    assert!(
        server
            .file_names()
            .iter()
            .all(|name| !name.starts_with("history/")),
        "a first publication has no previous pointer to keep"
    );
    fs::remove_dir_all(root).ok();
}

#[test]
fn publishing_the_same_content_again_uploads_nothing_and_keeps_the_pointer() {
    let root = tree("same", &[("x.txt", b"same content")]);
    let source = LocalDirectory::new(&root).unwrap();
    let server = start(true, true, "");
    let target = WebDavClient::new(&server.https(), trust(&server), Some(credentials())).unwrap();
    let first = publish(&source, &target, &CancelToken::new()).unwrap();
    let puts_after_first = server.puts();
    let before = server.file("current.json").unwrap();

    let second = publish(&source, &target, &CancelToken::new()).unwrap();
    assert_eq!(second.revision, first.revision);
    assert!(second.revision_was_complete);
    assert!(!second.pointer_changed);
    assert_eq!(second.files_uploaded, 0);
    assert_eq!(server.puts(), puts_after_first, "nothing may be written");
    assert_eq!(server.file("current.json").unwrap(), before);
    fs::remove_dir_all(root).ok();
}

#[test]
fn changed_content_publishes_a_new_revision_keeps_the_old_one_and_archives_the_old_pointer() {
    let root = tree("changed", &[("x.txt", b"version one")]);
    let source = LocalDirectory::new(&root).unwrap();
    let server = start(true, true, "");
    let target = WebDavClient::new(&server.https(), trust(&server), Some(credentials())).unwrap();
    let first = publish(&source, &target, &CancelToken::new()).unwrap();
    let old_pointer = server.file("current.json").unwrap();

    fs::write(root.join("x.txt"), b"version two, longer").unwrap();
    let second = publish(&source, &target, &CancelToken::new()).unwrap();
    assert_ne!(second.revision, first.revision);
    assert!(second.pointer_changed);
    assert_eq!(
        pointer_revision(&server.file("current.json").unwrap()),
        second.revision
    );

    // The old revision is still there, complete and unchanged.
    assert_eq!(
        server
            .file(&format!("revisions/{}/files/x.txt", first.revision))
            .unwrap(),
        b"version one"
    );
    assert!(
        server
            .file(&format!("revisions/{}/manifest.json", first.revision))
            .is_some()
    );
    // The old pointer was kept in history/ under the hash of its bytes.
    let archived = Sha256Hasher.hash(HashAlgorithm::Sha256, &old_pointer);
    assert_eq!(
        server
            .file(&format!("history/{}.json", archived.hex()))
            .unwrap(),
        old_pointer
    );
    fs::remove_dir_all(root).ok();
}

#[test]
fn cancelling_while_the_files_upload_keeps_the_previous_pointer_and_the_next_run_completes() {
    let root = tree("cancel", &[("small.txt", b"small")]);
    let source = LocalDirectory::new(&root).unwrap();
    let server = start(true, true, "");
    let target = WebDavClient::new(&server.https(), trust(&server), Some(credentials())).unwrap();
    let first = publish(&source, &target, &CancelToken::new()).unwrap();
    let old_pointer = server.file("current.json").unwrap();

    fs::write(root.join("big.bin"), bytes_of(2 * 1024 * 1024, 5)).unwrap();
    let cancel = CancelToken::new();
    let cancelling = CancelAfter {
        inner: &source,
        cancel: cancel.clone(),
        after: 64 * 1024,
    };
    let error = publish(&cancelling, &target, &cancel).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Cancelled, "{error}");
    assert_eq!(server.file("current.json").unwrap(), old_pointer);
    assert!(
        server
            .file_names()
            .iter()
            .all(|name| !name.ends_with("manifest.json") || name.contains(&first.revision)),
        "an interrupted revision must not have a manifest"
    );

    let next = publish(&source, &target, &CancelToken::new()).unwrap();
    assert!(next.pointer_changed);
    assert_eq!(
        pointer_revision(&server.file("current.json").unwrap()),
        next.revision
    );
    fs::remove_dir_all(root).ok();
}

#[test]
fn a_failure_while_writing_the_pointer_keeps_the_previous_one_and_is_reported() {
    let root = tree("pointer-failure", &[("x.txt", b"first")]);
    let source = LocalDirectory::new(&root).unwrap();
    let server = start(true, true, "");
    let target = WebDavClient::new(&server.https(), trust(&server), Some(credentials())).unwrap();
    publish(&source, &target, &CancelToken::new()).unwrap();
    let old_pointer = server.file("current.json").unwrap();

    fs::write(root.join("x.txt"), b"second").unwrap();
    server.fail_puts_to("current.json");
    let error = publish(&source, &target, &CancelToken::new()).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Network, "{error}");
    assert_eq!(server.file("current.json").unwrap(), old_pointer);

    // The previous pointer was archived before the failed write; the next run succeeds.
    server.clear_failures();
    let next = publish(&source, &target, &CancelToken::new()).unwrap();
    assert!(next.pointer_changed);
    assert_ne!(server.file("current.json").unwrap(), old_pointer);
    fs::remove_dir_all(root).ok();
}

#[test]
fn an_existing_revision_with_different_content_is_refused_and_nothing_changes() {
    let root = tree("conflict", &[("x.txt", b"content")]);
    let source = LocalDirectory::new(&root).unwrap();
    let server = start(true, true, "");
    let target = WebDavClient::new(&server.https(), trust(&server), Some(credentials())).unwrap();
    let first = publish(&source, &target, &CancelToken::new()).unwrap();
    let pointer = server.file("current.json").unwrap();

    // Someone replaces the manifest of the revision that is about to be published again.
    let manifest_name = format!("revisions/{}/manifest.json", first.revision);
    server.set_file(&manifest_name, b"{\"tampered\":true}");
    let error = publish(&source, &target, &CancelToken::new()).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Integrity, "{error}");
    assert_eq!(server.file("current.json").unwrap(), pointer);
    assert_eq!(
        server.file(&manifest_name).unwrap(),
        b"{\"tampered\":true}",
        "a conflicting revision is never overwritten"
    );
    fs::remove_dir_all(root).ok();
}
