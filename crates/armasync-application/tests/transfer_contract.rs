// SPDX-License-Identifier: AGPL-3.0-only
//! The streaming transfer contract (ADR-0007) at use-case level: staging, verification while receiving, atomic
//! commit, progress, cancellation, network and disk failures, existing target files, protected paths and large
//! files that are never held in memory. All data is synthetic.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::io::{self, Write};

use armasync_adapters::{InMemoryRepository, InMemoryStore, MemoryEventSink, Sha256Hasher};
use armasync_application::sync::{SyncContext, apply, plan, plan_cancellable};
use armasync_core::{
    CancelToken, Error, ErrorKind, FileEntry, HashAlgorithm, Hasher, JobState, LocalFile,
    LocalStore, Manifest, OperationEvent, OperationId, Phase, PlanOptions, RelativePath,
    RepositorySource, StagedFile, SyncStep, TRANSFER_CHUNK,
};

fn path(text: &str) -> RelativePath {
    RelativePath::parse(text).unwrap()
}

fn run(
    source: &dyn RepositorySource,
    store: &mut dyn LocalStore,
    options: &PlanOptions,
    cancel: &CancelToken,
) -> (
    Result<armasync_application::sync::ApplyReport, Error>,
    Vec<OperationEvent>,
) {
    let planned = plan_cancellable(source, store, options, cancel).unwrap();
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
        apply(OperationId::new(1), &planned, options, &mut context)
    };
    (result, sink.events)
}

fn big(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i % 251) as u8).collect()
}

#[test]
fn a_streamed_download_is_verified_committed_and_reported() {
    let data = big(TRANSFER_CHUNK * 5 + 123);
    let source = InMemoryRepository::new("r", &[("@m/big.pbo", data.as_slice())])
        .unwrap()
        .with_algorithm(HashAlgorithm::Sha1);
    let mut store = InMemoryStore::new();
    let (result, events) = run(
        &source,
        &mut store,
        &PlanOptions::default(),
        &CancelToken::new(),
    );
    let report = result.unwrap();
    assert_eq!((report.downloaded, report.bytes), (1, data.len() as u64));
    assert_eq!(store.content("@m/big.pbo"), Some(data.as_slice()));
    // progress: byte counters never decrease, never exceed the total, and end at the total
    let mut last = 0;
    for event in &events {
        assert!(event.progress.bytes_done >= last, "bytes_done decreased");
        assert!(event.progress.bytes_done <= event.progress.bytes_total);
        last = event.progress.bytes_done;
    }
    let end = events.last().unwrap();
    assert_eq!(end.state, JobState::Succeeded);
    assert_eq!(end.progress.bytes_done, data.len() as u64);
    assert!(
        events
            .iter()
            .filter(|e| e.phase == Phase::Transferring)
            .count()
            >= 3,
        "intermediate progress events during a large transfer"
    );
}

#[test]
fn cancellation_in_the_middle_of_a_transfer_leaves_the_existing_file_untouched() {
    let data = big(TRANSFER_CHUNK * 6);
    let source = InMemoryRepository::new("r", &[("@m/a.pbo", data.as_slice())]).unwrap();
    let mut store = InMemoryStore::new()
        .with_file("@m/a.pbo", b"old content")
        .unwrap();
    let cancel = CancelToken::new();
    let planned = plan(&source, &store, &PlanOptions::default()).unwrap();
    let mut sink = CancelAfter {
        events: Vec::new(),
        after_bytes: TRANSFER_CHUNK as u64 * 2,
        cancel: cancel.clone(),
    };
    let hasher = Sha256Hasher;
    let result = {
        let mut context = SyncContext {
            source: &source,
            store: &mut store,
            hasher: &hasher,
            unpacker: &armasync_adapters::archive::ZipUnpacker,
            sink: &mut sink,
            cancel: &cancel,
        };
        apply(
            OperationId::new(1),
            &planned,
            &PlanOptions::default(),
            &mut context,
        )
    };
    assert_eq!(result.unwrap_err().kind(), ErrorKind::Cancelled);
    assert_eq!(store.content("@m/a.pbo"), Some(b"old content".as_slice()));
    assert_eq!(store.paths(), ["@m/a.pbo"]);
    let last = sink.events.last().unwrap();
    assert_eq!(last.state, JobState::Cancelled);
    assert_eq!(
        last.progress.bytes_done, 0,
        "unfinished bytes are not counted as done"
    );
}

struct CancelAfter {
    events: Vec<OperationEvent>,
    after_bytes: u64,
    cancel: CancelToken,
}

impl armasync_core::EventSink for CancelAfter {
    fn emit(&mut self, event: OperationEvent) {
        if event.progress.bytes_done >= self.after_bytes {
            self.cancel.cancel();
        }
        self.events.push(event);
    }
}

