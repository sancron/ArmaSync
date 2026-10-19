// SPDX-License-Identifier: AGPL-3.0-only
//! End-to-end tests of the server auto-configuration import (MIG-022) against the synthetic MIG-001 fixtures:
//! preview writes nothing, apply is atomic and repeatable, no secret reaches the library, backup or journal.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use armasync_adapters::LibraryStore;
use armasync_adapters::import::{Action, Importer, OnConflict, Warning};
use armasync_core::{CancelToken, ErrorKind};
use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;

const SECRETS: [&str; 3] = [
    "fixture-user",
    "fixture-password-not-a-secret",
    "fixture-server-password-not-a-secret",
];

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "armasync-server-import-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
    ));
    // Unique even if the clock is coarse (two tests can read the same timestamp): `create_dir` fails when the
    // name exists, then the next suffix is tried.
    let mut candidate = dir.clone();
    let mut attempt = 0u32;
    while fs::create_dir(&candidate).is_err() {
        attempt += 1;
        assert!(attempt < 1000, "cannot create a unique scratch folder");
        candidate = PathBuf::from(format!("{}-{attempt}", dir.display()));
    }
    candidate
}

fn fixture(scenario: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/legacy-formats/generated/valid/server")
        .join(scenario)
        .join(".a3s/autoconfig")
}

fn all_files(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries {
            let path = entry.unwrap().path();
            if path.is_dir() {
                found.extend(all_files(&path));
            } else {
                found.push(path);
            }
        }
    }
    found
}

fn assert_no_secret_on_disk(dir: &Path) {
    for file in all_files(dir) {
        let content = String::from_utf8_lossy(&fs::read(&file).unwrap()).into_owned();
        for secret in SECRETS {
            assert!(
                !content.contains(secret),
                "{secret} found in {}",
                file.display()
            );
        }
    }
}

/// Decompresses a fixture, replaces `from` by `to` (same length) and compresses again.
fn patched(path: &Path, from: &[u8], to: &[u8]) -> Vec<u8> {
    assert_eq!(from.len(), to.len());
    let mut raw = Vec::new();
    GzDecoder::new(fs::read(path).unwrap().as_slice())
        .read_to_end(&mut raw)
        .unwrap();
    let at = raw
        .windows(from.len())
        .position(|w| w == from)
        .expect("pattern present");
    raw[at..at + from.len()].copy_from_slice(to);
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&raw).unwrap();
    encoder.finish().unwrap()
}

