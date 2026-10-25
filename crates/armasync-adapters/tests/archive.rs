// SPDX-License-Identifier: AGPL-3.0-only
//! The ZIP unpacker (MIG-030E, ADR-0008): valid archives, and every refusal the contract names.
//! All archives are generated here; nothing is taken from the legacy client.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::io::{self, Cursor, Write};

use armasync_adapters::archive::{MAX_PACKED_BYTES, ZipUnpacker};
use armasync_core::{ArchiveReader, CancelToken, Error, ErrorKind};
use zip::CompressionMethod;
use zip::write::SimpleFileOptions;

fn pattern(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i % 251) as u8 ^ (i / 7) as u8).collect()
}

fn stored(name: &str, data: &[u8]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    writer
        .start_file(
            name,
            SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
        )
        .unwrap();
    writer.write_all(data).unwrap();
    writer.finish().unwrap().into_inner()
}

fn deflated(name: &str, data: &[u8]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    writer
        .start_file(
            name,
            SimpleFileOptions::default().compression_method(CompressionMethod::Deflated),
        )
        .unwrap();
    writer.write_all(data).unwrap();
    writer.finish().unwrap().into_inner()
}

fn unpack(archive: &[u8], expected: u64) -> Result<Vec<u8>, Error> {
    let mut spool = Cursor::new(archive.to_vec());
    let mut out = Vec::new();
    let written =
        ZipUnpacker.extract_single(&mut spool, expected, &mut out, &CancelToken::new())?;
    assert_eq!(
        written,
        out.len() as u64,
        "reported and written bytes agree"
    );
    Ok(out)
}

fn refused(archive: &[u8], expected: u64, kind: ErrorKind) {
    let error = unpack(archive, expected).expect_err("the archive must be refused");
    assert_eq!(error.kind(), kind, "{error}");
}

#[test]
fn stored_entry_unpacks_to_the_announced_bytes() {
    let data = b"the unpacked file".to_vec();
    assert_eq!(
        unpack(&stored("example.pbo", &data), data.len() as u64).unwrap(),
        data
    );
}

#[test]
fn deflated_entry_unpacks_across_transfer_chunks() {
    // Larger than TRANSFER_CHUNK (64 KiB) so the unpacker has to loop.
    let data = pattern(300 * 1024);
    let archive = deflated("example.pbo", &data);
    assert!(
        archive.len() < data.len(),
        "the test data must be compressible"
    );
    assert_eq!(unpack(&archive, data.len() as u64).unwrap(), data);
}

#[test]
fn the_entry_name_does_not_decide_the_destination() {
    let data = b"content".to_vec();
    // The unpacker returns the bytes only; the caller places them at the manifest path.
    assert_eq!(
        unpack(&stored("some_other_name.bin", &data), 7).unwrap(),
        data
    );
}

#[test]
fn two_entries_are_refused() {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default();
    writer.start_file("a.pbo", options).unwrap();
    writer.write_all(b"aa").unwrap();
    writer.start_file("b.pbo", options).unwrap();
    writer.write_all(b"bb").unwrap();
    let archive = writer.finish().unwrap().into_inner();
    refused(&archive, 2, ErrorKind::Format);
}

#[test]
fn an_empty_archive_is_refused() {
    let archive = zip::ZipWriter::new(Cursor::new(Vec::new()))
        .finish()
        .unwrap()
        .into_inner();
    refused(&archive, 0, ErrorKind::Format);
}

#[test]
fn a_folder_entry_is_refused() {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    writer
        .add_directory("folder/", SimpleFileOptions::default())
        .unwrap();
    let archive = writer.finish().unwrap().into_inner();
    refused(&archive, 0, ErrorKind::Format);
}

#[test]
fn a_link_entry_is_refused() {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    writer
        .add_symlink("link.pbo", "target.pbo", SimpleFileOptions::default())
        .unwrap();
    let archive = writer.finish().unwrap().into_inner();
    refused(&archive, "target.pbo".len() as u64, ErrorKind::Format);
}

#[test]
fn unsafe_entry_names_are_refused() {
    for name in [
        "../escape.pbo",
        "/absolute.pbo",
        "dir/../../x.pbo",
        "back\\slash.pbo",
        "",
    ] {
        let archive = stored(name, b"x");
        let error = unpack(&archive, 1).expect_err(name);
        assert_eq!(error.kind(), ErrorKind::Format, "{name}: {error}");
    }
}