#[test]
fn a_dropped_connection_keeps_the_old_file_and_leaves_no_partial_file() {
    let data = big(TRANSFER_CHUNK * 3);
    let mut source =
        InMemoryRepository::new("r", &[("@m/a.pbo", data.as_slice()), ("@m/b.pbo", b"bee")])
            .unwrap();
    source.truncate_transfer(&path("@m/a.pbo"), TRANSFER_CHUNK + 5);
    let mut store = InMemoryStore::new().with_file("@m/a.pbo", b"old").unwrap();
    let (result, events) = run(
        &source,
        &mut store,
        &PlanOptions::default(),
        &CancelToken::new(),
    );
    assert_eq!(result.unwrap_err().kind(), ErrorKind::Network);
    assert_eq!(store.content("@m/a.pbo"), Some(b"old".as_slice()));
    assert_eq!(
        store.paths(),
        ["@m/a.pbo"],
        "no partial or staged file anywhere"
    );
    let last = events.last().unwrap();
    assert_eq!(
        (last.state, last.error),
        (JobState::Failed, Some(ErrorKind::Network))
    );
    assert_eq!(last.progress.bytes_done, 0);
}

#[test]
fn a_full_disk_during_staging_fails_without_touching_the_target() {
    let data = big(TRANSFER_CHUNK * 2);
    let source = InMemoryRepository::new("r", &[("@m/a.pbo", data.as_slice())]).unwrap();
    let mut store = InMemoryStore::new().with_file("@m/a.pbo", b"old").unwrap();
    store.fail_staging_after(TRANSFER_CHUNK);
    let (result, _) = run(
        &source,
        &mut store,
        &PlanOptions::default(),
        &CancelToken::new(),
    );
    assert_eq!(result.unwrap_err().kind(), ErrorKind::FileSystem);
    assert_eq!(store.content("@m/a.pbo"), Some(b"old".as_slice()));
}

#[test]
fn a_failing_commit_keeps_the_old_file() {
    let source = InMemoryRepository::new("r", &[("@m/a.pbo", b"new".as_slice())]).unwrap();
    let mut store = InMemoryStore::new().with_file("@m/a.pbo", b"old").unwrap();
    store.fail_writes_to(&path("@m/a.pbo"));
    let (result, _) = run(
        &source,
        &mut store,
        &PlanOptions::default(),
        &CancelToken::new(),
    );
    assert_eq!(result.unwrap_err().kind(), ErrorKind::FileSystem);
    assert_eq!(store.content("@m/a.pbo"), Some(b"old".as_slice()));
}

#[test]
fn wrong_checksum_wrong_length_and_overlong_data_are_integrity_errors() {
    for (served, what) in [
        (b"nnn".to_vec(), "same length, different content"),
        (b"short".to_vec(), "shorter"),
        (b"muchlonger".to_vec(), "longer"),
    ] {
        let mut source = InMemoryRepository::new("r", &[("a.pbo", b"abc".as_slice())]).unwrap();
        source.serve_corrupted(&path("a.pbo"), &served);
        let mut store = InMemoryStore::new().with_file("a.pbo", b"old").unwrap();
        let (result, _) = run(
            &source,
            &mut store,
            &PlanOptions::default(),
            &CancelToken::new(),
        );
        assert_eq!(result.unwrap_err().kind(), ErrorKind::Integrity, "{what}");
        assert_eq!(store.content("a.pbo"), Some(b"old".as_slice()), "{what}");
    }
}

#[test]
fn an_identical_file_is_not_fetched_again() {
    let source = CountingSource::new("a.pbo", b"same");
    let mut store = InMemoryStore::new().with_file("a.pbo", b"same").unwrap();
    let (result, _) = run(
        &source,
        &mut store,
        &PlanOptions::default(),
        &CancelToken::new(),
    );
    let report = result.unwrap();
    assert_eq!((report.kept, report.downloaded, report.replaced), (1, 0, 0));
    assert_eq!(source.fetches(), 0);
}

/// A source that counts `fetch` calls (SHA-256 manifest, content in memory).
struct CountingSource {
    inner: InMemoryRepository,
    fetched: std::cell::Cell<u32>,
}

impl CountingSource {
    fn new(name: &str, data: &[u8]) -> Self {
        Self {
            inner: InMemoryRepository::new("r", &[(name, data)]).unwrap(),
            fetched: std::cell::Cell::new(0),
        }
    }

    fn fetches(&self) -> u32 {
        self.fetched.get()
    }
}

impl RepositorySource for CountingSource {
    fn manifest(&self) -> Result<Manifest, Error> {
        self.inner.manifest()
    }

    fn read(&self, path: &RelativePath) -> Result<Vec<u8>, Error> {
        self.inner.read(path)
    }

