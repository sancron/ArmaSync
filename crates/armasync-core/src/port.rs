// SPDX-License-Identifier: AGPL-3.0-only
//! Ports: what the application layer needs from the outside world. Adapters implement them.

use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::error::{Error, ErrorKind};
use crate::hash::{ContentHash, HashAlgorithm};
use crate::manifest::Manifest;
use crate::path::RelativePath;
use crate::plan::LocalFile;

/// Read-only source of a repository revision.
pub trait RepositorySource {
    /// The current manifest. May change between calls when the repository is updated.
    ///
    /// # Errors
    /// Network, format or trust failures.
    fn manifest(&self) -> Result<Manifest, Error>;

    /// The full content of one file of the current revision, in memory. Meant for small files and tests; large
    /// files go through [`RepositorySource::fetch`].
    ///
    /// # Errors
    /// Network failures or a missing file.
    fn read(&self, path: &RelativePath) -> Result<Vec<u8>, Error>;

    /// Streams one file of the current revision into `out` without holding the whole file in memory.
    ///
    /// Contract (ADR-0007):
    /// * bytes are written in chunks of at most [`TRANSFER_CHUNK`]; after each chunk `progress` is called with the
    ///   number of bytes just written, and `cancel` is checked **before** each chunk (and before connecting);
    /// * never more than `expected_size` bytes are written; a source that has more fails with
    ///   [`ErrorKind::Integrity`] after exactly `expected_size` bytes; a source that ends early also fails with
    ///   [`ErrorKind::Integrity`];
    /// * cancellation fails with [`ErrorKind::Cancelled`]; a lost connection or bad status with
    ///   [`ErrorKind::Network`] (authentication and trust failures are `Network` as well, per the CLI contract);
    ///   a failing `out` with [`ErrorKind::FileSystem`];
    /// * on **any** error the bytes already written to `out` are an incomplete prefix: the caller discards them.
    ///   The source never retries by re-sending bytes (resume is a later, separate capability);
    /// * returns the number of bytes written, which equals `expected_size` on success.
    ///
    /// The default implementation reads the file with [`RepositorySource::read`] and copies it in chunks; network
    /// adapters override it to stream.
    ///
    /// # Errors
    /// See above.
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
        copy_in_chunks(&data, expected_size, out, cancel, progress)
    }
}

/// Largest number of bytes a transfer writes between progress and cancellation checks.
pub const TRANSFER_CHUNK: usize = 64 * 1024;

/// Copies `data` to `out` following the [`RepositorySource::fetch`] contract. Shared by the default `fetch` and by
/// adapters that already hold the bytes.
///
/// # Errors
/// [`ErrorKind::Cancelled`], [`ErrorKind::Integrity`] (length differs from `expected_size`),
/// [`ErrorKind::FileSystem`] (`out` failed).
pub fn copy_in_chunks(
    data: &[u8],
    expected_size: u64,
    out: &mut dyn Write,
    cancel: &CancelToken,
    progress: &mut dyn FnMut(u64),
) -> Result<u64, Error> {
    let mut written: u64 = 0;
    for chunk in data.chunks(TRANSFER_CHUNK) {
        if cancel.is_cancelled() {
            return Err(Error::new(ErrorKind::Cancelled, "cancelled"));
        }
        let allowed = expected_size.saturating_sub(written);
        let take = usize::try_from(allowed).map_or(chunk.len(), |a| a.min(chunk.len()));
        if take > 0 {
            out.write_all(&chunk[..take])
                .map_err(|_| Error::new(ErrorKind::FileSystem, "cannot write the received data"))?;
            written += take as u64;
            progress(take as u64);
        }
        if take < chunk.len() {
            return Err(Error::new(
                ErrorKind::Integrity,
                "the source delivers more data than announced",
            ));
        }
    }
    if written != expected_size {
        return Err(Error::new(
            ErrorKind::Integrity,
            "the source delivers less data than announced",
        ));
    }
    Ok(written)
}

