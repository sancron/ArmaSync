// SPDX-License-Identifier: AGPL-3.0-only
//! The sync use cases against the real local file-system adapter (MIG-030C): first sync, repeat sync without
//! transfers, replaced and extra files, protected paths, cancellation and a large streamed file. The repository
//! side is the in-memory reference; HTTP and the CLI are not involved. All data is synthetic.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::{Path, PathBuf};

use armasync_adapters::localfs::LocalDirectory;
use armasync_adapters::{InMemoryRepository, MemoryEventSink, Sha256Hasher};
use armasync_application::sync::{SyncContext, apply, plan};
use armasync_core::{
    CancelToken, Error, ErrorKind, FileEntry, HashAlgorithm, Hasher, JobState, Manifest,
    OperationEvent, OperationId, PlanOptions, RelativePath, RepositorySource, TRANSFER_CHUNK,
};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "armasync-localfs-flow-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
    ));
    // Unique even if the clock is coarse (two tests can read the same timestamp): `create_dir` fails when the
    // name exists, then the next suffix is tried.
    let mut candidate = dir.clone();
    let mut attempt = 0u32;
    while fs::create_dir(&candidate).is_err() {
        attempt += 1;
        assert!(attempt < 1000, "cannot create a unique scratch folder");
        candidate = PathBuf::from(format!("{}-{attempt}", dir.display()));
    }
    candidate
}

fn path(text: &str) -> RelativePath {
    RelativePath::parse(text).unwrap()
}

fn files(root: &Path) -> Vec<String> {
    fn walk(dir: &Path, prefix: &str, out: &mut Vec<String>) {
        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let name = format!("{prefix}{}", entry.file_name().to_string_lossy());
            if name == ".armasync-lock" {
                continue;
            }
            if entry.path().is_dir() {
                walk(&entry.path(), &format!("{name}/"), out);
            } else {
                out.push(name);
            }
        }
    }
    let mut out = Vec::new();
    walk(root, "", &mut out);
    out.sort();
    out
}

fn sync(
    source: &dyn RepositorySource,
    root: &Path,
    options: &PlanOptions,
    cancel: &CancelToken,
    sink: &mut MemoryEventSink,
) -> Result<armasync_application::sync::ApplyReport, Error> {
    let mut store = LocalDirectory::open_exclusive(root).unwrap();
    let planned = plan(source, &store, options).unwrap();
    let hasher = Sha256Hasher;
    let mut context = SyncContext {
        source,
        store: &mut store,
        hasher: &hasher,
        unpacker: &armasync_adapters::archive::ZipUnpacker,
        sink,
        cancel,
    };
    apply(OperationId::new(1), &planned, options, &mut context)
}

fn run(
    source: &dyn RepositorySource,
    root: &Path,
    options: &PlanOptions,
) -> armasync_application::sync::ApplyReport {
    sync(
        source,
        root,
        options,
        &CancelToken::new(),
        &mut MemoryEventSink::default(),
    )
    .unwrap()
}

fn repo(algorithm: HashAlgorithm, content: &[(&str, &[u8])]) -> InMemoryRepository {
    InMemoryRepository::new("r1", content)
        .unwrap()
        .with_algorithm(algorithm)
}

