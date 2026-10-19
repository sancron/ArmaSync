// SPDX-License-Identifier: AGPL-3.0-only
//! The destination folder on the local file system (MIG-030C, ADR-0007).
//!
//! [`LocalDirectory`] implements the core's [`LocalStore`]:
//! * **scan** (`list`): walks the folder, hashes regular files in chunks with the requested algorithm, can be
//!   cancelled, and never follows links;
//! * **staging** (`begin`/`commit`): new content is written to a temporary file **next to the target** (same file
//!   system), flushed to stable storage and then moved over the target in one step; until then the target keeps its
//!   old, complete content; dropping the handle removes the temporary file;
//! * **delete**: removes one regular file.
//!
//! Safety rules enforced here:
//! * every access stays inside the root: a path whose existing parent folder (or the file itself) is a symbolic
//!   link, a junction or any other reparse point is **refused**, and the scan does not descend into links;
//! * only regular files are listed, replaced and deleted (never folders, devices or links);
//! * temporary files carry the reserved prefix [`STAGING_PREFIX`]; they are never listed, a repository file with
//!   such a name is refused, and leftovers older than [`STALE_STAGING_AGE`] are removed during a scan;
//! * names that cannot be represented as a safe [`RelativePath`] (for example unusual bytes on Linux) are not
//!   listed and therefore never touched or deleted;
//! * error messages name the relative path, never the absolute location.
//!
//! Concurrency (MIG-030H, `docs/migration/LOCAL_FS_THREATS.md`):
//! * **One writer per destination.** Changing operations (`begin`, `write`, `delete`) need a handle from
//!   [`LocalDirectory::open_exclusive`], which holds an operating-system file lock (`flock`/`LockFileEx` on
//!   `.armasync-lock` in the destination) until it is dropped; a second run fails at once with a clear error. The
//!   lock disappears with the process, also after a crash. [`LocalDirectory::new`] is read-only (scan only).
//! * **Folders cannot be swapped for links while in use (Windows).** Every folder on the way to a target is opened
//!   as a handle that forbids renaming or deleting it for as long as the staged file exists, after it was verified
//!   not to be a link or reparse point; a competing process therefore cannot replace a verified folder.
//! * **Unix:** the standard library offers no race-free directory-relative access, so the folder check is
//!   check-then-use there, backed by a canonical-path check before and after the temporary file is created and
//!   before it replaces the target. A *local attacker who can write to the destination while a run is in progress*
//!   can still win this race on Unix. This is a documented residual risk and a release blocker for any claim of
//!   a race-free path boundary on Linux until a capability-based implementation (`openat`) has been evaluated.
//!
//! Known limits: free disk space is not checked up front (a full disk fails the write, and the temporary file is
//! removed).

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use fs4::FileExt;

use armasync_core::{
    CancelToken, ContentHash, Digest, Error, ErrorKind, HashAlgorithm, Hasher, LocalFile,
    LocalStore, RelativePath, StagedFile, TRANSFER_CHUNK,
};

use crate::memory::Sha256Hasher;

/// Prefix of temporary staging files; names with this prefix belong to the adapter.
pub const STAGING_PREFIX: &str = ".armasync-";
const STAGING_SUFFIX: &str = ".tmp";
/// Staging leftovers older than this are removed by a scan (a live transfer is never that old).
pub const STALE_STAGING_AGE: Duration = Duration::from_secs(60 * 60);

/// Name of the lock file in the destination (reserved prefix, never listed).
const LOCK_NAME: &str = ".armasync-lock";

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn fs_error(action: &str, relative: &str, error: &io::Error) -> Error {
    Error::new(
        ErrorKind::FileSystem,
        format!("cannot {action} '{relative}': {error}"),
    )
}

fn refused(relative: &str, why: &str) -> Error {
    Error::new(
        ErrorKind::FileSystem,
        format!("'{relative}' is refused: {why}"),
    )
}

fn is_staging_name(name: &str) -> bool {
    name.starts_with(STAGING_PREFIX) && name.ends_with(STAGING_SUFFIX)
}

/// Whether `metadata` describes a link or any other reparse point (never followed).
fn is_link(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return true;
        }
    }
    false
}

/// The destination folder.
#[derive(Debug, Clone)]
pub struct LocalDirectory {
    root: PathBuf,
    /// The held exclusive lock (`None` for a read-only handle). Released when the last clone is dropped.
    lock: Option<Arc<File>>,
}

