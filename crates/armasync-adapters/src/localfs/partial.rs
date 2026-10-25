// SPDX-License-Identifier: AGPL-3.0-only
//! Resumable partial files of the destination folder (MIG-032R, ADR-0009; hardening MIG-032R2).
//!
//! A partial file holds the bytes of one transfer while it arrives, possibly over several runs:
//! * data in `.armasync-partial-<id>.part` and its state in `.armasync-partial-<id>.state`, both in the folder of the
//!   target, both with the reserved prefix (never listed, never a repository file);
//! * the state records the source (as a digest), the path, the announced size and the **strong** validator of the
//!   answer the bytes came from; a state that does not match the requested key is never used;
//! * [`LocalPartial::commit_to`] is the only way the bytes reach the target, and only after the caller verified the
//!   complete content.
//!
//! Hardening (MIG-032R2): an existing name is **never** opened for writing. A fresh transfer removes the name (which
//! removes only the directory entry, never a link's target) and creates a new file with `create_new`. A resumed
//! transfer copies the valid prefix, read-only, into a new file the adapter created, and renames it over the name.
//! The state is written to a new temporary file and renamed over the state name. Every open of an existing name
//! refuses links and reparse points. On Windows, handles are opened without `FILE_SHARE_DELETE`, so the name cannot be
//! replaced while a handle is open. The link count is not checked: stable Rust cannot read it on Windows (MSRV 1.85).

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use armasync_core::{
    Error, PartialFile, PartialKey, RelativePath, StoredPartial, TRANSFER_CHUNK,
    is_strong_validator,
};
use sha2::{Digest as _, Sha256};

use super::{LocalDirectory, Pins, STAGING_PREFIX, fs_error, is_link, refused};

/// Prefix of the partial data and state files; names with this prefix belong to the adapter.
pub const PARTIAL_PREFIX: &str = ".armasync-partial-";
/// Partial files older than this are removed by a scan (a transfer that long ago cannot be continued).
pub const PARTIAL_MAX_AGE: Duration = Duration::from_secs(7 * 24 * 60 * 60);

const DATA_SUFFIX: &str = ".part";
const STATE_SUFFIX: &str = ".state";
const STATE_HEADER: &str = "armasync-partial 1";
/// A state file is a few hundred bytes; anything larger is not ours and is ignored.
const MAX_STATE_BYTES: u64 = 2048;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// The digest that identifies a source inside the state file (the address itself is not stored).
fn source_digest(source: &str) -> String {
    hex(&Sha256::digest(source.as_bytes()))
}

/// The data and state file names for `key`. Identical keys share them, so a second transfer finds the first
/// transfer's bytes.
fn names(key: &PartialKey) -> (String, String) {
    let digest = Sha256::digest(
        format!("{}\n{}\n{}", source_digest(&key.source), key.path, key.size).as_bytes(),
    );
    let id = hex(&digest[..16]);
    (
        format!("{PARTIAL_PREFIX}{id}{DATA_SUFFIX}"),
        format!("{PARTIAL_PREFIX}{id}{STATE_SUFFIX}"),
    )
}

/// The fields of a state file: source digest, path, size, validator.
fn parse_state(text: &str) -> Option<(String, String, u64, String)> {
    let mut lines = text.lines();
    if lines.next()? != STATE_HEADER {
        return None;
    }
    let source = lines.next()?.strip_prefix("source ")?.to_owned();
    let path = lines.next()?.strip_prefix("path ")?.to_owned();
    let size = lines.next()?.strip_prefix("size ")?.parse().ok()?;
    let validator = lines.next()?.strip_prefix("validator ")?.to_owned();
    if lines.next().is_some() {
        return None;
    }
    Some((source, path, size, validator))
}

/// Whether `metadata` is a regular file that is neither a link nor a reparse point.
fn is_plain_file(metadata: &fs::Metadata) -> bool {
    metadata.is_file() && !is_link(metadata)
}

/// Whether an open handle refers to a regular file that is not a link (checked on the handle, not on the name).
fn is_plain_handle(file: &File) -> bool {
    file.metadata()
        .is_ok_and(|metadata| is_plain_file(&metadata))
}