#[test]
fn first_sync_writes_real_files_and_the_repeat_changes_nothing() {
    for algorithm in [HashAlgorithm::Sha1, HashAlgorithm::Sha256] {
        let root = scratch("first");
        let source = repo(
            algorithm,
            &[
                ("@m/addons/a.pbo", b"alpha"),
                ("@m/keys/k.bikey", b"key"),
                ("top.txt", b"t"),
            ],
        );
        let report = run(&source, &root, &PlanOptions::default());
        assert_eq!(
            (report.downloaded, report.replaced, report.deleted),
            (3, 0, 0),
            "{algorithm:?}"
        );
        assert_eq!(fs::read(root.join("@m/addons/a.pbo")).unwrap(), b"alpha");
        assert_eq!(
            files(&root),
            ["@m/addons/a.pbo", "@m/keys/k.bikey", "top.txt"]
        );
        let again = run(&source, &root, &PlanOptions::default());
        assert_eq!(
            (again.downloaded, again.replaced, again.kept),
            (0, 0, 3),
            "{algorithm:?}: nothing is fetched again"
        );
        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn changed_files_are_replaced_and_extra_files_deleted_only_when_asked() {
    let root = scratch("changes");
    let source = repo(
        HashAlgorithm::Sha1,
        &[("@m/a.pbo", b"new alpha"), ("@m/b.pbo", b"beta")],
    );
    fs::create_dir_all(root.join("@m")).unwrap();
    fs::write(root.join("@m/a.pbo"), b"old alpha").unwrap();
    fs::write(root.join("@m/extra.pbo"), b"extra").unwrap();
    let report = run(&source, &root, &PlanOptions::default());
    assert_eq!(
        (report.replaced, report.downloaded, report.deleted),
        (1, 1, 0)
    );
    assert_eq!(fs::read(root.join("@m/a.pbo")).unwrap(), b"new alpha");
    assert!(
        root.join("@m/extra.pbo").exists(),
        "no deletion without the flag"
    );
    let options = PlanOptions {
        delete_extra: true,
        managed_roots: vec![path("@m")],
        ..PlanOptions::default()
    };
    let report = run(&source, &root, &options);
    assert_eq!(report.deleted, 1);
    assert_eq!(files(&root), ["@m/a.pbo", "@m/b.pbo"]);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn protected_paths_survive_a_deleting_sync_on_disk() {
    let root = scratch("protected");
    let source = repo(HashAlgorithm::Sha1, &[("@m/new.pbo", b"n")]);
    fs::create_dir_all(root.join("@m/hidden")).unwrap();
    fs::write(root.join("@m/hidden/mine.txt"), b"mine").unwrap();
    fs::write(
        root.join("@m/unzipped.pbo"),
        b"local copy of a compressed file",
    )
    .unwrap();
    fs::write(root.join("@m/extra.pbo"), b"extra").unwrap();
    let options = PlanOptions {
        delete_extra: true,
        managed_roots: vec![path("@m")],
        protected_paths: vec![path("@m/hidden"), path("@m/unzipped.pbo")],
    };
    let report = run(&source, &root, &options);
    assert_eq!(report.deleted, 1);
    assert_eq!(
        files(&root),
        ["@m/hidden/mine.txt", "@m/new.pbo", "@m/unzipped.pbo"]
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_corrupted_transfer_leaves_the_old_file_and_no_temporary_file() {
    let root = scratch("corrupt");
    let mut source = repo(HashAlgorithm::Sha1, &[("a.pbo", b"expected content")]);
    source.serve_corrupted(&path("a.pbo"), b"different bytes!!");
    fs::write(root.join("a.pbo"), b"old").unwrap();
    let error = sync(
        &source,
        &root,
        &PlanOptions::default(),
        &CancelToken::new(),
        &mut MemoryEventSink::default(),
    )
    .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Integrity);
    assert_eq!(fs::read(root.join("a.pbo")).unwrap(), b"old");
    assert_eq!(files(&root), ["a.pbo"]);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_dropped_connection_leaves_the_old_file_and_only_its_resumable_partial_bytes() {
    // ADR-0009, section 5: a network failure keeps the received bytes (with their validator) for a resume; the target
    // stays untouched. A cooperative cancellation would leave nothing (see the resume tests).
    let root = scratch("dropped");
    let big: Vec<u8> = (0..TRANSFER_CHUNK * 4).map(|i| (i % 251) as u8).collect();
    let mut source = repo(HashAlgorithm::Sha1, &[("a.pbo", big.as_slice())]);
    source.truncate_transfer(&path("a.pbo"), TRANSFER_CHUNK + 10);
    fs::write(root.join("a.pbo"), b"old").unwrap();
    let error = sync(
        &source,
        &root,
        &PlanOptions::default(),
        &CancelToken::new(),
        &mut MemoryEventSink::default(),
    )
    .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Network);
    assert_eq!(fs::read(root.join("a.pbo")).unwrap(), b"old");
    let leftovers: Vec<String> = files(&root)
        .into_iter()
        .filter(|name| name != "a.pbo")
        .collect();
    assert!(
        leftovers
            .iter()
            .all(|name| name.starts_with(armasync_adapters::localfs::PARTIAL_PREFIX)),
        "only resumable partial files may remain: {leftovers:?}"
    );
    assert_eq!(leftovers.len(), 2, "data and state of the partial file");
    fs::remove_dir_all(root).unwrap();
}

struct CancelAfter {
    cancel: CancelToken,
    after: u64,
    last: Option<OperationEvent>,
}

impl armasync_core::EventSink for CancelAfter {
    fn emit(&mut self, event: OperationEvent) {
        if event.progress.bytes_done >= self.after {
            self.cancel.cancel();
        }
        self.last = Some(event);
    }
}

#[test]
fn cancellation_leaves_finished_files_in_place_and_nothing_partial() {
    let root = scratch("cancel");
    let big: Vec<u8> = (0..TRANSFER_CHUNK * 8).map(|i| (i % 241) as u8).collect();
    let source = repo(
        HashAlgorithm::Sha1,
        &[("a.pbo", b"small"), ("z.pbo", big.as_slice())],
    );
    fs::write(root.join("z.pbo"), b"old z").unwrap();
    let mut store = LocalDirectory::open_exclusive(&root).unwrap();
    let planned = plan(&source, &store, &PlanOptions::default()).unwrap();
    let cancel = CancelToken::new();
    let mut sink = CancelAfter {
        cancel: cancel.clone(),
        after: 5 + TRANSFER_CHUNK as u64,
        last: None,
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
    assert_eq!(sink.last.unwrap().state, JobState::Cancelled);
    assert_eq!(
        fs::read(root.join("a.pbo")).unwrap(),
        b"small",
        "the finished file stays"
    );
    assert_eq!(
        fs::read(root.join("z.pbo")).unwrap(),
        b"old z",
        "the interrupted file keeps its old content"
    );
    assert_eq!(files(&root), ["a.pbo", "z.pbo"]);
    fs::remove_dir_all(root).unwrap();
}

/// A 32 MiB file that exists only as a generator on the repository side.
struct Generated {
    manifest: Manifest,
    size: u64,
}

fn generated_byte(i: u64) -> u8 {
    (i.wrapping_mul(2_654_435_761) >> 11) as u8
}

impl Generated {
    fn new(size: u64) -> Self {
        let mut digest = Sha256Hasher.digest(HashAlgorithm::Sha1);
        let mut done = 0u64;
        let mut chunk = vec![0u8; TRANSFER_CHUNK];
        while done < size {
            let n = (size - done).min(TRANSFER_CHUNK as u64) as usize;
            for (o, b) in chunk[..n].iter_mut().enumerate() {
                *b = generated_byte(done + o as u64);
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
            manifest: Manifest::new("1", vec![entry]).unwrap(),
            size,
        }
    }
}

impl RepositorySource for Generated {
    fn manifest(&self) -> Result<Manifest, Error> {
        Ok(self.manifest.clone())
    }

    fn read(&self, _p: &RelativePath) -> Result<Vec<u8>, Error> {
        panic!("must stream");
    }

    fn open(
        &self,
        _p: &RelativePath,
        expected: u64,
        offset: u64,
        _if_range: Option<&str>,
    ) -> Result<armasync_core::Opened, Error> {
        assert_eq!(expected, self.size);
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

/// The generated content of a source, streamed (never held in memory).
struct GeneratedBody {
    size: u64,
    done: u64,
}

impl std::io::Read for GeneratedBody {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        let left = self.size - self.done;
        let n = usize::try_from(left).map_or(buffer.len(), |left| left.min(buffer.len()));
        for (offset, byte) in buffer[..n].iter_mut().enumerate() {
            *byte = generated_byte(self.done + offset as u64);
        }
        self.done += n as u64;
        Ok(n)
    }
}

#[test]
fn a_large_file_is_streamed_to_disk_verified_and_found_identical_afterwards() {
    let root = scratch("large");
    let size = 32 * 1024 * 1024;
    let source = Generated::new(size);
    let report = run(&source, &root, &PlanOptions::default());
    assert_eq!(report.bytes, size);
    assert_eq!(fs::metadata(root.join("@m/huge.pbo")).unwrap().len(), size);
    assert_eq!(files(&root), ["@m/huge.pbo"]);
    // the scan hashes the 32 MiB file with SHA-1 and finds it identical: no transfer
    let again = run(&source, &root, &PlanOptions::default());
    assert_eq!((again.downloaded, again.replaced, again.kept), (0, 0, 1));
    fs::remove_dir_all(root).unwrap();
}
