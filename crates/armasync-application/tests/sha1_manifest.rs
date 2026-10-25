// SPDX-License-Identifier: AGPL-3.0-only
//! The use cases verify downloads with the manifest's own hash algorithm: repositories of existing servers publish
//! SHA-1 (ADR-0006), so a SHA-1 manifest must verify, and damaged data must still be refused.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;

use armasync_adapters::{InMemoryStore, MemoryEventSink, Sha256Hasher};
use armasync_application::sync::{SyncContext, apply, plan};
use armasync_core::{
    CancelToken, Error, ErrorKind, FileEntry, HashAlgorithm, Hasher, Manifest, OperationId,
    PlanOptions, RelativePath, RepositorySource,
};

struct Sha1Repository {
    files: BTreeMap<RelativePath, Vec<u8>>,
    served: BTreeMap<RelativePath, Vec<u8>>,
}

impl Sha1Repository {
    fn new(files: &[(&str, &[u8])]) -> Self {
        let map: BTreeMap<_, _> = files
            .iter()
            .map(|(p, d)| (RelativePath::parse(p).unwrap(), d.to_vec()))
            .collect();
        Self {
            files: map.clone(),
            served: map,
        }
    }
}

impl RepositorySource for Sha1Repository {
    fn manifest(&self) -> Result<Manifest, Error> {
        let entries = self
            .files
            .iter()
            .map(|(path, data)| FileEntry {
                path: path.clone(),
                size: data.len() as u64,
                hash: Sha256Hasher.hash(HashAlgorithm::Sha1, data),
            })
            .collect();
        Manifest::new("1", entries).map_err(|e| Error::new(ErrorKind::Format, e.to_string()))
    }

    fn read(&self, path: &RelativePath) -> Result<Vec<u8>, Error> {
        self.served
            .get(path)
            .cloned()
            .ok_or_else(|| Error::new(ErrorKind::Network, "missing"))
    }
}

fn run(source: &Sha1Repository, store: &mut InMemoryStore) -> Result<(), Error> {
    let options = PlanOptions::default();
    let planned = plan(source, store, &options)?;
    let mut sink = MemoryEventSink::default();
    let cancel = CancelToken::new();
    let hasher = Sha256Hasher;
    let mut context = SyncContext {
        source,
        store,
        hasher: &hasher,
        unpacker: &armasync_adapters::archive::ZipUnpacker,
        sink: &mut sink,
        cancel: &cancel,
    };
    apply(OperationId::new(1), &planned, &options, &mut context).map(|_| ())
}

#[test]
fn sha1_manifest_installs_and_a_repeat_is_a_noop() {
    let source = Sha1Repository::new(&[("@m/a.pbo", b"alpha"), ("@m/b.pbo", b"beta")]);
    let mut store = InMemoryStore::new();
    run(&source, &mut store).unwrap();
    assert_eq!(store.content("@m/a.pbo"), Some(b"alpha".as_slice()));
    // second run: planning sees identical files (local hashes are SHA-256 in memory, so they are re-fetched and
    // verified again, never trusted blindly) and still succeeds
    run(&source, &mut store).unwrap();
    assert_eq!(store.content("@m/b.pbo"), Some(b"beta".as_slice()));
}

#[test]
fn damaged_data_is_refused_under_sha1_and_the_old_file_stays() {
    let mut source = Sha1Repository::new(&[("@m/a.pbo", b"alpha")]);
    let mut store = InMemoryStore::new().with_file("@m/a.pbo", b"old").unwrap();
    source
        .served
        .insert(RelativePath::parse("@m/a.pbo").unwrap(), b"ALPHA".to_vec());
    let error = run(&source, &mut store).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Integrity);
    assert_eq!(store.content("@m/a.pbo"), Some(b"old".as_slice()));
}