/// Options that make opening an existing name refuse links: on Linux and macOS `O_NOFOLLOW`; on Windows the reparse
/// point itself is opened (and then refused by [`is_plain_handle`]). The share mode of Windows forbids renaming or
/// deleting the file while the handle is open.
fn guard(options: &mut OpenOptions) -> &mut OpenOptions {
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        const FILE_SHARE_READ: u32 = 0x1;
        const FILE_SHARE_WRITE: u32 = 0x2;
        const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
        options
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    }
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        use std::os::unix::fs::OpenOptionsExt;
        const O_NOFOLLOW: i32 = 0o400_000;
        options.custom_flags(O_NOFOLLOW);
    }
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    {
        use std::os::unix::fs::OpenOptionsExt;
        const O_NOFOLLOW: i32 = 0x0100;
        options.custom_flags(O_NOFOLLOW);
    }
    options
}

/// Opens an existing name for reading, never following a link.
fn open_existing_read(path: &Path) -> io::Result<File> {
    guard(OpenOptions::new().read(true)).open(path)
}

/// Opens an existing name for reading and writing, never following a link.
fn open_existing_write(path: &Path) -> io::Result<File> {
    guard(OpenOptions::new().read(true).write(true)).open(path)
}

/// Creates a new file. Fails if the name exists in any form, including a link.
fn create_new(path: &Path) -> io::Result<File> {
    guard(OpenOptions::new().read(true).write(true).create_new(true)).open(path)
}

/// Removes the directory entry at `path` (never the target of a link). Missing names are fine.
fn remove_entry(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

/// Whatever an earlier run left for `key`, if it can be continued: the state must match the key exactly, carry a
/// strong validator, and the data must be non-empty and shorter than the announced size. Both files are read through
/// handles that refuse links; the data handle is returned for the copy.
fn read_stored(
    key: &PartialKey,
    data_path: &Path,
    state_path: &Path,
) -> Option<(StoredPartial, File)> {
    if key.source.is_empty() {
        return None;
    }
    let mut state = open_existing_read(state_path).ok()?;
    if !is_plain_handle(&state) || state.metadata().ok()?.len() > MAX_STATE_BYTES {
        return None;
    }
    let mut text = String::new();
    (&mut state)
        .take(MAX_STATE_BYTES + 1)
        .read_to_string(&mut text)
        .ok()?;
    let (source, path, size, validator) = parse_state(&text)?;
    if source != source_digest(&key.source)
        || path != key.path.as_str()
        || size != key.size
        || !is_strong_validator(&validator)
    {
        return None;
    }
    let data = open_existing_read(data_path).ok()?;
    if !is_plain_handle(&data) {
        return None;
    }
    let offset = data.metadata().ok()?.len();
    (offset > 0 && offset < key.size).then_some((StoredPartial { offset, validator }, data))
}

/// Copies exactly `length` bytes from `source` to `target`. Returns `false` if `source` ends earlier.
fn copy_prefix(source: &mut File, target: &mut File, length: u64) -> io::Result<bool> {
    let mut buffer = vec![0u8; TRANSFER_CHUNK];
    let mut left = length;
    while left > 0 {
        let want = usize::try_from(left).map_or(buffer.len(), |left| left.min(buffer.len()));
        let read = source.read(&mut buffer[..want])?;
        if read == 0 {
            return Ok(false);
        }
        target.write_all(&buffer[..read])?;
        left -= read as u64;
    }
    Ok(true)
}

/// Continues from a prefix without writing to the existing name: the valid bytes are copied into a new file the
/// adapter created (a temporary name with the staging suffix, so a crash leaves a leftover the scan removes), and that
/// file is renamed over the data name. Returns the handle of the new data file, positioned at its end.
fn continue_in_place(
    directory: &Path,
    data_name: &str,
    data_path: &Path,
    mut source: File,
    offset: u64,
) -> io::Result<File> {
    let temporary = directory.join(format!("{data_name}.tmp"));
    remove_entry(&temporary)?;
    let mut copy = create_new(&temporary)?;
    let complete = copy_prefix(&mut source, &mut copy, offset)?;
    // The old name must be closed before it is replaced (Windows refuses to replace an open file).
    drop(source);
    if !complete {
        drop(copy);
        let _ = fs::remove_file(&temporary);
        return Err(io::Error::other(
            "the stored bytes are shorter than their state",
        ));
    }
    copy.sync_all()?;
    drop(copy);
    fs::rename(&temporary, data_path)?;
    let mut file = open_existing_write(data_path)?;
    if !is_plain_handle(&file) {
        return Err(io::Error::other("the data file was replaced"));
    }
    file.seek(SeekFrom::End(0))?;
    Ok(file)
}

/// What happens to the bytes when the handle ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fate {
    /// Kept if a validator is stored (also the result of dropping the handle), otherwise removed.
    Keep,
    /// Removed.
    Discard,
    /// Moved to the target; only the state is left to remove.
    Commit,
}

