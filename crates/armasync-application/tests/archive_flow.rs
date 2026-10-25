// SPDX-License-Identifier: AGPL-3.0-only
//! Packed (`.zip`) entries through the sync use case (MIG-030E, ADR-0008): the archive is downloaded, unpacked into
//! staging, verified and only then committed. Repository and archives are generated here.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::cell::Cell;
use std::collections::BTreeMap;
use std::io::{Cursor, Write};

use armasync_adapters::archive::ZipUnpacker;
use armasync_adapters::{InMemoryStore, MemoryEventSink, Sha256Hasher};
use armasync_application::sync::{ApplyReport, SyncContext, apply, plan};
use armasync_core::{
    ARCHIVE_SUFFIX, CancelToken, ContentHash, Error, ErrorKind, FileEntry, HashAlgorithm, Hasher,
    Manifest, OperationId, PlanOptions, RelativePath, RepositorySource,
};
use zip::CompressionMethod;
use zip::write::SimpleFileOptions;

const PATH: &str = "@example/addons/example.pbo";

fn pattern(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i % 253) as u8 ^ (i / 11) as u8).collect()
}

fn zip_of(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, data) in entries {
        writer
            .start_file(
                *name,
                SimpleFileOptions::default().compression_method(CompressionMethod::Deflated),
            )
            .unwrap();
        writer.write_all(data).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

fn sha1(data: &[u8]) -> ContentHash {
    Sha256Hasher.hash(HashAlgorithm::Sha1, data)
}

/// A repository that publishes `PATH` packed as `PATH` + `.zip`. `read` counts the downloads.
struct PackedRepository {
    manifest: Manifest,
    files: BTreeMap<RelativePath, Vec<u8>>,
    reads: Cell<u32>,
}

impl PackedRepository {
    fn new(announced_hash: ContentHash, size: u64, archive: Vec<u8>, archive_size: u64) -> Self {
        let entry = FileEntry {
            path: RelativePath::parse(PATH).unwrap(),
            size,
            hash: announced_hash,
        };
        let mut archives = BTreeMap::new();
        archives.insert(RelativePath::parse(PATH).unwrap(), archive_size);
        let manifest = Manifest::new("7", vec![entry])
            .unwrap()
            .with_archives(archives)
            .unwrap();
        let mut files = BTreeMap::new();
        files.insert(
            RelativePath::parse(&format!("{PATH}{ARCHIVE_SUFFIX}")).unwrap(),
            archive,
        );
        Self {
            manifest,
            files,
            reads: Cell::new(0),
        }
    }
}

impl RepositorySource for PackedRepository {
    fn manifest(&self) -> Result<Manifest, Error> {
        Ok(self.manifest.clone())
    }

    fn read(&self, path: &RelativePath) -> Result<Vec<u8>, Error> {
        self.reads.set(self.reads.get() + 1);
        self.files
            .get(path)
            .cloned()
            .ok_or_else(|| Error::new(ErrorKind::Network, "not published"))
    }
}

fn run(repository: &PackedRepository, store: &mut InMemoryStore) -> Result<ApplyReport, Error> {
    let options = PlanOptions::default();
    let planned = plan(repository, &*store, &options)?;
    let mut sink = MemoryEventSink::default();
    let cancel = CancelToken::new();
    let mut context = SyncContext {
        source: repository,
        store,
        hasher: &Sha256Hasher,
        unpacker: &ZipUnpacker,
        sink: &mut sink,
        cancel: &cancel,
    };
    apply(OperationId::new(1), &planned, &options, &mut context)
}

fn content() -> Vec<u8> {
    pattern(150 * 1024)
}

#[test]
fn a_packed_file_is_downloaded_unpacked_verified_and_committed() {
    let data = content();
    let archive = zip_of(&[("example.pbo", &data)]);
    let repository = PackedRepository::new(
        sha1(&data),
        data.len() as u64,
        archive.clone(),
        archive.len() as u64,
    );
    let mut store = InMemoryStore::new();

    let report = run(&repository, &mut store).expect("the packed file installs");

    assert_eq!(report.downloaded, 1);
    assert_eq!(store.content(PATH), Some(data.as_slice()));
    assert_eq!(repository.reads.get(), 1, "the archive is downloaded once");
}

#[test]
fn an_up_to_date_packed_file_is_not_downloaded_again() {
    let data = content();
    let archive = zip_of(&[("example.pbo", &data)]);
    let repository = PackedRepository::new(
        sha1(&data),
        data.len() as u64,
        archive.clone(),
        archive.len() as u64,
    );
    let mut store = InMemoryStore::new();
    run(&repository, &mut store).unwrap();
    let before = repository.reads.get();

    let report = run(&repository, &mut store).expect("the second run succeeds");

    assert_eq!(report.downloaded + report.replaced, 0);
    assert_eq!(
        repository.reads.get(),
        before,
        "nothing is fetched when the file matches"
    );
}

#[test]
fn a_damaged_archive_leaves_the_existing_file_unchanged() {
    let data = content();
    let mut archive = zip_of(&[("example.pbo", &data)]);
    let middle = archive.len() / 2;
    archive[middle] ^= 0xff;
    let repository = PackedRepository::new(
        sha1(&data),
        data.len() as u64,
        archive.clone(),
        archive.len() as u64,
    );
    let mut store = InMemoryStore::new()
        .with_file(PATH, b"the old version")
        .unwrap();

    let error = run(&repository, &mut store).expect_err("the damaged archive is refused");

    assert!(
        matches!(error.kind(), ErrorKind::Integrity | ErrorKind::Format),
        "{error}"
    );
    assert_eq!(store.content(PATH), Some(&b"the old version"[..]));
}

#[test]
fn a_checksum_that_does_not_match_leaves_the_existing_file_unchanged() {
    let data = content();
    let archive = zip_of(&[("example.pbo", &data)]);
    // The manifest announces the hash of other content.
    let announced = sha1(b"something else entirely");
    let repository = PackedRepository::new(
        announced,
        data.len() as u64,
        archive.clone(),
        archive.len() as u64,
    );
    let mut store = InMemoryStore::new()
        .with_file(PATH, b"the old version")
        .unwrap();

    let error = run(&repository, &mut store).expect_err("the checksum must fail");

    assert_eq!(error.kind(), ErrorKind::Integrity, "{error}");
    assert_eq!(store.content(PATH), Some(&b"the old version"[..]));
}

#[test]
fn an_archive_of_another_size_than_announced_is_refused() {
    let data = content();
    let archive = zip_of(&[("example.pbo", &data)]);
    let real = archive.len() as u64;
    let mut store = InMemoryStore::new()
        .with_file(PATH, b"the old version")
        .unwrap();

    for announced in [real - 1, real + 10] {
        let repository =
            PackedRepository::new(sha1(&data), data.len() as u64, archive.clone(), announced);
        let error = run(&repository, &mut store).expect_err("the size must match");
        assert_eq!(
            error.kind(),
            ErrorKind::Integrity,
            "announced {announced}: {error}"
        );
        assert_eq!(store.content(PATH), Some(&b"the old version"[..]));
    }
}

#[test]
fn an_archive_with_two_entries_is_refused_and_nothing_is_written() {
    let data = content();
    let archive = zip_of(&[("example.pbo", &data), ("extra.pbo", b"extra")]);
    let repository = PackedRepository::new(
        sha1(&data),
        data.len() as u64,
        archive.clone(),
        archive.len() as u64,
    );
    let mut store = InMemoryStore::new();

    let error = run(&repository, &mut store).expect_err("two entries are refused");

    assert_eq!(error.kind(), ErrorKind::Format, "{error}");
    assert_eq!(store.content(PATH), None);
    assert!(
        store.paths().is_empty(),
        "no other file appears: {:?}",
        store.paths()
    );
}

#[test]
fn a_cancelled_run_does_not_touch_the_destination() {
    let data = content();
    let archive = zip_of(&[("example.pbo", &data)]);
    let repository = PackedRepository::new(
        sha1(&data),
        data.len() as u64,
        archive.clone(),
        archive.len() as u64,
    );
    let mut store = InMemoryStore::new()
        .with_file(PATH, b"the old version")
        .unwrap();
    let options = PlanOptions::default();
    let planned = plan(&repository, &store, &options).unwrap();
    let mut sink = MemoryEventSink::default();
    let cancel = CancelToken::new();
    cancel.cancel();
    let mut context = SyncContext {
        source: &repository,
        store: &mut store,
        hasher: &Sha256Hasher,
        unpacker: &ZipUnpacker,
        sink: &mut sink,
        cancel: &cancel,
    };

    let error = apply(OperationId::new(1), &planned, &options, &mut context)
        .expect_err("cancelled before the start");

    assert_eq!(error.kind(), ErrorKind::Cancelled);
    assert_eq!(store.content(PATH), Some(&b"the old version"[..]));
}