#[test]
fn a_size_that_differs_from_the_manifest_is_refused() {
    let data = b"12345".to_vec();
    let archive = stored("x.pbo", &data);
    refused(&archive, 4, ErrorKind::Integrity);
    refused(&archive, 6, ErrorKind::Integrity);
}

#[test]
fn a_corrupted_stored_entry_fails_its_checksum() {
    let data = pattern(4096);
    let mut archive = stored("x.pbo", &data);
    // Stored data appears verbatim in the archive: flip one byte in the middle of it.
    let offset = archive
        .windows(data.len())
        .position(|window| window == data.as_slice())
        .expect("stored bytes are verbatim")
        + 2000;
    archive[offset] ^= 0x5a;
    let error = unpack(&archive, data.len() as u64).expect_err("the checksum must fail");
    assert!(
        matches!(error.kind(), ErrorKind::Integrity | ErrorKind::Format),
        "{error}"
    );
}

#[test]
fn a_damaged_deflated_entry_is_refused() {
    let data = pattern(100 * 1024);
    let mut archive = deflated("x.pbo", &data);
    let middle = archive.len() / 2;
    archive[middle] ^= 0xff;
    let error = unpack(&archive, data.len() as u64).expect_err("damage must be detected");
    assert!(
        matches!(error.kind(), ErrorKind::Integrity | ErrorKind::Format),
        "{error}"
    );
}

#[test]
fn a_truncated_archive_is_refused() {
    let data = pattern(8192);
    let archive = stored("x.pbo", &data);
    refused(
        &archive[..archive.len() - 30],
        data.len() as u64,
        ErrorKind::Format,
    );
}

#[test]
fn bytes_that_are_not_a_zip_archive_are_refused() {
    refused(b"not an archive at all, just text", 4, ErrorKind::Format);
}

#[test]
fn an_announced_size_above_the_maximum_is_refused_before_reading() {
    // The spool is never read: the size is refused first.
    refused(b"", MAX_PACKED_BYTES + 1, ErrorKind::Format);
}

#[test]
fn a_cancelled_unpack_writes_nothing() {
    let cancel = CancelToken::new();
    cancel.cancel();
    let mut out = Vec::new();
    let mut spool = Cursor::new(stored("x.pbo", b"data"));
    let error = ZipUnpacker
        .extract_single(&mut spool, 4, &mut out, &cancel)
        .expect_err("cancelled");
    assert_eq!(error.kind(), ErrorKind::Cancelled);
    assert!(out.is_empty());
}

/// Cancels the token after the first write, as a user pressing Ctrl+C during a transfer would.
struct CancelAfterFirstWrite {
    cancel: CancelToken,
    written: usize,
}

impl Write for CancelAfterFirstWrite {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        self.written += data.len();
        self.cancel.cancel();
        Ok(data.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn cancellation_during_unpacking_stops_at_the_next_chunk() {
    let data = pattern(300 * 1024);
    let archive = deflated("x.pbo", &data);
    let cancel = CancelToken::new();
    let mut out = CancelAfterFirstWrite {
        cancel: cancel.clone(),
        written: 0,
    };
    let mut spool = Cursor::new(archive);
    let error = ZipUnpacker
        .extract_single(&mut spool, data.len() as u64, &mut out, &cancel)
        .expect_err("cancelled after the first chunk");
    assert_eq!(error.kind(), ErrorKind::Cancelled);
    assert!(
        out.written < data.len(),
        "unpacking stopped early ({} of {} bytes)",
        out.written,
        data.len()
    );
}

/// A destination that fails on the first write (for example a full disk).
struct FullDisk;

impl Write for FullDisk {
    fn write(&mut self, _data: &[u8]) -> io::Result<usize> {
        Err(io::Error::other("no space left on device"))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn a_failing_destination_is_a_file_system_error() {
    let data = b"data".to_vec();
    let mut spool = Cursor::new(stored("x.pbo", &data));
    let error = ZipUnpacker
        .extract_single(&mut spool, 4, &mut FullDisk, &CancelToken::new())
        .expect_err("the write fails");
    assert_eq!(error.kind(), ErrorKind::FileSystem);
}
