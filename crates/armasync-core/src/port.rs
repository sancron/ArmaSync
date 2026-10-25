// SPDX-License-Identifier: AGPL-3.0-only
//! Ports: what the application layer needs from the outside world. Adapters implement them.

use std::io::{Read, Seek, Write};
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
    /// The default implementation streams the file through [`RepositorySource::open`] from the start.
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
        let mut opened = self.open(path, expected_size, 0, None)?;
        let written = copy_body(&mut *opened.body, expected_size, out, cancel, progress)?;
        if written != expected_size {
            return Err(Error::new(
                ErrorKind::Integrity,
                "the source delivers less data than announced",
            ));
        }
        Ok(written)
    }

    /// Stable name of this repository. It is part of the key of resumable partial files (MIG-032R, ADR-0009). An
    /// empty name means the source cannot be resumed.
    fn identity(&self) -> &str {
        ""
    }

    /// Opens one file of the current revision, starting at `offset` when that is possible (MIG-032R, ADR-0009).
    ///
    /// * `expected_size` is the announced size of the whole file (it bounds the time allowed for the body).
    /// * `offset == 0`: the whole file; `if_range` is ignored.
    /// * `offset > 0` with `if_range = Some(validator)`: the source may answer with the bytes from `offset` on
    ///   (`start == offset`) **only** if the file is still the one the validator names; any other answer must be
    ///   `start == 0` with the whole file, or a refusal with [`ErrorKind::Integrity`] ("resume refused").
    /// * `validator` is the strong validator of this answer, if the source has one.
    ///
    /// The body delivers exactly `expected_size - start` bytes; the caller checks length and hash. The default
    /// implementation has no range support: it reads the whole file and answers with `start == 0`.
    ///
    /// # Errors
    /// [`ErrorKind::Network`] for transport failures, [`ErrorKind::Integrity`] for a refused resume.
    fn open(
        &self,
        path: &RelativePath,
        expected_size: u64,
        offset: u64,
        if_range: Option<&str>,
    ) -> Result<Opened, Error> {
        let _ = (expected_size, offset, if_range);
        let data = self.read(path)?;
        Ok(Opened {
            start: 0,
            validator: None,
            body: Box::new(std::io::Cursor::new(data)),
        })
    }
}

/// An answer of [`RepositorySource::open`]: where the body starts, its validator and the body itself.
pub struct Opened {
    /// First byte the body delivers: the requested offset (resumed) or 0 (whole file).
    pub start: u64,
    /// Strong validator of this answer, if the source sent one. Only a strong validator may be stored for resume.
    pub validator: Option<String>,
    /// The bytes from `start` on.
    pub body: Box<dyn Read>,
}

impl std::fmt::Debug for Opened {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Opened")
            .field("start", &self.start)
            .field("validator", &self.validator)
            .finish_non_exhaustive()
    }
}

/// Whether `value` is a **strong** entity tag that may serve as a resume validator (ADR-0009): quoted, without the
/// weak prefix `W/`, visible ASCII only (spaces allowed inside the quotes), at most 256 bytes.
#[must_use]
pub fn is_strong_validator(value: &str) -> bool {
    value.len() >= 2
        && value.len() <= 256
        && value.starts_with('"')
        && value.ends_with('"')
        && value[1..value.len() - 1]
            .bytes()
            .all(|b| b == b'"' || (0x20..=0x7e).contains(&b))
        && !value[1..value.len() - 1].contains('"')
}

/// Copies at most `expected` bytes from `body` to `out` in chunks of at most [`TRANSFER_CHUNK`]; `cancel` is
/// checked before each chunk, `progress` is called after each chunk. Returns the number of bytes copied: equal to
/// `expected` on success, less if the body ends early (the caller decides how to handle that).
///
/// # Errors
/// [`ErrorKind::Cancelled`]; [`ErrorKind::Integrity`] if the body holds more data than `expected`;
/// [`ErrorKind::Network`] if reading the body fails; [`ErrorKind::FileSystem`] if `out` fails.
pub fn copy_body(
    body: &mut dyn Read,
    expected: u64,
    out: &mut dyn Write,
    cancel: &CancelToken,
    progress: &mut dyn FnMut(u64),
) -> Result<u64, Error> {
    let mut buffer = vec![0u8; TRANSFER_CHUNK];
    let mut copied: u64 = 0;
    while copied < expected {
        if cancel.is_cancelled() {
            return Err(Error::new(ErrorKind::Cancelled, "cancelled"));
        }
        let want =
            usize::try_from(expected - copied).map_or(TRANSFER_CHUNK, |w| w.min(TRANSFER_CHUNK));
        let read = match body.read(&mut buffer[..want]) {
            Ok(0) => return Ok(copied),
            Ok(read) => read,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => {
                return Err(Error::new(
                    ErrorKind::Network,
                    "the connection was lost during the transfer",
                ));
            }
        };
        out.write_all(&buffer[..read])
            .map_err(|_| Error::new(ErrorKind::FileSystem, "cannot write the received data"))?;
        copied += read as u64;
        progress(read as u64);
    }
    // The announced size is reached: anything more is a protocol violation.
    let mut probe = [0u8; 1];
    loop {
        match body.read(&mut probe) {
            Ok(0) => return Ok(copied),
            Ok(_) => {
                return Err(Error::new(
                    ErrorKind::Integrity,
                    "the source delivers more data than announced",
                ));
            }
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
            Err(_) => {
                return Err(Error::new(
                    ErrorKind::Network,
                    "the connection was lost during the transfer",
                ));
            }
        }
    }
}