#[test]
fn preview_writes_nothing_and_shows_the_repository() {
    let root = scratch("preview");
    let config = root.join("config");
    let preview = Importer::new(&config)
        .preview_autoconfig(&fixture("small"), OnConflict::KeepBoth)
        .unwrap();
    assert!(!config.exists());
    assert!(!preview.already_imported);
    let [item] = preview.plan.repositories.as_slice() else {
        panic!("one repository");
    };
    assert_eq!(item.action, Action::Add);
    assert_eq!(item.id.as_str(), "example-http");
    assert!(preview.plan.profiles.is_empty());
    assert!(
        preview
            .plan
            .warnings
            .iter()
            .any(|w| matches!(w, Warning::CredentialsNeeded { .. }))
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn apply_stores_address_without_credentials_and_repeat_changes_nothing() {
    let root = scratch("apply");
    let config = root.join("config");
    let importer = Importer::new(&config);
    let outcome = importer
        .apply_autoconfig(&fixture("small"), OnConflict::KeepBoth, &CancelToken::new())
        .unwrap();
    assert!(outcome.changed);
    let library = LibraryStore::new(config.join("library.json"))
        .load()
        .unwrap();
    let [repository] = library.repositories() else {
        panic!("one repository");
    };
    assert_eq!(repository.url.as_str(), "https://repo.example.invalid");
    assert_no_secret_on_disk(&config);

    let before = fs::read(config.join("library.json")).unwrap();
    let again = importer
        .apply_autoconfig(&fixture("small"), OnConflict::KeepBoth, &CancelToken::new())
        .unwrap();
    assert!(!again.changed);
    assert_eq!(fs::read(config.join("library.json")).unwrap(), before);
    let journal = importer.journal().unwrap();
    assert_eq!(journal.len(), 1);
    assert_eq!(journal[0].source_tool, "armasync-server-metadata");
    // The other two fixture scenarios publish the same repository: still nothing new.
    for scenario in ["empty", "hostile-paths"] {
        let outcome = importer
            .apply_autoconfig(
                &fixture(scenario),
                OnConflict::KeepBoth,
                &CancelToken::new(),
            )
            .unwrap();
        assert!(!outcome.changed, "{scenario}");
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn existing_library_is_backed_up_and_kept() {
    let root = scratch("backup");
    let config = root.join("config");
    let importer = Importer::new(&config);
    importer
        .apply_autoconfig(&fixture("small"), OnConflict::KeepBoth, &CancelToken::new())
        .unwrap();
    // Same name, different address: the id collides, so the new one is added next to the existing one.
    let other = root.join("autoconfig-other");
    fs::write(
        &other,
        patched(
            &fixture("small"),
            b"repo.example.invalid",
            b"rep0.example.invalid",
        ),
    )
    .unwrap();
    let outcome = importer
        .apply_autoconfig(&other, OnConflict::KeepBoth, &CancelToken::new())
        .unwrap();
    assert!(outcome.changed);
    let backup = outcome.backup.expect("a backup of the previous library");
    assert!(backup.exists());
    let library = LibraryStore::new(config.join("library.json"))
        .load()
        .unwrap();
    let ids: Vec<_> = library
        .repositories()
        .iter()
        .map(|r| r.id.as_str().to_owned())
        .collect();
    assert_eq!(ids, ["example-http", "example-http-2"]);
    assert_no_secret_on_disk(&config);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn proxy_type_is_unsupported_and_nothing_is_written() {
    let root = scratch("socks");
    let config = root.join("config");
    let file = root.join("autoconfig");
    fs::write(&file, patched(&fixture("small"), b"HTTPS", b"SOCKS")).unwrap();
    let error = Importer::new(&config)
        .apply_autoconfig(&file, OnConflict::KeepBoth, &CancelToken::new())
        .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Unsupported);
    assert!(!config.exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn hostile_inputs_are_rejected_without_touching_the_library() {
    let root = scratch("hostile");
    let config = root.join("config");
    let importer = Importer::new(&config);
    importer
        .apply_autoconfig(&fixture("small"), OnConflict::KeepBoth, &CancelToken::new())
        .unwrap();
    let before = fs::read(config.join("library.json")).unwrap();
    let invalid = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/legacy-formats/generated/invalid");
    for entry in fs::read_dir(invalid).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "zip") {
            continue;
        }
        let error = importer
            .apply_autoconfig(&path, OnConflict::KeepBoth, &CancelToken::new())
            .unwrap_err();
        assert_eq!(error.kind(), ErrorKind::Format, "{}", path.display());
    }
    let missing = importer
        .preview_autoconfig(&root.join("does-not-exist"), OnConflict::KeepBoth)
        .unwrap_err();
    assert_eq!(missing.kind(), ErrorKind::Usage);
    assert_eq!(fs::read(config.join("library.json")).unwrap(), before);
    assert_eq!(importer.journal().unwrap().len(), 1);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn cancelled_import_changes_nothing() {
    let root = scratch("cancel");
    let config = root.join("config");
    let cancel = CancelToken::new();
    cancel.cancel();
    let error = Importer::new(&config)
        .apply_autoconfig(&fixture("small"), OnConflict::KeepBoth, &cancel)
        .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Cancelled);
    assert!(!config.exists());
    fs::remove_dir_all(root).unwrap();
}
