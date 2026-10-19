// SPDX-License-Identifier: AGPL-3.0-only
//! Tests of the metadata → manifest mapping (MIG-030A) against the MIG-001 fixtures and crafted trees, and of the
//! plan that results when local files are compared by SHA-1. All data is synthetic.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;

use armasync_adapters::Sha256Hasher;
use armasync_adapters::sync_meta::{SyncWarning, metadata_from_tree, read_sync_metadata};
use armasync_compat::{ServerInfo, SyncDirectory, SyncFile, SyncTree};
use armasync_core::{
    ErrorKind, HashAlgorithm, Hasher, LocalFile, PlanOptions, RelativePath, SyncStep, plan_sync,
};

fn fixture(scenario: &str, file: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/legacy-formats/generated/valid/server")
        .join(scenario)
        .join(".a3s")
        .join(file);
    std::fs::read(path).unwrap()
}

fn path(text: &str) -> RelativePath {
    RelativePath::parse(text).unwrap()
}

fn file(name: &str, content: &[u8]) -> SyncFile {
    SyncFile {
        name: name.to_owned(),
        sha1: Sha256Hasher
            .hash(HashAlgorithm::Sha1, content)
            .hex()
            .to_owned(),
        size: content.len() as u64,
        compressed: false,
        compressed_size: 0,
    }
}

fn dir(name: &str, directories: Vec<SyncDirectory>, files: Vec<SyncFile>) -> SyncDirectory {
    SyncDirectory {
        name: name.to_owned(),
        hidden: false,
        mark_as_addon: false,
        directories,
        files,
    }
}

fn tree(root: SyncDirectory) -> SyncTree {
    let file_count = {
        fn count(d: &SyncDirectory) -> usize {
            d.files.len() + d.directories.iter().map(count).sum::<usize>()
        }
        count(&root)
    };
    SyncTree { root, file_count }
}

fn info(hidden: &[&str]) -> ServerInfo {
    ServerInfo {
        revision: 7,
        number_of_files: 0,
        total_files_size: 0,
        number_of_connections: 4,
        hidden_folder_paths: hidden.iter().map(|s| (*s).to_owned()).collect(),
    }
}

#[test]
fn small_fixture_becomes_a_sha1_manifest() {
    let metadata =
        read_sync_metadata(&fixture("small", "sync"), &fixture("small", "serverinfo")).unwrap();
    let manifest = &metadata.manifest;
    assert_eq!(manifest.revision(), "3");
    let listed: Vec<_> = manifest.entries().iter().map(|e| e.path.as_str()).collect();
    assert_eq!(
        listed,
        [
            "@example_core/addons/example_core.pbo",
            "@example_core/keys/example.bikey"
        ]
    );
    let pbo = manifest
        .get(&path("@example_core/addons/example_core.pbo"))
        .unwrap();
    assert_eq!(pbo.size, 8193);
    assert_eq!(
        pbo.hash.to_string(),
        "sha1:7935137a8950813a6ac435aa2b4c9beea989d744"
    );
    // the compressed leaf is neither installed nor forgotten
    assert_eq!(
        metadata.unsupported,
        [path("@example_core/addons/example_core.pbo.zip")]
    );
    assert!(
        metadata
            .warnings
            .iter()
            .any(|w| matches!(w, SyncWarning::CompressedFileUnsupported(_)))
    );
    assert_eq!(metadata.protected, [path("@second")]);
    assert_eq!(metadata.server.revision, 3);
    assert_eq!(metadata.server.number_of_connections, 4);
}

#[test]
fn empty_fixture_is_an_empty_manifest() {
    let metadata =
        read_sync_metadata(&fixture("empty", "sync"), &fixture("empty", "serverinfo")).unwrap();
    assert!(metadata.manifest.entries().is_empty());
    assert!(metadata.warnings.is_empty());
}

#[test]
fn hostile_fixture_is_rejected_because_its_names_collide() {
    let error = read_sync_metadata(
        &fixture("hostile-paths", "sync"),
        &fixture("hostile-paths", "serverinfo"),
    )
    .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Format);
    assert!(error.message().contains("not usable"), "{error}");
}

#[test]
fn wrong_files_are_a_format_error_without_content() {
    let error = read_sync_metadata(
        &fixture("small", "serverinfo"),
        &fixture("small", "serverinfo"),
    )
    .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Format);
    let error =
        read_sync_metadata(&fixture("small", "sync"), &fixture("small", "sync")).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Format);
}

#[test]
fn unsafe_names_are_left_out_with_a_warning_and_never_become_paths() {
    let hostile = [
        "..",
        "../escape.pbo",
        "/absolute.pbo",
        "\\\\host\\share\\x.pbo",
        "a/../../b.pbo",
        "back\\slash.pbo",
        "C:\\Windows\\x.pbo",
        "CON",
        "file.pbo:stream",
        "trailing.",
        "nul\u{0}byte",
        "",
    ];
    let mut files: Vec<SyncFile> = hostile.iter().map(|n| file(n, b"x")).collect();
    files.push(file("good.pbo", b"good"));
    let metadata =
        metadata_from_tree(&tree(dir("racine", vec![], files)), &info(&[]), "1").unwrap();
    let listed: Vec<_> = metadata
        .manifest
        .entries()
        .iter()
        .map(|e| e.path.as_str())
        .collect();
    assert_eq!(listed, ["good.pbo"]);
    let unsafe_count = metadata
        .warnings
        .iter()
        .filter(|w| matches!(w, SyncWarning::UnsafeFileName { .. }))
        .count();
    assert_eq!(unsafe_count, hostile.len());
    for warning in &metadata.warnings {
        if let SyncWarning::UnsafeFileName { shown, .. } = warning {
            assert!(!shown.chars().any(char::is_control), "{shown:?}");
        }
    }
}