/// Identity of a resumable transfer: one file of one repository at one announced size (ADR-0009).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PartialKey {
    /// [`RepositorySource::identity`] of the source. Never empty for a resumable transfer.
    pub source: String,
    /// The transferred object: the file itself, or its packed copy `<file>.zip` (ADR-0008).
    pub path: RelativePath,
    /// The size the object is announced with.
    pub size: u64,
}

/// What an earlier run left behind for a [`PartialKey`]: the valid bytes and the validator they belong to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredPartial {
    /// Number of valid bytes at the start of the partial file (`0 < offset < size`).
    pub offset: u64,
    /// The strong validator of the answer these bytes came from.
    pub validator: String,
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

    /// Creates a temporary read/write file for a packed archive (ADR-0008). It holds the complete archive while it is
    /// unpacked; it lives on the destination's file system, is invisible to [`LocalStore::list`] and is removed when
    /// the handle is dropped. Its name never collides with a repository file.
    ///
    /// # Errors
    /// File-system failures.
    fn spool(&mut self, expected_size: u64) -> Result<Box<dyn Spool>, Error>;

    /// Free bytes available to the sync on the file system of the destination, checked before a run starts
    /// (MIG-032B). `Ok(None)` means the value is not known for this store or platform: the run then goes ahead
    /// without the advance check, and a full disk still fails on write. The value is advisory only; other processes
    /// may use the space after the check.
    ///
    /// # Errors
    /// Only if the store itself is broken; a missing value is `Ok(None)`, never an error.
    fn free_space(&self) -> Result<Option<u64>, Error> {
        Ok(None)
    }

    /// Opens the resumable partial file for `key` (MIG-032R, ADR-0009). It holds the transferred object while it
    /// arrives, possibly over several runs. It is never visible under its final name and never listed.
    ///
    /// Contract:
    /// * opening reads the state of an earlier run; [`PartialFile::stored`] reports it only if it matches `key`
    ///   exactly and carries a strong validator, otherwise the transfer starts from zero;
    /// * the handle decides the fate of the bytes: [`PartialFile::keep`] (also on drop) leaves them for a later run,
    ///   [`PartialFile::discard`] removes them, [`PartialFile::commit_to`] makes them the target;
    /// * requires the exclusive lock of the destination, like every change.
    ///
    /// # Errors
    /// File-system failures.
    fn partial(&mut self, key: &PartialKey) -> Result<Box<dyn PartialFile + '_>, Error>;
}

/// A temporary file that holds a packed archive (see [`LocalStore::spool`]). The archive is written first, then read
/// back from the start.
pub trait Spool: Read + Write + Seek {}

impl<T: Read + Write + Seek> Spool for T {}

/// A resumable partial file (MIG-032R, ADR-0009). Writes go to the current position; reads serve the bytes that were
/// written. The file is only ever moved to a target by [`PartialFile::commit_to`], after the caller verified the
/// complete content.
pub trait PartialFile: Read + Write + Seek {
    /// The state an earlier run left for this key, if it can be continued (see [`LocalStore::partial`]). Fixed when the
    /// file was opened.
    fn stored(&self) -> Option<StoredPartial>;

    /// Empties the file and stores `validator` for the bytes that follow (`None`: nothing can be resumed later).
    ///
    /// # Errors
    /// File-system failures.
    fn restart(&mut self, validator: Option<&str>) -> Result<(), Error>;

    /// Leaves the bytes and their state on disk for a later run. Without a stored validator the bytes are removed
    /// instead, because they could never be continued safely.
    fn keep(self: Box<Self>);

    /// Removes the bytes and their state at once (cancellation, integrity failure, end of a verified archive).
    fn discard(self: Box<Self>);

