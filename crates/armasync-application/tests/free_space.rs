// SPDX-License-Identifier: AGPL-3.0-only
//! The advance check of the free space (MIG-032B): enough, not enough (before any change and before any network
//! transfer), unknown, overflow, and the credit for replaced files and archives. Repository and archives are generated.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::cell::Cell;
use std::collections::BTreeMap;
use std::io::{Cursor, Write};

use armasync_adapters::archive::ZipUnpacker;
use armasync_adapters::{InMemoryStore, MemoryEventSink, Sha256Hasher};
use armasync_application::sync::{FREE_SPACE_RESERVE, SyncContext, apply, plan};
use armasync_core::{
    ARCHIVE_SUFFIX, CancelToken, ContentHash, Error, ErrorKind, FileEntry, HashAlgorithm, Hasher,
    Manifest, OperationId, PlanOptions, RelativePath, RepositorySource,
};
use zip::CompressionMethod;
use zip::write::SimpleFileOptions;

const MIB: u64 = 1024 * 1024;

fn noise(len: usize, seed: u64) -> Vec<u8> {
    let mut x = seed | 1;
    (0..len)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            x as u8
        })
        .collect()
}

fn sha1(data: &[u8]) -> ContentHash {
    Sha256Hasher.hash(HashAlgorithm::Sha1, data)
}

/// A repository with plain files and, optionally, packed files (announced with their archive size).
struct Repository {
    manifest: Manifest,
    files: BTreeMap<RelativePath, Vec<u8>>,
    reads: Cell<u32>,
}

impl Repository {
    fn new(entries: Vec<(&str, &[u8])>, packed: &[&str]) -> Self {
        let mut files = BTreeMap::new();
        let mut archives = BTreeMap::new();
        let mut manifest_entries = Vec::new();
        for (path, data) in entries {
            let relative = RelativePath::parse(path).unwrap();
            manifest_entries.push(FileEntry {
                path: relative.clone(),
                size: data.len() as u64,
                hash: sha1(data),
            });
            if packed.contains(&path) {
                let archive = zip_of(data);
                archives.insert(relative.clone(), archive.len() as u64);
                files.insert(
                    RelativePath::parse(&format!("{path}{ARCHIVE_SUFFIX}")).unwrap(),
                    archive,
                );
            } else {
                files.insert(relative, data.to_vec());
            }
        }
        let manifest = Manifest::new("9", manifest_entries)
            .unwrap()
            .with_archives(archives)
            .unwrap();
        Self {
            manifest,
            files,
            reads: Cell::new(0),
        }
    }
}

fn zip_of(data: &[u8]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    writer
        .start_file(
            "entry.bin",
            SimpleFileOptions::default().compression_method(CompressionMethod::Deflated),
        )
        .unwrap();
    writer.write_all(data).unwrap();
    writer.finish().unwrap().into_inner()
}