    fn open(
        &self,
        path: &RelativePath,
        expected_size: u64,
        offset: u64,
        if_range: Option<&str>,
    ) -> Result<armasync_core::Opened, Error> {
        self.fetched.set(self.fetched.get() + 1);
        self.inner.open(path, expected_size, offset, if_range)
    }
}

#[test]
fn protected_and_unsupported_paths_survive_a_deleting_sync() {
    let source = InMemoryRepository::new("r", &[("@m/new.pbo", b"new".as_slice())]).unwrap();
    let mut store = InMemoryStore::new()
        .with_file("@m/extra.pbo", b"extra")
        .unwrap()
        .with_file(
            "@m/addons/local_unzipped.pbo",
            b"unzipped copy of a compressed file",
        )
        .unwrap()
        .with_file("@m/hidden/mine.txt", b"mine")
        .unwrap();
    let options = PlanOptions {
        delete_extra: true,
        managed_roots: vec![path("@m")],
        protected_paths: vec![path("@m/addons/local_unzipped.pbo"), path("@m/hidden")],
    };
    let (result, _) = run(&source, &mut store, &options, &CancelToken::new());
    let report = result.unwrap();
    assert_eq!(report.deleted, 1);
    assert_eq!(store.content("@m/extra.pbo"), None);
    assert!(store.content("@m/addons/local_unzipped.pbo").is_some());
    assert!(store.content("@m/hidden/mine.txt").is_some());
    assert!(store.content("@m/new.pbo").is_some());
}

/// A 64 MiB file that exists only as a generator: neither side ever holds it in memory.
struct GeneratedSource {
    size: u64,
    manifest: Manifest,
}

fn generated_byte(i: u64) -> u8 {
    (i.wrapping_mul(2_654_435_761) >> 13) as u8
}

impl GeneratedSource {
    fn new(size: u64) -> Self {
        let hasher = Sha256Hasher;
        let mut digest = hasher.digest(HashAlgorithm::Sha1);
        let mut chunk = vec![0u8; TRANSFER_CHUNK];
        let mut done = 0u64;
        while done < size {
            let n = (size - done).min(TRANSFER_CHUNK as u64) as usize;
            for (offset, byte) in chunk[..n].iter_mut().enumerate() {
                *byte = generated_byte(done + offset as u64);
            }
            digest.update(&chunk[..n]);
            done += n as u64;
        }
        let entry = FileEntry {
            path: path("@m/huge.pbo"),
            size,
            hash: digest.finish(),
        };
        Self {
            size,
            manifest: Manifest::new("1", vec![entry]).unwrap(),
        }
    }
}

impl RepositorySource for GeneratedSource {
    fn manifest(&self) -> Result<Manifest, Error> {
        Ok(self.manifest.clone())
    }

    fn read(&self, _path: &RelativePath) -> Result<Vec<u8>, Error> {
        panic!("the transfer must stream, not read the whole file");
    }

    fn open(
        &self,
        _path: &RelativePath,
        expected_size: u64,
        offset: u64,
        _if_range: Option<&str>,
    ) -> Result<armasync_core::Opened, Error> {
        assert_eq!(expected_size, self.size);
        assert_eq!(offset, 0, "this source has no validator and never resumes");
        Ok(armasync_core::Opened {
            start: 0,
            validator: None,
            body: Box::new(GeneratedBody {
                size: self.size,
                done: 0,
            }),
        })
    }
}

/// The generated content of a source, streamed chunk by chunk.
struct GeneratedBody {
    size: u64,
    done: u64,
}

impl io::Read for GeneratedBody {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let left = self.size - self.done;
        let n = usize::try_from(left).map_or(buffer.len(), |left| left.min(buffer.len()));
        for (offset, byte) in buffer[..n].iter_mut().enumerate() {
            *byte = generated_byte(self.done + offset as u64);
        }
        self.done += n as u64;
        Ok(n)
    }
}

/// A store whose staging files only count bytes (and remember the largest single write).
#[derive(Default)]
struct CountingStore {
    committed: Option<(RelativePath, u64)>,
    largest_write: usize,
}

struct CountingStage<'a> {
    store: &'a mut CountingStore,
    path: RelativePath,
    bytes: u64,
}

impl Write for CountingStage<'_> {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        self.store.largest_write = self.store.largest_write.max(data.len());
        self.bytes += data.len() as u64;
        Ok(data.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl StagedFile for CountingStage<'_> {
    fn commit(self: Box<Self>) -> Result<(), Error> {
        let this = *self;
        this.store.committed = Some((this.path, this.bytes));
        Ok(())
    }
}

impl LocalStore for CountingStore {
    fn list(&self, _a: HashAlgorithm, _c: &CancelToken) -> Result<Vec<LocalFile>, Error> {
        Ok(Vec::new())
    }