    /// Makes the complete file the content of `target`, after checking that the installed content has the hash
    /// `expected` (the verified checksum of the manifest). The swap is by name, so the check runs after it.
    ///
    /// Outcomes (MIG-032R2):
    /// * before the swap fails (flush, target check, rename): `target` keeps its old content and the bytes are removed;
    /// * the swap succeeded but the installed content does not match `expected`: the unverified file is removed and
    ///   [`ErrorKind::Integrity`] is returned; **the previous content of `target` is lost** by then. If that removal
    ///   fails too, [`ErrorKind::FileSystem`] is returned and the message says that an unverified file may remain;
    /// * success: the state is removed and `target` has the verified content.
    ///
    /// # Errors
    /// [`ErrorKind::FileSystem`], [`ErrorKind::Integrity`].
    fn commit_to(
        self: Box<Self>,
        target: &RelativePath,
        expected: &ContentHash,
    ) -> Result<(), Error>;
}

/// Unpacks a packed archive (ADR-0008). The core only states the contract; the archive format lives in an adapter.
///
/// Contract:
/// * the archive must contain **exactly one** regular file; folders, links, encrypted entries and unsupported
///   compression methods are refused; the entry name is validated as a [`RelativePath`] but never used as the
///   destination path (the manifest decides where the file goes);
/// * the entry must announce exactly `expected_size` bytes; never more than `expected_size` bytes are written, and a
///   longer or shorter entry fails with [`ErrorKind::Integrity`] or [`ErrorKind::Format`];
/// * a damaged archive (checksum of the entry, structure, truncation) fails; nothing else is written after the first
///   error;
/// * `cancel` is checked before and during writing ([`ErrorKind::Cancelled`]);
/// * returns the number of bytes written, equal to `expected_size` on success. The caller still verifies the hash.
pub trait ArchiveReader {
    /// Writes the unpacked file of `archive` to `out`.
    ///
    /// # Errors
    /// See the contract above.
    fn extract_single(
        &self,
        archive: &mut dyn Spool,
        expected_size: u64,
        out: &mut dyn Write,
        cancel: &CancelToken,
    ) -> Result<u64, Error>;
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

/// A local file that can be uploaded: its safe relative path and its size in bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFile {
    /// Relative path below the source root.
    pub path: RelativePath,
    /// Size in bytes at listing time.
    pub size: u64,
}

/// A local file opened for an upload (ADR-0011).
pub struct OpenedSource {
    /// Size in bytes at the time the file was opened.
    pub size: u64,
    /// Reader over the whole content; it is read from the first byte.
    pub content: Box<dyn Read + Send>,
}

impl std::fmt::Debug for OpenedSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenedSource")
            .field("size", &self.size)
            .finish_non_exhaustive()
    }
}

/// Local files an upload reads from (ADR-0011). Implementations never list or open links, junctions or other
/// non-regular files, and they never leave their root.
pub trait LocalSource {
    /// All regular files below the source root, sorted by path. `cancel` is checked between files.
    ///
    /// # Errors
    /// File-system failures; [`ErrorKind::Cancelled`].
    fn files(&self, cancel: &CancelToken) -> Result<Vec<SourceFile>, Error>;

    /// Opens one regular file for reading.
    ///
    /// # Errors
    /// [`ErrorKind::FileSystem`] for missing files, links or parents that are links, and other file-system failures.
    fn open(&self, path: &RelativePath) -> Result<OpenedSource, Error>;
}

/// A destination that accepts uploads below a folder tree (ADR-0011). Folders are created one level per call.
pub trait RemoteTarget {
    /// Whether a file or a folder exists at `path`.
    ///
    /// # Errors
    /// Transport and authentication failures.
    fn exists(&self, path: &RelativePath) -> Result<bool, Error>;

    /// Creates one folder; an existing folder is accepted. The parent folder must exist.
    ///
    /// # Errors
    /// Transport failures; [`ErrorKind::FileSystem`] when the parent is missing or the target refuses the folder.
    fn make_folder(&self, path: &RelativePath) -> Result<(), Error>;

    /// Replaces the file at `path` with the whole content of `source`. The parent folder must exist. An upload
    /// that is cancelled before its last byte leaves no complete file at `path`, as far as the server guarantees
    /// it; the adapter reports [`ErrorKind::Cancelled`].
    ///
    /// # Errors
    /// [`ErrorKind::Cancelled`] when cancelled; [`ErrorKind::FileSystem`] for refused targets; transport
    /// failures otherwise.
    fn upload(
        &self,
        path: &RelativePath,
        source: OpenedSource,
        cancel: &CancelToken,
    ) -> Result<(), Error>;
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
