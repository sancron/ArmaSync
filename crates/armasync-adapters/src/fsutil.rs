// SPDX-License-Identifier: AGPL-3.0-only
//! File helpers shared by the stores: bounded reads and atomic replacement with a backup of the previous version.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use armasync_core::{Error, ErrorKind};

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// Maps an I/O failure to the shared error type (file-system category, no file content in the message).
pub(crate) fn io_error(action: &str, path: &Path, error: &io::Error) -> Error {
    Error::new(
        ErrorKind::FileSystem,
        format!("cannot {action} '{}': {error}", path.display()),
    )
}

/// `<path>.bak`, the previous good version of a document.
pub(crate) fn backup_path(path: &Path) -> PathBuf {
    sibling(path, "", ".bak")
}

fn sibling(path: &Path, prefix: &str, suffix: &str) -> PathBuf {
    let name = path
        .file_name()
        .map_or_else(|| "data".into(), |name| name.to_string_lossy().into_owned());
    path.with_file_name(format!("{prefix}{name}{suffix}"))
}

/// Reads at most `max` bytes of a regular file. `Ok(None)` if the file does not exist.
///
/// The size is checked before reading and the read itself is capped, so a file that grows while it is read cannot
/// exhaust memory. Directories, devices and pipes are refused (a pipe would block).
///
/// # Errors
/// [`ErrorKind::Format`] if the file is larger than `max`; [`ErrorKind::FileSystem`] for I/O failures or a file
/// that is not a regular file.
pub(crate) fn read_limited(path: &Path, max: u64) -> Result<Option<Vec<u8>>, Error> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(io_error("open", path, &error)),
    };
    let metadata = file
        .metadata()
        .map_err(|error| io_error("inspect", path, &error))?;
    if !metadata.is_file() {
        return Err(Error::new(
            ErrorKind::FileSystem,
            format!("'{}' is not a regular file", path.display()),
        ));
    }
    let too_large = || {
        Error::new(
            ErrorKind::Format,
            format!(
                "'{}' is larger than the allowed {max} bytes",
                path.display()
            ),
        )
    };
    if metadata.len() > max {
        return Err(too_large());
    }
    let mut buffer = Vec::new();
    file.take(max + 1)
        .read_to_end(&mut buffer)
        .map_err(|error| io_error("read", path, &error))?;
    if buffer.len() as u64 > max {
        return Err(too_large());
    }
    Ok(Some(buffer))
}

/// Replaces `path` atomically with `bytes`.
///
/// The content goes to a temporary file in the same directory, is flushed to disk and then renamed over the target:
/// afterwards the file has either the old or the complete new content, also after a crash. With `keep_backup` the
/// previous version is first copied to `<path>.bak` (itself via temporary file and rename). Missing directories are
/// created. The temporary file is removed on failure.
///
/// # Errors
/// [`ErrorKind::FileSystem`]; the previous content stays intact.
pub(crate) fn write_atomic(path: &Path, bytes: &[u8], keep_backup: bool) -> Result<(), Error> {
    write_atomic_with(path, bytes, keep_backup, |_| Ok(()))
}

/// Like [`write_atomic`], with a hook that runs after the new content is fully written and before it replaces the
/// target (test seam for crash and failure behaviour).
pub(crate) fn write_atomic_with(
    path: &Path,
    bytes: &[u8],
    keep_backup: bool,
    before_commit: impl FnOnce(&Path) -> io::Result<()>,
) -> Result<(), Error> {
    let directory = path.parent().filter(|p| !p.as_os_str().is_empty());
    if let Some(directory) = directory {
        fs::create_dir_all(directory).map_err(|error| io_error("create", directory, &error))?;
    }
    let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
    let suffix = format!(".tmp-{}-{unique}", std::process::id());
    let temporary = sibling(path, ".", &suffix);
    let mut created = vec![temporary.clone()];
    let result = (|| -> io::Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        before_commit(&temporary)?;
        if keep_backup && path.is_file() {
            let backup_temporary =
                sibling(path, ".", &format!(".bak-{}-{unique}", std::process::id()));
            created.push(backup_temporary.clone());
            fs::copy(path, &backup_temporary)?;
            fs::rename(&backup_temporary, backup_path(path))?;
        }
        fs::rename(&temporary, path)?;
        if let Some(directory) = directory {
            sync_directory(directory);
        }
        Ok(())
    })();
    if let Err(error) = result {
        for leftover in created {
            let _ = fs::remove_file(leftover);
        }
        return Err(io_error("write", path, &error));
    }
    Ok(())
}

/// Flushes the directory entry after a rename where the platform supports it (best effort).
#[cfg(unix)]
fn sync_directory(directory: &Path) {
    if let Ok(handle) = File::open(directory) {
        let _ = handle.sync_all();
    }
}

#[cfg(not(unix))]
fn sync_directory(_directory: &Path) {}

#[cfg(test)]
mod tests {
    use super::{backup_path, read_limited, write_atomic, write_atomic_with};
    use armasync_core::ErrorKind;
    use std::fs;
    use std::io;
    use std::path::PathBuf;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "armasync-fsutil-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos())
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn entries(dir: &std::path::Path) -> Vec<String> {
        let mut names: Vec<_> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn write_creates_directories_and_replaces_content() {
        let dir = scratch("create");
        let file = dir.join("a").join("b").join("doc.json");
        write_atomic(&file, b"one", false).unwrap();
        assert_eq!(fs::read(&file).unwrap(), b"one");
        write_atomic(&file, b"two", false).unwrap();
        assert_eq!(fs::read(&file).unwrap(), b"two");
        assert_eq!(entries(file.parent().unwrap()), ["doc.json"]);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn backup_keeps_the_previous_version() {
        let dir = scratch("backup");
        let file = dir.join("doc.json");
        write_atomic(&file, b"v1", true).unwrap();
        assert!(!backup_path(&file).exists(), "nothing to back up yet");
        write_atomic(&file, b"v2", true).unwrap();
        write_atomic(&file, b"v3", true).unwrap();
        assert_eq!(fs::read(&file).unwrap(), b"v3");
        assert_eq!(fs::read(backup_path(&file)).unwrap(), b"v2");
        assert_eq!(entries(&dir), ["doc.json", "doc.json.bak"]);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn failure_before_commit_keeps_old_content_and_cleans_up() {
        let dir = scratch("fail");
        let file = dir.join("doc.json");
        write_atomic(&file, b"old", true).unwrap();
        let error = write_atomic_with(&file, b"new", true, |_| {
            Err(io::Error::other("simulated crash before rename"))
        })
        .unwrap_err();
        assert_eq!(error.kind(), ErrorKind::FileSystem);
        assert_eq!(fs::read(&file).unwrap(), b"old");
        assert_eq!(entries(&dir), ["doc.json"], "temporary files are removed");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn read_is_bounded_and_refuses_non_files() {
        let dir = scratch("read");
        let file = dir.join("doc.json");
        assert_eq!(read_limited(&file, 10).unwrap(), None);
        fs::write(&file, b"0123456789").unwrap();
        assert_eq!(read_limited(&file, 10).unwrap().unwrap(), b"0123456789");
        fs::write(&file, b"01234567890").unwrap();
        let error = read_limited(&file, 10).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::Format);
        let error = read_limited(&dir, 10).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::FileSystem);
        fs::remove_dir_all(dir).unwrap();
    }
}
