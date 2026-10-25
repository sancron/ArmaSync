// SPDX-License-Identifier: AGPL-3.0-only
//! Unpacking of the packed copies a repository publishes as `<file>.zip` (MIG-030E, ADR-0008).
//!
//! [`ZipUnpacker`] implements the core's [`ArchiveReader`] with the `zip` crate. It accepts only what the tests cover:
//! an archive with **exactly one** regular file, not encrypted, stored or deflated. Everything else is refused before
//! a single byte of file data is written:
//! * folders, links, more than one entry, duplicate names and names that are not safe relative paths (the name is
//!   never used as the destination; the manifest decides where the file goes),
//! * encryption and any other compression method,
//! * a declared size that differs from the size announced by the manifest,
//! * a declared expansion that a DEFLATE stream cannot reach (see [`MAX_DEFLATE_RATIO`]).
//!
//! While the data is written, at most the announced number of bytes leave the unpacker, and the checksum of the entry
//! is verified at its end (`zip` reports a mismatch as an I/O error). Damage shows up as [`ErrorKind::Integrity`] or
//! [`ErrorKind::Format`]; error messages never contain names or checksums from the archive.

use std::io::{self, Read, Write};

use armasync_core::{
    ArchiveReader, CancelToken, Error, ErrorKind, RelativePath, Spool, TRANSFER_CHUNK,
};
use zip::result::ZipError;
use zip::{CompressionMethod, ZipArchive};

/// Largest file (and archive) the unpacker accepts, in bytes. Checked against the size the manifest announces before
/// anything is downloaded and again before unpacking.
pub const MAX_PACKED_BYTES: u64 = 2 * 1024 * 1024 * 1024;

/// Largest possible expansion of a DEFLATE stream (RFC 1951 codes: a 258-byte match takes at least one bit). A header
/// that claims more cannot come from a valid encoder and is refused before decoding.
pub const MAX_DEFLATE_RATIO: u64 = 1032;

/// Slack for the header arithmetic of very small entries.
const RATIO_SLACK: u64 = 1024;

/// The ZIP unpacker of the adapters. It has no state; the archive is read from the spool handed in.
#[derive(Debug, Clone, Copy, Default)]
pub struct ZipUnpacker;

impl ArchiveReader for ZipUnpacker {
    fn extract_single(
        &self,
        archive: &mut dyn Spool,
        expected_size: u64,
        out: &mut dyn Write,
        cancel: &CancelToken,
    ) -> Result<u64, Error> {
        extract_single_limited(archive, expected_size, MAX_PACKED_BYTES, out, cancel)
    }
}

