// SPDX-License-Identifier: AGPL-3.0-only
//! In-memory adapters: a SHA-256 hasher, a repository, a local store and an event sink.
//!
//! They implement the core ports without touching the file system or the network, so use cases can be tested
//! deterministically. They are test and prototype adapters, not a persistence layer.

use std::collections::BTreeMap;
use std::io::{self, Write};

use armasync_core::{
    CancelToken, ContentHash, Digest as CoreDigest, Error, ErrorKind, EventSink, FileEntry,
    HashAlgorithm, Hasher, LocalFile, LocalStore, Manifest, OperationEvent, RelativePath,
    RepositorySource, StagedFile, copy_in_chunks,
};
use sha1::Sha1;
use sha2::{Digest, Sha256};

/// Implementation of the [`Hasher`] port. The name is historical: it computes SHA-256 and, for the integrity check
/// of existing repositories (ADR-0006), SHA-1.
#[derive(Debug, Clone, Copy, Default)]
pub struct Sha256Hasher;

struct Sha256State(Sha256);

impl CoreDigest for Sha256State {
    fn update(&mut self, data: &[u8]) {
        Digest::update(&mut self.0, data);
    }

    fn finish(self: Box<Self>) -> ContentHash {
        ContentHash::from_sha256_digest(self.0.finalize().into())
    }
}

struct Sha1State(Sha1);

impl CoreDigest for Sha1State {
    fn update(&mut self, data: &[u8]) {
        Digest::update(&mut self.0, data);
    }

    fn finish(self: Box<Self>) -> ContentHash {
        ContentHash::from_sha1_digest(self.0.finalize().into())
    }
}

impl Hasher for Sha256Hasher {
    fn digest(&self, algorithm: HashAlgorithm) -> Box<dyn CoreDigest> {
        match algorithm {
            HashAlgorithm::Sha256 => Box::new(Sha256State(Sha256::new())),
            HashAlgorithm::Sha1 => Box::new(Sha1State(Sha1::new())),
        }
    }
}

/// A repository held in memory. The *served* bytes can differ from the *advertised* manifest to simulate
/// corruption or a half-updated server.
#[derive(Debug, Clone)]
pub struct InMemoryRepository {
    revision: String,
    files: BTreeMap<RelativePath, Vec<u8>>,
    served_override: BTreeMap<RelativePath, Vec<u8>>,
    truncated: BTreeMap<RelativePath, usize>,
    algorithm: HashAlgorithm,
    unreachable: bool,
}

impl InMemoryRepository {
    /// Creates a repository at `revision` with the given files.
    ///
    /// # Panics
    /// Never; invalid paths in `files` are rejected by returning an error instead.
    ///
    /// # Errors
    /// [`ErrorKind::Format`] if a path is not a valid [`RelativePath`].
    pub fn new(revision: &str, files: &[(&str, &[u8])]) -> Result<Self, Error> {
        let mut map = BTreeMap::new();
        for (path, data) in files {
            let path = RelativePath::parse(path)
                .map_err(|error| Error::new(ErrorKind::Format, error.to_string()))?;
            map.insert(path, data.to_vec());
        }
        Ok(Self {
            revision: revision.to_owned(),
            files: map,
            served_override: BTreeMap::new(),
            truncated: BTreeMap::new(),
            algorithm: HashAlgorithm::Sha256,
            unreachable: false,
        })
    }

    /// Advertises the manifest with `algorithm` hashes (SHA-1 simulates an existing repository).
    #[must_use]
    pub fn with_algorithm(mut self, algorithm: HashAlgorithm) -> Self {
        self.algorithm = algorithm;
        self
    }

    /// Makes `fetch` of `path` deliver only the first `bytes` bytes and then fail with a network error
    /// (simulates a connection that drops mid-transfer).
    pub fn truncate_transfer(&mut self, path: &RelativePath, bytes: usize) {
        self.truncated.insert(path.clone(), bytes);
    }

    /// Publishes a new revision with different file content (replaces the file set).
    pub fn publish(&mut self, revision: &str, files: BTreeMap<RelativePath, Vec<u8>>) {
        self.revision = revision.to_owned();
        self.files = files;
        self.served_override.clear();
    }

    /// Makes `read` return `data` for `path` while the manifest still advertises the original hash.
    pub fn serve_corrupted(&mut self, path: &RelativePath, data: &[u8]) {
        self.served_override.insert(path.clone(), data.to_vec());
    }

