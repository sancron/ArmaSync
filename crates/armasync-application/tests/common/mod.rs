// SPDX-License-Identifier: AGPL-3.0-only
//! Shared helpers for the resume tests (MIG-032R, ADR-0009). Everything is synthetic.
#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]

use std::cell::{Cell, RefCell};
use std::io::{self, Read};

use armasync_adapters::{InMemoryRepository, MemoryEventSink, Sha256Hasher};
use armasync_application::sync::{ApplyReport, SyncContext, apply, plan};
use armasync_core::{
    CancelToken, Error, ErrorKind, LocalStore, Manifest, Opened, OperationId, PlanOptions,
    RelativePath, RepositorySource, TRANSFER_CHUNK,
};

pub fn path(text: &str) -> RelativePath {
    RelativePath::parse(text).unwrap()
}

/// Deterministic content of `length` bytes that differs for every `seed`.
pub fn content(length: usize, seed: u8) -> Vec<u8> {
    (0..length)
        .map(|i| (i % 251) as u8 ^ seed.wrapping_mul(37))
        .collect()
}

/// What the wrapper does with requests that carry an offset (a resume).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OnResume {
    /// Behave like the wrapped repository.
    Normal,
    /// Refuse every resume with `Integrity` ("resume refused"), as a server that cannot answer ranges.
    Refuse,
    /// Answer every resume with the right validator but **garbage** bytes of the right length.
    LieAboutBytes,
}

/// A repository for the tests. It records every `open` (offset and `If-Range`), can cut the connection after a
/// number of bytes, can cancel a token after a number of bytes, and can misbehave on resumes.
pub struct Recording {
    inner: InMemoryRepository,
    /// Offset and `If-Range` of every `open` call, in order.
    pub opens: RefCell<Vec<(u64, Option<String>)>>,
    /// Absolute file position at which the connection drops (a read error), if set.
    pub cut_at: Cell<Option<u64>>,
    /// Token to cancel once this many body bytes were delivered.
    pub cancel_after: RefCell<Option<(u64, CancelToken)>>,
    pub on_resume: Cell<OnResume>,
}

impl Recording {
    pub fn new(inner: InMemoryRepository) -> Self {
        Self {
            inner,
            opens: RefCell::new(Vec::new()),
            cut_at: Cell::new(None),
            cancel_after: RefCell::new(None),
            on_resume: Cell::new(OnResume::Normal),
        }
    }

    /// Publishes one file as a new revision (replaces the whole file set).
    pub fn publish(&mut self, revision: &str, files: &[(&str, &[u8])]) {
        let map = files
            .iter()
            .map(|(name, data)| (path(name), data.to_vec()))
            .collect();
        self.inner.publish(revision, map);
    }

    pub fn inner_mut(&mut self) -> &mut InMemoryRepository {
        &mut self.inner
    }
}

/// The body wrapper: drops the connection at `cut` (absolute position) and/or cancels a token after some bytes.
struct Hooked {
    inner: Box<dyn Read>,
    position: u64,
    cut: Option<u64>,
    cancel: Option<(u64, CancelToken)>,
}

impl Read for Hooked {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if let Some(cut) = self.cut {
            if self.position >= cut {
                return Err(io::Error::new(
                    io::ErrorKind::ConnectionReset,
                    "connection lost",
                ));
            }
        }
        let room = match self.cut {
            Some(cut) => {
                usize::try_from(cut - self.position).map_or(buffer.len(), |r| r.min(buffer.len()))
            }
            None => buffer.len(),
        };
        let read = self.inner.read(&mut buffer[..room])?;
        self.position += read as u64;
        if let Some((after, token)) = &self.cancel {
            if self.position >= *after {
                token.cancel();
            }
        }
        Ok(read)
    }
}

/// Garbage of exactly `length` bytes.
struct Garbage {
    left: usize,
}

impl Read for Garbage {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let n = self.left.min(buffer.len());
        buffer[..n].fill(0x5A);
        self.left -= n;
        Ok(n)
    }
}

impl RepositorySource for Recording {
    fn manifest(&self) -> Result<Manifest, Error> {
        self.inner.manifest()
    }

    fn read(&self, path: &RelativePath) -> Result<Vec<u8>, Error> {
        self.inner.read(path)
    }

    fn identity(&self) -> &str {
        self.inner.identity()
    }

    fn open(
        &self,
        path: &RelativePath,
        expected_size: u64,
        offset: u64,
        if_range: Option<&str>,
    ) -> Result<Opened, Error> {
        self.opens
            .borrow_mut()
            .push((offset, if_range.map(str::to_owned)));
        if offset > 0 {
            match self.on_resume.get() {
                OnResume::Normal => {}
                OnResume::Refuse => {
                    return Err(Error::new(
                        ErrorKind::Integrity,
                        "resume refused: the answer does not match the requested range",
                    ));
                }
                OnResume::LieAboutBytes => {
                    return Ok(Opened {
                        start: offset,
                        validator: if_range.map(str::to_owned),
                        body: Box::new(Garbage {
                            left: usize::try_from(expected_size - offset).unwrap(),
                        }),
                    });
                }
            }
        }
        let opened = self.inner.open(path, expected_size, offset, if_range)?;
        let cut = self.cut_at.get();
        let cancel = self.cancel_after.borrow().clone();
        Ok(Opened {
            start: opened.start,
            validator: opened.validator,
            body: Box::new(Hooked {
                inner: opened.body,
                position: opened.start,
                cut,
                cancel,
            }),
        })
    }
}

/// One run of the sync use case against `store`: plan, then apply, with the given cancellation.
pub fn run(
    source: &dyn RepositorySource,
    store: &mut dyn LocalStore,
    cancel: &CancelToken,
) -> Result<ApplyReport, Error> {
    let options = PlanOptions::default();
    let planned = plan(source, &*store, &options)?;
    let hasher = Sha256Hasher;
    let mut sink = MemoryEventSink::default();
    let mut context = SyncContext {
        source,
        store,
        hasher: &hasher,
        unpacker: &armasync_adapters::archive::ZipUnpacker,
        sink: &mut sink,
        cancel,
    };
    apply(OperationId::new(1), &planned, &options, &mut context)
}

/// Enough content for several transfer chunks, so an interrupted transfer really leaves a partial file.
pub fn big_file() -> Vec<u8> {
    content(TRANSFER_CHUNK * 3 + 4321, 1)
}

/// Where the first run is cut in [`big_file`].
pub const CUT: u64 = (TRANSFER_CHUNK + 1000) as u64;
