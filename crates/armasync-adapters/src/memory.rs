// SPDX-License-Identifier: AGPL-3.0-only
//! In-memory adapters: a SHA-256 hasher, a repository, a local store and an event sink.
//!
//! They implement the core ports without touching the file system or the network, so use cases can be tested
//! deterministically. They are test and prototype adapters, not a persistence layer.

use std::collections::BTreeMap;

use armasync_core::{
    ContentHash, Error, ErrorKind, EventSink, FileEntry, HashAlgorithm, Hasher, LocalFile,
    LocalStore, Manifest, OperationEvent, RelativePath, RepositorySource,
};
use sha2::{Digest, Sha256};

/// SHA-256 implementation of the [`Hasher`] port.
#[derive(Debug, Clone, Copy, Default)]
pub struct Sha256Hasher;

impl Hasher for Sha256Hasher {
    fn hash(&self, algorithm: HashAlgorithm, data: &[u8]) -> ContentHash {
        match algorithm {
            HashAlgorithm::Sha256 => ContentHash::from_sha256_digest(Sha256::digest(data).into()),
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
            unreachable: false,
        })
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
                hash: hasher.hash(HashAlgorithm::Sha256, data),
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
}

/// A local destination held in memory with atomic writes.
#[derive(Debug, Clone, Default)]
pub struct InMemoryStore {
    files: BTreeMap<RelativePath, Vec<u8>>,
    fail_writes_to: Option<RelativePath>,
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
    fn list(&self) -> Result<Vec<LocalFile>, Error> {
        let hasher = Sha256Hasher;
        Ok(self
            .files
            .iter()
            .map(|(path, data)| LocalFile {
                path: path.clone(),
                size: data.len() as u64,
                hash: Some(hasher.hash(HashAlgorithm::Sha256, data)),
            })
            .collect())
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
        ErrorKind, HashAlgorithm, Hasher, LocalStore, RelativePath, RepositorySource,
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
        assert_eq!(store.list().unwrap()[0].size, 3);
        let path = RelativePath::parse("a.pbo").unwrap();
        store.fail_writes_to(&path);
        assert_eq!(
            store.write(&path, b"new").unwrap_err().kind(),
            ErrorKind::FileSystem
        );
        assert_eq!(store.content("a.pbo"), Some(b"old".as_slice()));
    }
}