/// A partial file of the local destination (see the module notes).
#[derive(Debug)]
pub(super) struct LocalPartial<'a> {
    store: &'a LocalDirectory,
    key: PartialKey,
    data_path: PathBuf,
    state_path: PathBuf,
    file: Option<File>,
    validator: Option<String>,
    stored: Option<StoredPartial>,
    fate: Fate,
    /// Keeps the verified folders in place while the partial file exists.
    _pins: Pins,
}

/// Opens (or creates) the partial file for `key` inside the destination (see [`LocalDirectory::partial`]).
pub(super) fn open<'a>(
    store: &'a LocalDirectory,
    key: &PartialKey,
) -> Result<LocalPartial<'a>, Error> {
    store.require_lock()?;
    let relative = key.path.as_str();
    if relative
        .split('/')
        .any(|part| part.starts_with(STAGING_PREFIX))
    {
        return Err(refused(relative, "the name is reserved for the adapter"));
    }
    let (target, pins) = store.secure_target(&key.path, true)?;
    let directory = target
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| refused(relative, "no parent folder"))?;
    store.ensure_inside(&directory, relative)?;
    let (data_name, state_name) = names(key);
    let data_path = directory.join(&data_name);
    let state_path = directory.join(state_name);
    for path in [&data_path, &state_path] {
        if fs::symlink_metadata(path).is_ok_and(|metadata| metadata.is_dir()) {
            return Err(refused(
                relative,
                "a leftover of an earlier transfer is a folder",
            ));
        }
    }
    let fresh = |data_path: &Path| -> Result<File, Error> {
        remove_entry(data_path)
            .map_err(|e| fs_error("clear the partial file for", relative, &e))?;
        create_new(data_path).map_err(|e| fs_error("create the partial file for", relative, &e))
    };
    let (stored, file) = match read_stored(key, &data_path, &state_path) {
        Some((stored, source)) => {
            match continue_in_place(&directory, &data_name, &data_path, source, stored.offset) {
                Ok(file) => (Some(stored), file),
                Err(_) => (None, fresh(&data_path)?),
            }
        }
        None => (None, fresh(&data_path)?),
    };
    Ok(LocalPartial {
        store,
        key: key.clone(),
        data_path,
        state_path,
        file: Some(file),
        validator: stored.as_ref().map(|s| s.validator.clone()),
        stored,
        fate: Fate::Keep,
        _pins: pins,
    })
}

impl LocalPartial<'_> {
    fn file_mut(&mut self) -> io::Result<&mut File> {
        self.file
            .as_mut()
            .ok_or_else(|| io::Error::other("the partial file is closed"))
    }

    fn reset_data(&mut self) -> Result<(), Error> {
        let relative = self.key.path.as_str().to_owned();
        let file = self
            .file
            .as_mut()
            .ok_or_else(|| refused(&relative, "the partial file is closed"))?;
        file.set_len(0)
            .and_then(|()| file.seek(SeekFrom::Start(0)).map(|_| ()))
            .map_err(|e| fs_error("empty the partial file for", &relative, &e))
    }

    /// Writes the state through a new temporary file that is renamed over the state name: a link at the state name
    /// is replaced, its target is never written.
    fn write_state(&self, validator: &str) -> Result<(), Error> {
        let relative = self.key.path.as_str();
        let text = format!(
            "{STATE_HEADER}\nsource {}\npath {}\nsize {}\nvalidator {validator}\n",
            source_digest(&self.key.source),
            self.key.path,
            self.key.size,
        );
        let mut temporary = self.state_path.as_os_str().to_owned();
        temporary.push(".tmp");
        let temporary = PathBuf::from(temporary);
        let result = (|| -> io::Result<()> {
            remove_entry(&temporary)?;
            let mut file = create_new(&temporary)?;
            file.write_all(text.as_bytes())?;
            file.sync_all()?;
            drop(file);
            fs::rename(&temporary, &self.state_path)
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result.map_err(|e| fs_error("write the state of the partial file for", relative, &e))
    }

    fn remove_state(&self) {
        let _ = remove_entry(&self.state_path);
    }
}

impl Read for LocalPartial<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.file_mut()?.read(buffer)
    }
}

