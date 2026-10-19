// SPDX-License-Identifier: AGPL-3.0-only
//! Tests of the transfer contract helpers (ADR-0007) and of the protected-path rule of the plan.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::io::{self, Write};

use armasync_core::{
    CancelToken, ContentHash, ErrorKind, FileEntry, HashAlgorithm, LocalFile, Manifest,
    PlanOptions, PlanWarning, RelativePath, SyncStep, TRANSFER_CHUNK, copy_in_chunks, plan_sync,
};

fn path(text: &str) -> RelativePath {
    RelativePath::parse(text).unwrap()
}

struct FailingSink {
    accept: usize,
}

impl Write for FailingSink {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.len() > self.accept {
            return Err(io::Error::other("disk full"));
        }
        self.accept -= buf.len();
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn copies_in_bounded_chunks_and_reports_progress() {
    let data = vec![7u8; TRANSFER_CHUNK * 2 + 10];
    let mut out = Vec::new();
    let mut reported = Vec::new();
    let written = copy_in_chunks(
        &data,
        data.len() as u64,
        &mut out,
        &CancelToken::new(),
        &mut |n| reported.push(n),
    )
    .unwrap();
    assert_eq!(written, data.len() as u64);
    assert_eq!(out, data);
    assert_eq!(reported, [TRANSFER_CHUNK as u64, TRANSFER_CHUNK as u64, 10]);
    assert!(reported.iter().all(|n| *n <= TRANSFER_CHUNK as u64));
}

#[test]
fn never_writes_more_than_announced() {
    let data = vec![1u8; TRANSFER_CHUNK + 5];
    let mut out = Vec::new();
    let error = copy_in_chunks(&data, 100, &mut out, &CancelToken::new(), &mut |_| {}).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Integrity);
    assert_eq!(out.len(), 100, "exactly the announced length, then stop");
}

#[test]
fn a_short_source_is_an_integrity_error() {
    let mut out = Vec::new();
    let error = copy_in_chunks(b"abc", 10, &mut out, &CancelToken::new(), &mut |_| {}).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Integrity);
    assert_eq!(out, b"abc");
}

#[test]
fn cancellation_is_checked_before_each_chunk() {
    let data = vec![0u8; TRANSFER_CHUNK * 3];
    let cancel = CancelToken::new();
    let mut out = Vec::new();
    let mut chunks = 0;
    let error = copy_in_chunks(&data, data.len() as u64, &mut out, &cancel, &mut |_| {
        chunks += 1;
        cancel.cancel();
    })
    .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Cancelled);
    assert_eq!((chunks, out.len()), (1, TRANSFER_CHUNK));
    // already cancelled: nothing is written at all
    let mut empty = Vec::new();
    let error =
        copy_in_chunks(&data, data.len() as u64, &mut empty, &cancel, &mut |_| {}).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Cancelled);
    assert!(empty.is_empty());
}

#[test]
fn a_failing_sink_is_a_file_system_error_without_leaking_detail() {
    let mut sink = FailingSink { accept: 10 };
    let error = copy_in_chunks(
        &[0u8; 100],
        100,
        &mut sink,
        &CancelToken::new(),
        &mut |_| {},
    )
    .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::FileSystem);
}

fn hash(digit: char) -> ContentHash {
    ContentHash::new(HashAlgorithm::Sha1, &digit.to_string().repeat(40)).unwrap()
}

fn manifest(files: &[(&str, u64, char)]) -> Manifest {
    Manifest::new(
        "1",
        files
            .iter()
            .map(|(p, size, d)| FileEntry {
                path: path(p),
                size: *size,
                hash: hash(*d),
            })
            .collect(),
    )
    .unwrap()
}

fn local(p: &str) -> LocalFile {
    LocalFile {
        path: path(p),
        size: 1,
        hash: None,
    }
}

#[test]
fn protected_paths_are_never_deleted_even_inside_managed_roots() {
    let options = PlanOptions {
        delete_extra: true,
        managed_roots: vec![path("mods")],
        protected_paths: vec![path("mods/@hidden"), path("mods/@x/addons/a.pbo")],
    };
    let files = [
        local("mods/@hidden/keep.pbo"),
        local("mods/@hidden/deep/er/keep2.pbo"),
        local("mods/@x/addons/a.pbo"),
        local("mods/@x/addons/b.pbo"),
        local("mods/@y/extra.pbo"),
    ];
    let plan = plan_sync(&manifest(&[("mods/@x/new.pbo", 1, 'a')]), &files, &options);
    let deleted: Vec<_> = plan
        .steps
        .iter()
        .filter_map(|s| match s {
            SyncStep::Delete { path } => Some(path.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(deleted, ["mods/@x/addons/b.pbo", "mods/@y/extra.pbo"]);
    let kept: Vec<_> = plan
        .warnings
        .iter()
        .filter_map(|w| match w {
            PlanWarning::ExtraKeptProtected(p) => Some(p.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(
        kept,
        [
            "mods/@hidden/deep/er/keep2.pbo",
            "mods/@hidden/keep.pbo",
            "mods/@x/addons/a.pbo"
        ]
    );
    assert_eq!(plan.summary.delete_files, 2);
    // a protected prefix is matched segment-wise: "@hidden2" is not below "@hidden"
    let sibling = [local("mods/@hidden2/x.pbo")];
    let plan = plan_sync(&manifest(&[]), &sibling, &options);
    assert_eq!(plan.summary.delete_files, 1);
}

#[test]
fn protection_only_matters_when_deletion_is_requested() {
    let options = PlanOptions {
        delete_extra: false,
        managed_roots: vec![path("mods")],
        protected_paths: vec![path("mods/@hidden")],
    };
    let plan = plan_sync(&manifest(&[]), &[local("mods/@hidden/a.pbo")], &options);
    assert_eq!(plan.summary.delete_files, 0);
    assert!(plan.warnings.is_empty());
}
