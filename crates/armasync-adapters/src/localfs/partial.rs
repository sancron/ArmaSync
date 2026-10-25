// SPDX-License-Identifier: AGPL-3.0-only
//! Resumable partial files of the destination folder (MIG-032R, ADR-0009).
//!
//! A partial file holds the bytes of one transfer while it arrives, possibly over several runs:
//! * data in `.armasync-partial-<id>.part` and its state in `.armasync-partial-<id>.state`, both in the folder of the
//!   target, both with the reserved prefix (never listed, never a repository file);
//! * the state records the source (as a digest), the path, the announced size and the **strong** validator of the
//!   answer the bytes came from; a state that does not match the requested key is never used;
//! * [`LocalPartial::commit_to`] is the only way the bytes reach the target, and only after the caller verified the
//!   complete content;
//! * the handle decides the fate of the bytes: kept (with a validator) on drop without a decision, removed by
//!   `discard`, `commit` and every state without a validator.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use armasync_core::{
    Error, PartialFile, PartialKey, RelativePath, StoredPartial, is_strong_validator,
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

/// The data and state file names for `key`. Identical keys share them, so a second transfer finds the first one's
/// bytes.
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

/// Whatever an earlier run left for `key`, if it can be continued: the state must match the key exactly, carry a
/// strong validator, and the data must be non-empty and shorter than the announced size.
fn read_stored(key: &PartialKey, data_path: &Path, state_path: &Path) -> Option<StoredPartial> {
    if key.source.is_empty() {
        return None;
    }
    let state_meta = fs::symlink_metadata(state_path).ok()?;
    if !is_plain_file(&state_meta) || state_meta.len() > MAX_STATE_BYTES {
        return None;
    }
    let text = fs::read_to_string(state_path).ok()?;
    let (source, path, size, validator) = parse_state(&text)?;
    if source != source_digest(&key.source)
        || path != key.path.as_str()
        || size != key.size
        || !is_strong_validator(&validator)
    {
        return None;
    }
    let data_meta = fs::symlink_metadata(data_path).ok()?;
    if !is_plain_file(&data_meta) {
        return None;
    }
    let offset = data_meta.len();
    (offset > 0 && offset < key.size).then_some(StoredPartial { offset, validator })
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
    let data_path = directory.join(data_name);
    let state_path = directory.join(state_name);
    for path in [&data_path, &state_path] {
        if let Ok(metadata) = fs::symlink_metadata(path) {
            if !is_plain_file(&metadata) {
                return Err(refused(
                    relative,
                    "a leftover of an earlier transfer is not a file",
                ));
            }
        }
    }
    let stored = read_stored(key, &data_path, &state_path);
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&data_path)
        .map_err(|e| fs_error("open the partial file for", relative, &e))?;
    let mut partial = LocalPartial {
        store,
        key: key.clone(),
        data_path,
        state_path,
        file: Some(file),
        validator: stored.as_ref().map(|s| s.validator.clone()),
        stored,
        fate: Fate::Keep,
        _pins: pins,
    };
    if partial.stored.is_some() {
        partial
            .file_mut()
            .map_err(|e| fs_error("position the partial file for", relative, &e))?
            .seek(SeekFrom::End(0))
            .map_err(|e| fs_error("position the partial file for", relative, &e))?;
    } else {
        // Nothing reusable: whatever is in the data file belongs to no valid state and is dropped.
        partial.reset_data()?;
        partial.validator = None;
    }
    Ok(partial)
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

    fn write_state(&self, validator: &str) -> Result<(), Error> {
        let relative = self.key.path.as_str();
        if let Ok(metadata) = fs::symlink_metadata(&self.state_path) {
            if !is_plain_file(&metadata) {
                return Err(refused(
                    relative,
                    "a leftover of an earlier transfer is not a file",
                ));
            }
        }
        let text = format!(
            "{STATE_HEADER}\nsource {}\npath {}\nsize {}\nvalidator {validator}\n",
            source_digest(&self.key.source),
            self.key.path,
            self.key.size,
        );
        fs::write(&self.state_path, text)
            .map_err(|e| fs_error("write the state of the partial file for", relative, &e))
    }

    fn remove_state(&self) {
        let _ = fs::remove_file(&self.state_path);
    }

    fn data_len(&self) -> u64 {
        fs::symlink_metadata(&self.data_path)
            .map(|metadata| metadata.len())
            .unwrap_or(0)
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
        self.file = None;
        match self.fate {
            Fate::Keep if self.validator.is_some() && self.data_len() > 0 => {}
            Fate::Keep | Fate::Discard => {
                let _ = fs::remove_file(&self.data_path);
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