    /// Simulates a network outage for all further calls.
    pub fn set_unreachable(&mut self, unreachable: bool) {
        self.unreachable = unreachable;
    }
}

impl RepositorySource for InMemoryRepository {
    fn manifest(&self) -> Result<Manifest, Error> {
        if self.unreachable {
            return Err(Error::new(ErrorKind::Network, "repository unreachable"));
        }
        let hasher = Sha256Hasher;
        let entries = self
            .files
            .iter()
            .map(|(path, data)| FileEntry {
                path: path.clone(),
                size: data.len() as u64,
                hash: hasher.hash(self.algorithm, data),
            })
            .collect();
        Manifest::new(self.revision.clone(), entries)
            .map_err(|error| Error::new(ErrorKind::Format, error.to_string()))
    }

    fn read(&self, path: &RelativePath) -> Result<Vec<u8>, Error> {
        if self.unreachable {
            return Err(Error::new(ErrorKind::Network, "repository unreachable"));
        }
        if let Some(data) = self.served_override.get(path) {
            return Ok(data.clone());
        }
        self.files
            .get(path)
            .cloned()
            .ok_or_else(|| Error::new(ErrorKind::Network, format!("not found: {path}")))
    }

    fn fetch(
        &self,
        path: &RelativePath,
        expected_size: u64,
        out: &mut dyn Write,
        cancel: &CancelToken,
        progress: &mut dyn FnMut(u64),
    ) -> Result<u64, Error> {
        if cancel.is_cancelled() {
            return Err(Error::new(ErrorKind::Cancelled, "cancelled"));
        }
        let data = self.read(path)?;
        if let Some(limit) = self.truncated.get(path) {
            let prefix = &data[..(*limit).min(data.len())];
            // The prefix is delivered; the connection then drops.
            match copy_in_chunks(prefix, prefix.len() as u64, out, cancel, progress) {
                Ok(_) => {
                    return Err(Error::new(
                        ErrorKind::Network,
                        "connection lost during the transfer",
                    ));
                }
                Err(error) => return Err(error),
            }
        }
        copy_in_chunks(&data, expected_size, out, cancel, progress)
    }
}

/// A local destination held in memory with atomic writes.
#[derive(Debug, Clone, Default)]
pub struct InMemoryStore {
    files: BTreeMap<RelativePath, Vec<u8>>,
    fail_writes_to: Option<RelativePath>,
    fail_staging_after: Option<usize>,
}

impl InMemoryStore {
    /// An empty store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Puts a file into the store (test setup).
    ///
    /// # Errors
    /// [`ErrorKind::Format`] for an invalid path.
    pub fn with_file(mut self, path: &str, data: &[u8]) -> Result<Self, Error> {
        let path = RelativePath::parse(path)
            .map_err(|error| Error::new(ErrorKind::Format, error.to_string()))?;
        self.files.insert(path, data.to_vec());
        Ok(self)
    }

    /// Makes writes to `path` fail with a file-system error (simulates a full disk).
    pub fn fail_writes_to(&mut self, path: &RelativePath) {
        self.fail_writes_to = Some(path.clone());
    }

    /// Makes writing into a staging file fail after `bytes` bytes with a file-system error (simulates a full disk
    /// during a transfer).
    pub fn fail_staging_after(&mut self, bytes: usize) {
        self.fail_staging_after = Some(bytes);
    }

    /// The content of `path`, if present.
    #[must_use]
    pub fn content(&self, path: &str) -> Option<&[u8]> {
        RelativePath::parse(path)
            .ok()
            .and_then(|path| self.files.get(&path))
            .map(Vec::as_slice)
    }

    /// All paths currently stored, sorted.
    #[must_use]
    pub fn paths(&self) -> Vec<String> {
        self.files.keys().map(ToString::to_string).collect()
    }
}

impl LocalStore for InMemoryStore {
    fn list(
        &self,
        algorithm: HashAlgorithm,
        cancel: &CancelToken,
    ) -> Result<Vec<LocalFile>, Error> {
        let hasher = Sha256Hasher;
        let mut listed = Vec::with_capacity(self.files.len());
        for (path, data) in &self.files {
            if cancel.is_cancelled() {
                return Err(Error::new(ErrorKind::Cancelled, "cancelled"));
            }
            listed.push(LocalFile {
                path: path.clone(),
                size: data.len() as u64,
                hash: Some(hasher.hash(algorithm, data)),
            });
        }
        Ok(listed)
    }