#[test]
fn an_unsafe_folder_takes_its_files_with_it() {
    let bad = dir("..", vec![], vec![file("a.pbo", b"a"), file("b.pbo", b"b")]);
    let good = dir("@mod", vec![], vec![file("c.pbo", b"c")]);
    let metadata = metadata_from_tree(
        &tree(dir("racine", vec![bad, good], vec![])),
        &info(&[]),
        "1",
    )
    .unwrap();
    assert_eq!(metadata.manifest.entries().len(), 1);
    assert!(metadata.warnings.iter().any(|w| matches!(
        w,
        SyncWarning::UnsafeFolderName {
            files_left_out: 2,
            ..
        }
    )));
}

#[test]
fn collisions_reject_the_document_instead_of_merging() {
    for (a, b) in [("Case.pbo", "case.PBO"), ("same.pbo", "same.pbo")] {
        let files = vec![file(a, b"1"), file(b, b"2")];
        let error =
            metadata_from_tree(&tree(dir("racine", vec![], files)), &info(&[]), "1").unwrap_err();
        assert_eq!(error.kind(), ErrorKind::Format, "{a} / {b}");
    }
    // NFC and NFD spellings are different names on NTFS and ext4 (the target file systems) and are kept apart;
    // only macOS folds them, and macOS is not a target (docs/migration/SYNC_METADATA.md).
    let nfc_nfd = vec![file("caf\u{e9}.pbo", b"1"), file("cafe\u{301}.pbo", b"2")];
    let kept = metadata_from_tree(&tree(dir("racine", vec![], nfc_nfd)), &info(&[]), "1").unwrap();
    assert_eq!(kept.manifest.entries().len(), 2);
    // a file that is also a folder
    let clash = dir(
        "racine",
        vec![dir("x", vec![], vec![file("y", b"y")])],
        vec![file("x", b"x")],
    );
    assert!(metadata_from_tree(&tree(clash), &info(&[]), "1").is_err());
    assert!(metadata_from_tree(&tree(dir("racine", vec![], vec![])), &info(&[]), "").is_err());
}

#[test]
fn hidden_folders_are_protected_and_server_paths_are_normalised() {
    let mut hidden = dir("h", vec![], vec![file("a.pbo", b"a")]);
    hidden.hidden = true;
    let metadata = metadata_from_tree(
        &tree(dir("racine", vec![hidden], vec![])),
        &info(&["@x\\sub", "../bad", "@x/sub"]),
        "1",
    )
    .unwrap();
    assert_eq!(metadata.protected, [path("@x/sub"), path("h")]);
    assert!(
        metadata
            .warnings
            .contains(&SyncWarning::ServerInfoPathIgnored)
    );
    // files below a hidden folder are still part of the manifest
    assert_eq!(metadata.manifest.entries().len(), 1);
}

#[test]
fn local_sha1_comparison_drives_the_plan() {
    let root = dir(
        "racine",
        vec![dir(
            "@m",
            vec![],
            vec![
                file("same.pbo", b"same"),
                file("changed.pbo", b"new"),
                file("missing.pbo", b"m"),
            ],
        )],
        vec![],
    );
    let metadata = metadata_from_tree(&tree(root), &info(&[]), "9").unwrap();
    let hasher = Sha256Hasher;
    let local = vec![
        LocalFile {
            path: path("@m/same.pbo"),
            size: 4,
            hash: Some(hasher.hash(HashAlgorithm::Sha1, b"same")),
        },
        LocalFile {
            path: path("@m/changed.pbo"),
            size: 3,
            hash: Some(hasher.hash(HashAlgorithm::Sha1, b"old")),
        },
        LocalFile {
            path: path("@m/extra.pbo"),
            size: 1,
            hash: None,
        },
    ];
    let plan = plan_sync(&metadata.manifest, &local, &PlanOptions::default());
    assert_eq!(plan.revision, "9");
    assert!(plan.steps.contains(&SyncStep::Keep {
        path: path("@m/same.pbo")
    }));
    assert!(plan.steps.contains(&SyncStep::Replace {
        path: path("@m/changed.pbo"),
        size: 3
    }));
    assert!(plan.steps.contains(&SyncStep::Download {
        path: path("@m/missing.pbo"),
        size: 1
    }));
    assert!(
        !plan
            .steps
            .iter()
            .any(|s| matches!(s, SyncStep::Delete { .. }))
    );
    // A SHA-256 hash for the same file is a different hash: the file counts as changed, never as identical.
    let wrong_kind = vec![LocalFile {
        path: path("@m/same.pbo"),
        size: 4,
        hash: Some(hasher.hash(HashAlgorithm::Sha256, b"same")),
    }];
    let plan = plan_sync(&metadata.manifest, &wrong_kind, &PlanOptions::default());
    assert!(plan.steps.contains(&SyncStep::Replace {
        path: path("@m/same.pbo"),
        size: 4
    }));
}

#[test]
fn sha1_digest_matches_the_published_test_vectors() {
    let hasher = Sha256Hasher;
    assert_eq!(
        hasher.hash(HashAlgorithm::Sha1, b"").hex(),
        "da39a3ee5e6b4b0d3255bfef95601890afd80709"
    );
    assert_eq!(
        hasher.hash(HashAlgorithm::Sha1, b"abc").hex(),
        "a9993e364706816aba3e25717850c26c9cd0d89d"
    );
    assert_eq!(
        hasher.hash(HashAlgorithm::Sha1, &[b'a'; 1000]).hex(),
        "291e9a6c66994949b57ba5e650361e98fc36b1ba"
    );
}
