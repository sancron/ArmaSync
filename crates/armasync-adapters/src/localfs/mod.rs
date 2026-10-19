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
//! Known limits: the check-then-use sequence cannot defend against another process that swaps a folder for a link
//! while a transfer runs (the destination is assumed to be writable only by the user); free disk space is not
//! checked up front (a full disk fails the write, and the temporary file is removed); the same destination must
//! not be synchronised by two processes at once (no lock yet).

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

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
}

impl LocalDirectory {
    /// The destination folder `root`, which must exist and be a real folder (not a link).
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
        Ok(Self { root })
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
    fn secure_target(&self, path: &RelativePath, create: bool) -> Result<PathBuf, Error> {
        let relative = path.as_str();
        let mut current = self.root.clone();
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
        Ok(target)
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
                remove_if_stale(&entry.path(), &metadata);
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
        if relative.split('/').any(is_staging_name) {
            return Err(refused(
                relative,
                "the name is reserved for temporary files",
            ));
        }
        let target = self.secure_target(path, true)?;
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
        Ok(Box::new(Staged {
            store: self,
            relative: relative.to_owned(),
            path: path.clone(),
            staging,
            target,
            file: Some(file),
            done: false,
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
        let target = self.secure_target(path, false)?;
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
        self.store.secure_target(&self.path, false)?;
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