impl Write for LocalPartial<'_> {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        self.file_mut()?.write(data)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file_mut()?.flush()
    }
}

impl Seek for LocalPartial<'_> {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        self.file_mut()?.seek(position)
    }
}

impl PartialFile for LocalPartial<'_> {
    fn stored(&self) -> Option<StoredPartial> {
        self.stored.clone()
    }

    fn restart(&mut self, validator: Option<&str>) -> Result<(), Error> {
        self.reset_data()?;
        self.stored = None;
        match validator.filter(|v| is_strong_validator(v)) {
            Some(validator) => {
                self.write_state(validator)?;
                self.validator = Some(validator.to_owned());
            }
            None => {
                self.remove_state();
                self.validator = None;
            }
        }
        Ok(())
    }

    fn keep(self: Box<Self>) {
        drop(self);
    }

    fn discard(mut self: Box<Self>) {
        self.fate = Fate::Discard;
    }

    fn commit_to(mut self: Box<Self>, target: &RelativePath) -> Result<(), Error> {
        // Every failure before the swap removes the bytes (the target keeps its content); only a successful swap
        // leaves nothing but the state to remove.
        self.fate = Fate::Discard;
        let relative = target.as_str().to_owned();
        let file = self
            .file
            .take()
            .ok_or_else(|| refused(&relative, "the partial file is already closed"))?;
        file.sync_all()
            .map_err(|e| fs_error("flush to disk", &relative, &e))?;
        drop(file);
        // Re-check right before the swap, like a staged commit.
        let (target_path, _recheck) = self.store.secure_target(target, false)?;
        fs::rename(&self.data_path, &target_path)
            .map_err(|e| fs_error("replace", &relative, &e))?;
        self.fate = Fate::Commit;
        self.remove_state();
        #[cfg(unix)]
        if let Some(parent) = target_path.parent() {
            if let Ok(directory) = File::open(parent) {
                let _ = directory.sync_all();
            }
        }
        Ok(())
    }
}

impl Drop for LocalPartial<'_> {
    fn drop(&mut self) {
        let len = self
            .file
            .as_ref()
            .and_then(|file| file.metadata().ok())
            .map_or(0, |metadata| metadata.len());
        self.file = None;
        match self.fate {
            Fate::Keep if self.validator.is_some() && len > 0 => {}
            Fate::Keep | Fate::Discard => {
                let _ = remove_entry(&self.data_path);
                self.remove_state();
            }
            Fate::Commit => self.remove_state(),
        }
    }
}

/// Whether a leftover of an earlier transfer is old enough to be removed.
pub(super) fn is_stale(metadata: &fs::Metadata) -> bool {
    metadata
        .modified()
        .ok()
        .and_then(|modified| SystemTime::now().duration_since(modified).ok())
        .is_some_and(|age| age >= PARTIAL_MAX_AGE)
}

/// Whether a partial file (data or state) is removable under the exclusive lock: it is old, or its partner file is
/// missing. Without the partner a data file can never be continued, and a state without data has nothing to continue.
pub(super) fn is_removable(path: &Path, metadata: &fs::Metadata) -> bool {
    if is_stale(metadata) {
        return true;
    }
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    let partner = if let Some(stem) = name.strip_suffix(DATA_SUFFIX) {
        format!("{stem}{STATE_SUFFIX}")
    } else if let Some(stem) = name.strip_suffix(STATE_SUFFIX) {
        format!("{stem}{DATA_SUFFIX}")
    } else {
        return false;
    };
    let partner_path = path.with_file_name(partner);
    fs::symlink_metadata(partner_path).is_err()
}