/// [`ZipUnpacker::extract_single`] with an explicit size limit (tests use a small one).
pub(crate) fn extract_single_limited(
    archive: &mut dyn Spool,
    expected_size: u64,
    max_size: u64,
    out: &mut dyn Write,
    cancel: &CancelToken,
) -> Result<u64, Error> {
    if cancel.is_cancelled() {
        return Err(cancelled());
    }
    if expected_size > max_size {
        return Err(Error::new(
            ErrorKind::Format,
            "the announced file is larger than the accepted maximum",
        ));
    }
    let mut zip = ZipArchive::new(archive).map_err(zip_error)?;
    match zip.len() {
        1 => {}
        0 => return Err(Error::new(ErrorKind::Format, "the archive is empty")),
        _ => {
            return Err(Error::new(
                ErrorKind::Format,
                "the archive contains more than one entry; exactly one is accepted",
            ));
        }
    }
    let (name_ok, is_dir, is_link, encrypted, method, compressed, declared) = {
        let meta = zip.by_index_raw(0).map_err(zip_error)?;
        (
            RelativePath::parse(meta.name()).is_ok(),
            meta.is_dir(),
            meta.is_symlink(),
            meta.encrypted(),
            meta.compression(),
            meta.compressed_size(),
            meta.size(),
        )
    };
    if is_dir {
        return Err(Error::new(
            ErrorKind::Format,
            "the archive contains a folder",
        ));
    }
    if is_link {
        return Err(Error::new(ErrorKind::Format, "the archive contains a link"));
    }
    if !name_ok {
        return Err(Error::new(
            ErrorKind::Format,
            "the entry name is not a safe relative path",
        ));
    }
    if encrypted {
        return Err(Error::new(
            ErrorKind::Unsupported,
            "encrypted archives are not supported",
        ));
    }
    if declared != expected_size {
        return Err(Error::new(
            ErrorKind::Integrity,
            "the archive announces a size that differs from the manifest",
        ));
    }
    match method {
        CompressionMethod::Stored => {
            if compressed != declared {
                return Err(Error::new(
                    ErrorKind::Format,
                    "a stored entry must have equal packed and unpacked sizes",
                ));
            }
        }
        CompressionMethod::Deflated => {
            let limit = compressed
                .saturating_mul(MAX_DEFLATE_RATIO)
                .saturating_add(RATIO_SLACK);
            if declared > limit {
                return Err(Error::new(
                    ErrorKind::Format,
                    "the announced expansion is impossible for DEFLATE data",
                ));
            }
        }
        _ => {
            return Err(Error::new(
                ErrorKind::Unsupported,
                "this compression method is not supported",
            ));
        }
    }

    let mut entry = zip.by_index(0).map_err(zip_error)?;
    let mut buffer = vec![0u8; TRANSFER_CHUNK];
    let mut written: u64 = 0;
    loop {
        if cancel.is_cancelled() {
            return Err(cancelled());
        }
        let remaining = expected_size - written;
        if remaining == 0 {
            // Reading past the announced size must yield the end of the entry; this read also verifies its checksum.
            let mut probe = [0u8; 1];
            let extra = read_entry(&mut entry, &mut probe)?;
            if extra != 0 {
                return Err(Error::new(
                    ErrorKind::Integrity,
                    "the archive holds more data than announced",
                ));
            }
            return Ok(written);
        }
        let want = usize::try_from(remaining).map_or(buffer.len(), |r| r.min(buffer.len()));
        let read = read_entry(&mut entry, &mut buffer[..want])?;
        if read == 0 {
            return Err(Error::new(
                ErrorKind::Integrity,
                "the archive holds less data than announced",
            ));
        }
        out.write_all(&buffer[..read])
            .map_err(|_| Error::new(ErrorKind::FileSystem, "cannot write the unpacked data"))?;
        written += read as u64;
    }
}

fn read_entry(entry: &mut impl Read, buffer: &mut [u8]) -> Result<usize, Error> {
    entry.read(buffer).map_err(|error| match error.kind() {
        io::ErrorKind::InvalidData | io::ErrorKind::UnexpectedEof => Error::new(
            ErrorKind::Integrity,
            "the archive is damaged (checksum or data error)",
        ),
        _ => Error::new(ErrorKind::FileSystem, "cannot read the archive"),
    })
}

fn zip_error(error: ZipError) -> Error {
    match error {
        ZipError::Io(error) => match error.kind() {
            io::ErrorKind::InvalidData | io::ErrorKind::UnexpectedEof => Error::new(
                ErrorKind::Integrity,
                "the archive is damaged (checksum or data error)",
            ),
            _ => Error::new(ErrorKind::FileSystem, "cannot read the archive"),
        },
        ZipError::InvalidArchive(_) | ZipError::FileNotFound => Error::new(
            ErrorKind::Format,
            "the archive is damaged or not a ZIP archive",
        ),
        ZipError::UnsupportedArchive(_) | ZipError::InvalidPassword => Error::new(
            ErrorKind::Unsupported,
            "the archive uses a feature that is not supported",
        ),
        // `ZipError` is non-exhaustive: anything new is refused rather than guessed.
        _ => Error::new(
            ErrorKind::Unsupported,
            "the archive uses a feature that is not supported",
        ),
    }
}

fn cancelled() -> Error {
    Error::new(ErrorKind::Cancelled, "cancelled")
}

#[cfg(test)]
mod tests {
    use super::MAX_DEFLATE_RATIO;

    #[test]
    fn deflate_ratio_is_the_known_bound() {
        assert_eq!(MAX_DEFLATE_RATIO, 1032);
    }
}