    fn begin<'a>(
        &'a mut self,
        path: &RelativePath,
        _expected_size: u64,
    ) -> Result<Box<dyn StagedFile + 'a>, Error> {
        Ok(Box::new(CountingStage {
            store: self,
            path: path.clone(),
            bytes: 0,
        }))
    }

    fn write(&mut self, _p: &RelativePath, _d: &[u8]) -> Result<(), Error> {
        panic!("the transfer must stage, not write whole files");
    }

    fn delete(&mut self, _p: &RelativePath) -> Result<(), Error> {
        Ok(())
    }

    fn spool(&mut self, _expected_size: u64) -> Result<Box<dyn armasync_core::Spool>, Error> {
        Ok(Box::new(std::io::Cursor::new(Vec::new())))
    }

    fn partial(
        &mut self,
        _key: &armasync_core::PartialKey,
    ) -> Result<Box<dyn armasync_core::PartialFile + '_>, Error> {
        Ok(Box::new(CountingPartial {
            store: self,

            bytes: 0,
        }))
    }
}

/// The partial file of [`CountingStore`]: counts the bytes, keeps nothing, and reports the total on commit.
struct CountingPartial<'a> {
    store: &'a mut CountingStore,
    bytes: u64,
}

impl Write for CountingPartial<'_> {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        self.store.largest_write = self.store.largest_write.max(data.len());
        self.bytes += data.len() as u64;
        Ok(data.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl io::Read for CountingPartial<'_> {
    fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
        Ok(0)
    }
}

impl io::Seek for CountingPartial<'_> {
    fn seek(&mut self, _position: io::SeekFrom) -> io::Result<u64> {
        Ok(self.bytes)
    }
}

impl armasync_core::PartialFile for CountingPartial<'_> {
    fn stored(&self) -> Option<armasync_core::StoredPartial> {
        None
    }

    fn restart(&mut self, _validator: Option<&str>) -> Result<(), Error> {
        Ok(())
    }

    fn keep(self: Box<Self>) {}

    fn discard(self: Box<Self>) {}

    fn commit_to(
        self: Box<Self>,
        target: &RelativePath,
        _expected: &armasync_core::ContentHash,
    ) -> Result<(), Error> {
        let this = *self;
        this.store.committed = Some((target.clone(), this.bytes));
        Ok(())
    }
}

#[test]
fn a_64_mib_file_is_transferred_and_verified_without_ever_being_held_in_memory() {
    let source = GeneratedSource::new(64 * 1024 * 1024);
    let mut store = CountingStore::default();
    let (result, events) = run(
        &source,
        &mut store,
        &PlanOptions::default(),
        &CancelToken::new(),
    );
    let report = result.unwrap();
    assert_eq!(report.bytes, 64 * 1024 * 1024);
    assert_eq!(
        store.committed,
        Some((path("@m/huge.pbo"), 64 * 1024 * 1024))
    );
    assert!(
        store.largest_write <= TRANSFER_CHUNK,
        "writes are chunk-sized"
    );
    // bounded number of progress events (64 MiB / 256 KiB steps), not one per chunk write storm
    assert!(events.len() < 400, "{} events", events.len());
}

#[test]
fn cancelling_a_huge_transfer_stops_it_early() {
    let source = GeneratedSource::new(64 * 1024 * 1024);
    let mut store = CountingStore::default();
    let cancel = CancelToken::new();
    let planned = plan(&source, &store, &PlanOptions::default()).unwrap();
    let mut sink = CancelAfter {
        events: Vec::new(),
        after_bytes: 1024 * 1024,
        cancel: cancel.clone(),
    };
    let hasher = Sha256Hasher;
    let result = {
        let mut context = SyncContext {
            source: &source,
            store: &mut store,
            hasher: &hasher,
            unpacker: &armasync_adapters::archive::ZipUnpacker,
            sink: &mut sink,
            cancel: &cancel,
        };
        apply(
            OperationId::new(1),
            &planned,
            &PlanOptions::default(),
            &mut context,
        )
    };
    assert_eq!(result.unwrap_err().kind(), ErrorKind::Cancelled);
    assert!(store.committed.is_none());
    assert!(
        sink.events.len() < 40,
        "stopped long before the end: {}",
        sink.events.len()
    );
}

#[test]
fn plan_steps_use_the_manifest_algorithm_for_the_local_scan() {
    // The in-memory store hashes with the requested algorithm, so a SHA-1 repository and an identical local file
    // are recognised as identical (no re-download).
    let source = InMemoryRepository::new("r", &[("a.pbo", b"same".as_slice())])
        .unwrap()
        .with_algorithm(HashAlgorithm::Sha1);
    let store = InMemoryStore::new().with_file("a.pbo", b"same").unwrap();
    let planned = plan(&source, &store, &PlanOptions::default()).unwrap();
    assert!(matches!(planned.steps.as_slice(), [SyncStep::Keep { .. }]));
}