/// Handles that keep verified folders from being renamed or deleted (Windows); empty elsewhere.
type Pins = Vec<File>;

impl LocalDirectory {
    /// A **read-only** handle: [`LocalStore::list`] works, changing operations are refused. The destination
    /// folder `root` must exist and be a real folder (not a link).
    ///
    /// # Errors
    /// [`ErrorKind::FileSystem`] if it does not exist or is not a folder.
    pub fn new(root: impl AsRef<Path>) -> Result<Self, Error> {
        let root = root.as_ref();
        let metadata = fs::symlink_metadata(root).map_err(|e| {
            Error::new(
                ErrorKind::FileSystem,
                format!("the destination folder cannot be used: {e}"),
            )
        })?;
        if !metadata.is_dir() || is_link(&metadata) {
            return Err(Error::new(
                ErrorKind::FileSystem,
                "the destination is not a folder (or is a link)",
            ));
        }
        let root = fs::canonicalize(root).map_err(|e| {
            Error::new(
                ErrorKind::FileSystem,
                format!("the destination folder cannot be resolved: {e}"),
            )
        })?;
        Ok(Self { root, lock: None })
    }

    /// A handle that may change the destination and holds the exclusive lock until it is dropped. Use it for the
    /// whole run (planning, transfers and deletions).
    ///
    /// # Errors
    /// [`ErrorKind::Config`] if another run already holds the lock; [`ErrorKind::FileSystem`] if the destination is
    /// unusable or the lock file cannot be created.
    pub fn open_exclusive(root: impl AsRef<Path>) -> Result<Self, Error> {
        let mut directory = Self::new(root)?;
        let lock_path = directory.root.join(LOCK_NAME);
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)
            .map_err(|e| {
                Error::new(
                    ErrorKind::FileSystem,
                    format!("cannot create the lock file in the destination: {e}"),
                )
            })?;
        if let Ok(metadata) = fs::symlink_metadata(&lock_path) {
            if is_link(&metadata) || !metadata.is_file() {
                return Err(refused(LOCK_NAME, "the lock file is not a regular file"));
            }
        }
        FileExt::try_lock(&file).map_err(|_| {
            Error::new(
                ErrorKind::Config,
                "another ArmaSync run is using this destination folder; wait for it to finish",
            )
        })?;
        directory.lock = Some(Arc::new(file));
        Ok(directory)
    }

    fn require_lock(&self) -> Result<(), Error> {
        if self.lock.is_some() {
            Ok(())
        } else {
            Err(Error::new(
                ErrorKind::Config,
                "the destination was opened read-only; changes need an exclusive handle",
            ))
        }
    }

    /// The folder must still resolve to a place inside the root (guards against links that were swapped in).
    fn ensure_inside(&self, directory: &Path, relative: &str) -> Result<(), Error> {
        let resolved = fs::canonicalize(directory)
            .map_err(|e| fs_error("resolve the folder of", relative, &e))?;
        if resolved.starts_with(&self.root) {
            Ok(())
        } else {
            Err(refused(
                relative,
                "the folder resolves to a place outside the destination",
            ))
        }
    }

    fn join(&self, path: &RelativePath) -> PathBuf {
        let mut full = self.root.clone();
        for part in path.as_str().split('/') {
            full.push(part);
        }
        full
    }

    /// Checks that every existing folder on the way to `path` is a real folder and that `path` itself, if it
    /// exists, is a regular file. Creates missing parent folders when `create` is set (one at a time, re-checked).
    fn secure_target(&self, path: &RelativePath, create: bool) -> Result<(PathBuf, Pins), Error> {
        let relative = path.as_str();
        let mut current = self.root.clone();
        let mut pins: Pins = Vec::new();
        pin_folder(&current, relative, &mut pins)?;
        let parts: Vec<&str> = relative.split('/').collect();
        for part in &parts[..parts.len() - 1] {
            current.push(part);
            match fs::symlink_metadata(&current) {
                Ok(metadata) => {
                    if is_link(&metadata) {
                        return Err(refused(relative, "a parent folder is a link"));
                    }
                    if !metadata.is_dir() {
                        return Err(refused(relative, "a parent is not a folder"));
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::NotFound && create => {
                    fs::create_dir(&current)
                        .map_err(|e| fs_error("create a folder for", relative, &e))?;
                    let made = fs::symlink_metadata(&current)
                        .map_err(|e| fs_error("inspect a folder for", relative, &e))?;
                    if is_link(&made) || !made.is_dir() {
                        return Err(refused(relative, "a created folder was replaced"));
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                Err(e) => return Err(fs_error("inspect", relative, &e)),
            }
            if current.exists() {
                pin_folder(&current, relative, &mut pins)?;
            }
        }
        let target = self.join(path);
        if let Ok(metadata) = fs::symlink_metadata(&target) {
            if is_link(&metadata) {
                return Err(refused(relative, "the file is a link"));
            }
            if !metadata.is_file() {
                return Err(refused(relative, "it is not a regular file"));
            }
        }
        if let Some(parent) = target.parent() {
            if parent.exists() {
                self.ensure_inside(parent, relative)?;
            }
        }
        Ok((target, pins))
    }

    fn scan(
        &self,
        directory: &Path,
        prefix: &str,
        algorithm: HashAlgorithm,
        cancel: &CancelToken,
        out: &mut Vec<LocalFile>,
    ) -> Result<(), Error> {
        let entries = fs::read_dir(directory).map_err(|e| {
            fs_error(
                "read the folder",
                if prefix.is_empty() { "." } else { prefix },
                &e,
            )
        })?;
        for entry in entries {
            if cancel.is_cancelled() {
                return Err(Error::new(ErrorKind::Cancelled, "cancelled"));
            }
            let Ok(entry) = entry else { continue };
            let file_name = entry.file_name();
            let Some(name) = file_name.to_str() else {
                continue;
            };
            let Ok(metadata) = fs::symlink_metadata(entry.path()) else {
                continue;
            };
            if is_staging_name(name) {
                if self.lock.is_some() {
                    // We hold the exclusive lock, so no other run can own this file: it is a leftover.
                    if metadata.is_file() {
                        let _ = fs::remove_file(entry.path());
                    }
                } else {
                    remove_if_stale(&entry.path(), &metadata);
                }
                continue;
            }
            if name.starts_with(STAGING_PREFIX) {
                continue;
            }
            if is_link(&metadata) {
                continue;
            }
            let relative = format!("{prefix}{name}");
            if metadata.is_dir() {
                self.scan(
                    &entry.path(),
                    &format!("{relative}/"),
                    algorithm,
                    cancel,
                    out,
                )?;
            } else if metadata.is_file() {
                let Ok(path) = RelativePath::parse(&relative) else {
                    continue;
                };
                let hash = hash_file(&entry.path(), algorithm, cancel)?;
                out.push(LocalFile {
                    path,
                    size: metadata.len(),
                    hash,
                });
            }
        }
        Ok(())
    }
}

/// Verifies that `directory` is a real folder (not a link or reparse point) and, on Windows, keeps it open in a way
/// that forbids renaming or deleting it until the handle is dropped. A no-op check elsewhere (see the module notes).
#[cfg(windows)]
fn pin_folder(directory: &Path, relative: &str, pins: &mut Pins) -> Result<(), Error> {
    use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
    const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
    const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    const SHARE_READ_WRITE: u32 = 0x1 | 0x2; // not FILE_SHARE_DELETE
    let handle = OpenOptions::new()
        .read(true)
        .share_mode(SHARE_READ_WRITE)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
        .open(directory)
        .map_err(|e| fs_error("pin a folder of", relative, &e))?;
    let metadata = handle
        .metadata()
        .map_err(|e| fs_error("inspect a folder of", relative, &e))?;
    if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 || !metadata.is_dir() {
        return Err(refused(relative, "a parent folder is a link"));
    }
    pins.push(handle);
    Ok(())
}

#[cfg(not(windows))]
fn pin_folder(_directory: &Path, _relative: &str, _pins: &mut Pins) -> Result<(), Error> {
    Ok(())
}

fn remove_if_stale(path: &Path, metadata: &fs::Metadata) {
    let age = metadata
        .modified()
        .ok()
        .and_then(|modified| SystemTime::now().duration_since(modified).ok());
    if metadata.is_file() && age.is_some_and(|age| age >= STALE_STAGING_AGE) {
        let _ = fs::remove_file(path);
    }
}

/// Hashes a file in chunks. `Ok(None)` if it cannot be opened or read (for example locked by another program);
/// the caller then treats the file as changed. Cancellation is an error.
fn hash_file(
    path: &Path,
    algorithm: HashAlgorithm,
    cancel: &CancelToken,
) -> Result<Option<ContentHash>, Error> {
    let Ok(mut file) = File::open(path) else {
        return Ok(None);
    };
    let mut digest: Box<dyn Digest> = Sha256Hasher.digest(algorithm);
    let mut buffer = vec![0u8; TRANSFER_CHUNK];
    loop {
        if cancel.is_cancelled() {
            return Err(Error::new(ErrorKind::Cancelled, "cancelled"));
        }
        match file.read(&mut buffer) {
            Ok(0) => return Ok(Some(digest.finish())),
            Ok(n) => digest.update(&buffer[..n]),
            Err(_) => return Ok(None),
        }
    }
}

impl LocalStore for LocalDirectory {
    fn list(
        &self,
        algorithm: HashAlgorithm,
        cancel: &CancelToken,
    ) -> Result<Vec<LocalFile>, Error> {
        let mut files = Vec::new();
        self.scan(&self.root, "", algorithm, cancel, &mut files)?;
        files.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(files)
    }

    fn begin<'a>(
        &'a mut self,
        path: &RelativePath,
        _expected_size: u64,
    ) -> Result<Box<dyn StagedFile + 'a>, Error> {
        let relative = path.as_str();
        self.require_lock()?;
        if relative
            .split('/')
            .any(|part| part.starts_with(STAGING_PREFIX))
        {
            return Err(refused(relative, "the name is reserved for the adapter"));
        }
        let (target, pins) = self.secure_target(path, true)?;
        let directory = target
            .parent()
            .map(Path::to_path_buf)
            .ok_or_else(|| refused(relative, "no parent folder"))?;
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        let staging = directory.join(format!(
            "{STAGING_PREFIX}{}-{}-{nanos}{STAGING_SUFFIX}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&staging)
            .map_err(|e| fs_error("create a temporary file for", relative, &e))?;
        if let Err(error) = self.ensure_inside(&directory, relative) {
            drop(file);
            let _ = fs::remove_file(&staging);
            return Err(error);
        }
        Ok(Box::new(Staged {
            store: self,
            relative: relative.to_owned(),
            path: path.clone(),
            staging,
            target,
            file: Some(file),
            done: false,
            _pins: pins,
        }))
    }

    fn write(&mut self, path: &RelativePath, data: &[u8]) -> Result<(), Error> {
        let mut staged = self.begin(path, data.len() as u64)?;
        staged
            .write_all(data)
            .map_err(|e| fs_error("write", path.as_str(), &e))?;
        staged.commit()
    }

    fn delete(&mut self, path: &RelativePath) -> Result<(), Error> {
        let relative = path.as_str();
        self.require_lock()?;
        let (target, _pins) = self.secure_target(path, false)?;
        match fs::symlink_metadata(&target) {
            Ok(_) => fs::remove_file(&target).map_err(|e| fs_error("delete", relative, &e)),
            Err(e) => Err(fs_error("delete", relative, &e)),
        }
    }
}

/// A file being written into staging. Dropping it without `commit` removes the temporary file.
struct Staged<'a> {
    store: &'a LocalDirectory,
    relative: String,
    path: RelativePath,
    staging: PathBuf,
    target: PathBuf,
    file: Option<File>,
    done: bool,
    /// Keeps the verified folders in place until the staged file is committed or dropped.
    _pins: Pins,
}

impl Write for Staged<'_> {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        match self.file.as_mut() {
            Some(file) => file.write(data),
            None => Err(io::Error::other("the staging file is closed")),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self.file.as_mut() {
            Some(file) => file.flush(),
            None => Ok(()),
        }
    }
}

impl StagedFile for Staged<'_> {
    fn commit(mut self: Box<Self>) -> Result<(), Error> {
        let relative = self.relative.clone();
        let file = self
            .file
            .take()
            .ok_or_else(|| refused(&relative, "already committed"))?;
        file.sync_all()
            .map_err(|e| fs_error("flush to disk", &relative, &e))?;
        drop(file);
        // Re-check right before the swap: the target must still be a regular file (or absent) in a real folder.
        let (_, _recheck_pins) = self.store.secure_target(&self.path, false)?;
        fs::rename(&self.staging, &self.target).map_err(|e| fs_error("replace", &relative, &e))?;
        self.done = true;
        #[cfg(unix)]
        if let Some(parent) = self.target.parent() {
            if let Ok(directory) = File::open(parent) {
                let _ = directory.sync_all();
            }
        }
        Ok(())
    }
}

impl Drop for Staged<'_> {
    fn drop(&mut self) {
        if !self.done {
            self.file = None;
            let _ = fs::remove_file(&self.staging);
        }
    }
}