impl RepositorySource for Repository {
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

fn run(repository: &Repository, store: &mut InMemoryStore) -> Result<(), Error> {
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
    apply(OperationId::new(1), &planned, &options, &mut context).map(|_| ())
}

#[test]
fn enough_free_space_lets_the_run_go_ahead() {
    let data = noise(150_000, 1);
    let repository = Repository::new(vec![("@a/a.pbo", &data)], &[]);
    let mut store = InMemoryStore::new().with_free_space(data.len() as u64 + FREE_SPACE_RESERVE);

    run(&repository, &mut store).expect("exactly enough space is enough");

    assert_eq!(store.content("@a/a.pbo"), Some(data.as_slice()));
}

#[test]
fn one_byte_short_is_refused_before_anything_is_downloaded_or_changed() {
    let data = noise(150_000, 2);
    let repository = Repository::new(vec![("@a/a.pbo", &data)], &[]);
    let mut store = InMemoryStore::new()
        .with_file("@a/a.pbo", b"the old version")
        .unwrap()
        .with_free_space(data.len() as u64 + FREE_SPACE_RESERVE - 1);

    let error = run(&repository, &mut store).expect_err("one byte short must be refused");

    assert_eq!(error.kind(), ErrorKind::FileSystem, "{error}");
    assert!(
        error.to_string().contains("not enough free space"),
        "{error}"
    );
    assert_eq!(repository.reads.get(), 0, "no transfer started");
    assert_eq!(store.content("@a/a.pbo"), Some(&b"the old version"[..]));
}

#[test]
fn a_later_file_that_does_not_fit_stops_the_run_before_the_first_file_is_written() {
    let small = noise(1000, 3);
    let large = noise(100_000, 4);
    let repository = Repository::new(vec![("@a/a.pbo", &small), ("@b/b.pbo", &large)], &[]);
    // Room for the small file, not for the large one after it (the check walks the plan in order).
    let free = FREE_SPACE_RESERVE + 50_000;
    let mut store = InMemoryStore::new().with_free_space(free);

    let error = run(&repository, &mut store).expect_err("the large file does not fit");

    assert_eq!(error.kind(), ErrorKind::FileSystem);
    assert_eq!(repository.reads.get(), 0, "nothing is downloaded");
    assert_eq!(
        store.content("@a/a.pbo"),
        None,
        "the small file is not written either"
    );
}

#[test]
fn unknown_free_space_does_not_block_the_run() {
    let data = noise(150_000, 5);
    let repository = Repository::new(vec![("@a/a.pbo", &data)], &[]);
    let mut store = InMemoryStore::new();

    run(&repository, &mut store).expect("unknown space is not a refusal");

    assert_eq!(store.content("@a/a.pbo"), Some(data.as_slice()));
}

#[test]
fn a_packed_file_needs_its_archive_as_well_as_the_unpacked_file() {
    let data = noise(150_000, 6);
    let archive_len = zip_of(&data).len() as u64;
    let repository = Repository::new(vec![("@a/a.pbo", &data)], &["@a/a.pbo"]);
    let needed = data.len() as u64 + archive_len + FREE_SPACE_RESERVE;

    let mut short = InMemoryStore::new().with_free_space(needed - 1);
    let error = run(&repository, &mut short).expect_err("the archive is not counted");
    assert_eq!(error.kind(), ErrorKind::FileSystem);
    assert_eq!(repository.reads.get(), 0);

    let mut enough = InMemoryStore::new().with_free_space(needed);
    run(&repository, &mut enough).expect("unpacked file plus archive plus reserve fits");
    assert_eq!(enough.content("@a/a.pbo"), Some(data.as_slice()));
}

#[test]
fn a_replaced_file_gives_its_old_size_back_for_the_next_file() {
    // The first file replaces an old file of the same size, so after its commit the old space is free again.
    let first = noise(MIB as usize, 7);
    let second = noise(2 * MIB as usize, 8);
    let repository = Repository::new(vec![("@a/a.pbo", &first), ("@b/b.pbo", &second)], &[]);
    let old = noise(MIB as usize, 9);
    let mut store = InMemoryStore::new()
        .with_file("@a/a.pbo", &old)
        .unwrap()
        .with_free_space(second.len() as u64 + FREE_SPACE_RESERVE);

    run(&repository, &mut store).expect("the replacement frees the old space");

    assert_eq!(store.content("@b/b.pbo"), Some(second.as_slice()));
    assert_eq!(store.content("@a/a.pbo"), Some(first.as_slice()));
}

#[test]
fn a_size_that_overflows_is_refused_without_panicking() {
    let data = noise(10, 10);
    let mut repository = Repository::new(vec![("@a/a.pbo", &data)], &[]);
    // A manifest that announces an impossible size.
    let entry = FileEntry {
        path: RelativePath::parse("@a/a.pbo").unwrap(),
        size: u64::MAX,
        hash: sha1(&data),
    };
    repository.manifest = Manifest::new("9", vec![entry]).unwrap();
    let mut store = InMemoryStore::new().with_free_space(u64::MAX);

    let error = run(&repository, &mut store).expect_err("overflow is not enough space");

    assert_eq!(error.kind(), ErrorKind::FileSystem);
    assert_eq!(repository.reads.get(), 0);
}