    fn begin<'a>(
        &'a mut self,
        path: &RelativePath,
        _expected_size: u64,
    ) -> Result<Box<dyn StagedFile + 'a>, Error> {
        let fail_after = self.fail_staging_after;
        Ok(Box::new(InMemoryStaged {
            store: self,
            path: path.clone(),
            buffer: Vec::new(),
            fail_after,
        }))
    }

    fn write(&mut self, path: &RelativePath, data: &[u8]) -> Result<(), Error> {
        if self.fail_writes_to.as_ref() == Some(path) {
            // Old content stays intact, as the port contract requires.
            return Err(Error::new(
                ErrorKind::FileSystem,
                format!("cannot write {path}"),
            ));
        }
        self.files.insert(path.clone(), data.to_vec());
        Ok(())
    }

    fn delete(&mut self, path: &RelativePath) -> Result<(), Error> {
        self.files
            .remove(path)
            .map(|_| ())
            .ok_or_else(|| Error::new(ErrorKind::FileSystem, format!("not found: {path}")))
    }
}

/// A staged file of the in-memory store: bytes collect in a private buffer; only `commit` changes the store, and
/// dropping the handle discards everything.
struct InMemoryStaged<'a> {
    store: &'a mut InMemoryStore,
    path: RelativePath,
    buffer: Vec<u8>,
    fail_after: Option<usize>,
}

impl Write for InMemoryStaged<'_> {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        if let Some(limit) = self.fail_after {
            if self.buffer.len() + data.len() > limit {
                return Err(io::Error::other("no space left"));
            }
        }
        self.buffer.extend_from_slice(data);
        Ok(data.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl StagedFile for InMemoryStaged<'_> {
    fn commit(self: Box<Self>) -> Result<(), Error> {
        let this = *self;
        this.store.write(&this.path, &this.buffer)
    }
}

/// Collects events in memory.
#[derive(Debug, Clone, Default)]
pub struct MemoryEventSink {
    /// Events in delivery order.
    pub events: Vec<OperationEvent>,
}

impl EventSink for MemoryEventSink {
    fn emit(&mut self, event: OperationEvent) {
        self.events.push(event);
    }
}

#[cfg(test)]
mod tests {
    use super::{InMemoryRepository, InMemoryStore, Sha256Hasher};
    use armasync_core::{
        CancelToken, ErrorKind, HashAlgorithm, Hasher, LocalStore, RelativePath, RepositorySource,
    };

    #[test]
    fn sha256_matches_known_vectors() {
        let hasher = Sha256Hasher;
        assert_eq!(
            hasher.hash(HashAlgorithm::Sha256, b"").hex(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            hasher.hash(HashAlgorithm::Sha256, b"abc").hex(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn repository_manifest_matches_content_and_rejects_bad_paths() {
        let repo = InMemoryRepository::new("r1", &[("a.pbo", b"abc".as_slice())]).unwrap();
        let manifest = repo.manifest().unwrap();
        assert_eq!(manifest.entries()[0].size, 3);
        assert_eq!(
            repo.read(&RelativePath::parse("a.pbo").unwrap()).unwrap(),
            b"abc"
        );
        assert_eq!(
            InMemoryRepository::new("r", &[("../x", b"".as_slice())])
                .unwrap_err()
                .kind(),
            ErrorKind::Format
        );
    }

    #[test]
    fn store_lists_hashes_and_failed_write_keeps_old_content() {
        let mut store = InMemoryStore::new().with_file("a.pbo", b"old").unwrap();
        let listed = store
            .list(HashAlgorithm::Sha1, &CancelToken::new())
            .unwrap();
        assert_eq!(listed[0].size, 3);
        assert_eq!(
            listed[0].hash.as_ref().unwrap().algorithm(),
            HashAlgorithm::Sha1
        );
        let path = RelativePath::parse("a.pbo").unwrap();
        store.fail_writes_to(&path);
        assert_eq!(
            store.write(&path, b"new").unwrap_err().kind(),
            ErrorKind::FileSystem
        );
        assert_eq!(store.content("a.pbo"), Some(b"old".as_slice()));
    }
}