/// The local destination.
pub trait LocalStore {
    /// All files below the destination root with their size and hash **computed with `algorithm`** (the
    /// algorithm of the manifest the result is compared with; a hash of another algorithm never equals a manifest
    /// hash, so the file would be fetched again). `hash` may be `None` only if the file could not be hashed.
    /// Staging leftovers of this adapter are never listed. `cancel` is checked between files and during hashing;
    /// a cancelled scan fails with [`ErrorKind::Cancelled`].
    ///
    /// # Errors
    /// File-system failures; cancellation.
    fn list(&self, algorithm: HashAlgorithm, cancel: &CancelToken)
    -> Result<Vec<LocalFile>, Error>;

    /// Starts writing `path` into a **staging file** and returns the handle to write to.
    ///
    /// Contract (ADR-0007):
    /// * nothing at `path` changes until [`StagedFile::commit`]; an existing file stays complete and readable while
    ///   the new content is written;
    /// * the staging file lives on the same file system as `path` (so the final step can be an atomic replace) and
    ///   is invisible to [`LocalStore::list`];
    /// * dropping the handle without `commit` removes the staging file (best effort; leftovers from a crash are
    ///   removed by the next `begin`/`list` of the adapter);
    /// * missing parent directories are created at `begin` or `commit`, never outside the destination root; a path
    ///   that would leave the root (links, junctions) is refused with [`ErrorKind::FileSystem`];
    /// * `expected_size` allows checking free space early (a full disk fails here or on write, never at commit).
    ///
    /// # Errors
    /// File-system failures.
    fn begin<'a>(
        &'a mut self,
        path: &RelativePath,
        expected_size: u64,
    ) -> Result<Box<dyn StagedFile + 'a>, Error>;

    /// Writes `data` to `path` **atomically**: afterwards the file has either the old or the complete new content,
    /// never a partial one, even after a crash or cancellation. Missing parent directories are created.
    ///
    /// # Errors
    /// File-system failures; the previous content must remain intact.
    fn write(&mut self, path: &RelativePath, data: &[u8]) -> Result<(), Error>;

    /// Deletes one file below the destination root. Never follows links out of the root.
    ///
    /// # Errors
    /// File-system failures.
    fn delete(&mut self, path: &RelativePath) -> Result<(), Error>;
}

/// A file being written into staging (see [`LocalStore::begin`]). Bytes are written with [`Write`].
pub trait StagedFile: Write {
    /// Makes the staged content the content of the target **atomically**: afterwards the target has either the old
    /// or the complete new content, never a mixture, also after a crash. The content is flushed to stable storage
    /// first. If this fails (for example because the target is locked) the old target is untouched and the staging
    /// file is removed.
    ///
    /// # Errors
    /// [`ErrorKind::FileSystem`].
    fn commit(self: Box<Self>) -> Result<(), Error>;
}

/// An incremental digest (see [`Hasher::digest`]).
pub trait Digest {
    /// Feeds more data.
    fn update(&mut self, data: &[u8]);

    /// The digest of everything fed so far.
    fn finish(self: Box<Self>) -> ContentHash;
}

/// Computes content hashes (kept as a port so the core stays free of crypto dependencies).
///
/// SHA-1 (existing repositories) is an integrity check of the old protocol only, never a trust anchor
/// (ADR-0006).
pub trait Hasher {
    /// Starts an incremental digest.
    fn digest(&self, algorithm: HashAlgorithm) -> Box<dyn Digest>;

    /// Hashes `data` in one go.
    fn hash(&self, algorithm: HashAlgorithm, data: &[u8]) -> ContentHash {
        let mut digest = self.digest(algorithm);
        digest.update(data);
        digest.finish()
    }
}

/// Cooperative cancellation shared between a frontend and a running operation.
#[derive(Debug, Clone, Default)]
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    /// A fresh, not yet cancelled token.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Requests cancellation (idempotent, visible to all clones).
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    /// Whether cancellation was requested.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::CancelToken;

    #[test]
    fn cancellation_is_shared_between_clones() {
        let token = CancelToken::new();
        let clone = token.clone();
        assert!(!clone.is_cancelled());
        token.cancel();
        token.cancel();
        assert!(clone.is_cancelled());
    }
}
