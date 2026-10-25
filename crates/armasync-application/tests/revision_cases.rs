// SPDX-License-Identifier: AGPL-3.0-only
//! Revision identifiers, manifests, pointers and the write order of the publication, against an in-memory target
//! (ADR-0012). The loopback tests in the adapter crate cover the real WebDAV behaviour.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::{BTreeMap, BTreeSet};
use std::io::{Cursor, Read};
use std::sync::Mutex;

use armasync_adapters::Sha256Hasher;
use armasync_application::revision::{self, Revision};
use armasync_core::{
    CancelToken, Error, ErrorKind, HashAlgorithm, Hasher, InventoryEntry, LocalSource,
    OpenedSource, RelativePath, RemoteTarget, SourceFile,
};

fn path(text: &str) -> RelativePath {
    RelativePath::parse(text).unwrap()
}

fn file(name: &str, bytes: &[u8]) -> SourceFile {
    SourceFile {
        path: path(name),
        size: bytes.len() as u64,
        hash: Sha256Hasher.hash(HashAlgorithm::Sha256, bytes),
    }
}

#[derive(Default)]
struct FakeSource {
    contents: BTreeMap<String, Vec<u8>>,
}

impl LocalSource for FakeSource {
    fn inventory(&self, _cancel: &CancelToken) -> Result<Vec<InventoryEntry>, Error> {
        Ok(self
            .contents
            .iter()
            .map(|(name, bytes)| InventoryEntry {
                path: path(name),
                size: bytes.len() as u64,
            })
            .collect())
    }

    fn files(&self, _cancel: &CancelToken) -> Result<Vec<SourceFile>, Error> {
        Ok(self
            .contents
            .iter()
            .map(|(name, bytes)| file(name, bytes))
            .collect())
    }

    fn open(&self, path: &RelativePath) -> Result<OpenedSource, Error> {
        let bytes = self
            .contents
            .get(path.as_str())
            .ok_or_else(|| Error::new(ErrorKind::FileSystem, "missing"))?;
        Ok(OpenedSource {
            size: bytes.len() as u64,
            content: Box::new(Cursor::new(bytes.clone())),
        })
    }
}

/// A target folder in memory that records the order of its writes.
#[derive(Default)]
struct MemoryTarget {
    files: Mutex<BTreeMap<String, Vec<u8>>>,
    folders: Mutex<BTreeSet<String>>,
    writes: Mutex<Vec<String>>,
}

impl RemoteTarget for MemoryTarget {
    fn exists(&self, path: &RelativePath) -> Result<bool, Error> {
        Ok(self.folders.lock().unwrap().contains(path.as_str())
            || self.files.lock().unwrap().contains_key(path.as_str()))
    }

    fn make_folder(&self, path: &RelativePath) -> Result<(), Error> {
        self.folders
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
        let mut bytes = Vec::new();
        source.content.read_to_end(&mut bytes).unwrap();
        self.writes.lock().unwrap().push(path.as_str().to_owned());
        self.files
            .lock()
            .unwrap()
            .insert(path.as_str().to_owned(), bytes);
        Ok(())
    }

    fn fetch(&self, path: &RelativePath, _limit: u64) -> Result<Option<Vec<u8>>, Error> {
        Ok(self.files.lock().unwrap().get(path.as_str()).cloned())
    }
}

#[test]
fn the_revision_identifier_depends_on_the_content_only() {
    let a = file("a.txt", b"alpha");
    let b = file("b/b.txt", b"beta");
    let forward = Revision::build(&[a.clone(), b.clone()], &Sha256Hasher).unwrap();
    let backward = Revision::build(&[b.clone(), a.clone()], &Sha256Hasher).unwrap();
    assert_eq!(forward.id, backward.id);
    assert_eq!(forward.manifest, backward.manifest);
    assert!(forward.id.starts_with("r-") && forward.id.len() == 2 + 32);

    let changed = Revision::build(&[a, file("b/b.txt", b"beta!")], &Sha256Hasher).unwrap();
    assert_ne!(changed.id, forward.id);
}

#[test]
fn the_manifest_is_sorted_and_escapes_special_characters_in_paths() {
    // Backslashes are refused by RelativePath; quotes are the special character that reaches the JSON writer.
    let weird = file("q\"uote.txt", b"x");
    let plain = file("a.txt", b"y");
    let revision = Revision::build(&[weird, plain], &Sha256Hasher).unwrap();
    let text = String::from_utf8(revision.manifest.clone()).unwrap();
    assert!(
        text.starts_with("{\"schema\":\"armasync.revision\",\"version\":1"),
        "{text}"
    );
    let first = text.find("a.txt").unwrap();
    let second = text.find("q\\\"uote.txt").unwrap();
    assert!(first < second, "files are listed by path: {text}");
    assert!(text.contains("\"path\":\"q\\\"uote.txt\""), "{text}");
    assert!(text.ends_with("]}"));
}

#[test]
fn the_pointer_names_the_revision_and_the_hash_of_its_manifest() {
    let revision = Revision::build(&[file("a.txt", b"x")], &Sha256Hasher).unwrap();
    let pointer = String::from_utf8(revision.current_pointer()).unwrap();
    assert!(
        pointer.contains(&format!("\"revision\":\"{}\"", revision.id)),
        "{pointer}"
    );
    let expected = Sha256Hasher
        .hash(HashAlgorithm::Sha256, &revision.manifest)
        .hex()
        .to_owned();
    assert_eq!(revision.manifest_sha256, expected);
    assert!(
        pointer.contains(&format!("\"manifest_sha256\":\"{expected}\"")),
        "{pointer}"
    );
}

#[test]
fn files_are_written_before_the_manifest_and_the_manifest_before_the_pointer() {
    let source = FakeSource {
        contents: BTreeMap::from([
            ("a/one.txt".to_owned(), b"one".to_vec()),
            ("two.txt".to_owned(), b"two".to_vec()),
        ]),
    };
    let target = MemoryTarget::default();
    let report = revision::publish(&source, &target, &Sha256Hasher, &CancelToken::new()).unwrap();
    assert_eq!(report.files_uploaded, 2);
    assert!(report.pointer_changed);

    let writes = target.writes.lock().unwrap().clone();
    let manifest = format!("revisions/{}/manifest.json", report.revision);
    let position = |name: &str| writes.iter().position(|w| w == name).unwrap();
    assert!(
        position(&format!("revisions/{}/files/a/one.txt", report.revision)) < position(&manifest)
    );
    assert!(
        position(&format!("revisions/{}/files/two.txt", report.revision)) < position(&manifest)
    );
    assert_eq!(
        writes.last().map(String::as_str),
        Some("current.json"),
        "{writes:?}"
    );
}

#[test]
fn a_second_publication_of_the_same_content_writes_nothing() {
    let source = FakeSource {
        contents: BTreeMap::from([("x.txt".to_owned(), b"x".to_vec())]),
    };
    let target = MemoryTarget::default();
    let cancel = CancelToken::new();
    revision::publish(&source, &target, &Sha256Hasher, &cancel).unwrap();
    let writes_before = target.writes.lock().unwrap().len();
    let report = revision::publish(&source, &target, &Sha256Hasher, &cancel).unwrap();
    assert!(report.revision_was_complete);
    assert!(!report.pointer_changed);
    assert_eq!(target.writes.lock().unwrap().len(), writes_before);
}
