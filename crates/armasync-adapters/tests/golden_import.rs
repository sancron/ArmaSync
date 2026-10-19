// SPDX-License-Identifier: AGPL-3.0-only
//! The checked-in neutral import files (`fixtures/neutral-import/`) must stay valid and importable.
//! `from-legacy-local.json` is the output of the offline exporter for the MIG-001 legacy fixtures
//! (checked byte-for-byte-equivalent by `migration-tools/legacy-export/test_export.py`).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::PathBuf;

use armasync_adapters::LibraryStore;
use armasync_adapters::import::{Importer, OnConflict, parse_import};
use armasync_core::CancelToken;

fn golden(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join("neutral-import")
        .join(name)
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "armasync-golden-{name}-{}-{}",
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

#[test]
fn golden_files_parse_and_contain_no_secret_like_text() {
    for name in ["from-legacy-local.json", "synthetic-full.json"] {
        let path = golden(name);
        let bytes = fs::read(&path).unwrap();
        parse_import(&path, &bytes).unwrap();
        let text = String::from_utf8(bytes).unwrap().to_lowercase();
        for word in ["password", "login", "token", "secret"] {
            assert!(!text.contains(word), "{name}: {word}");
        }
    }
}

#[test]
fn exporter_output_imports_profiles_with_unresolved_repository_references() {
    let dir = scratch("legacy");
    let importer = Importer::new(&dir);
    let preview = importer
        .preview(&golden("from-legacy-local.json"), OnConflict::KeepBoth)
        .unwrap();
    assert_eq!(preview.plan.counts().profiles_added, 1);
    assert_eq!(preview.plan.counts().repositories_added, 0);
    assert_eq!(preview.plan.profiles[0].unresolved, ["Example SFTP"]);
    assert!(
        preview
            .plan
            .notes
            .iter()
            .any(|n| n.code == "skipped-encrypted-repository")
    );
    assert!(!dir.join("library.json").exists());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn legacy_profiles_then_repositories_then_repeat() {
    let dir = scratch("sequence");
    let importer = Importer::new(&dir);
    let cancel = CancelToken::new();
    importer
        .apply(
            &golden("from-legacy-local.json"),
            OnConflict::KeepBoth,
            &cancel,
        )
        .unwrap();
    let store = LibraryStore::new(dir.join("library.json"));
    assert!(store.load().unwrap().profiles()[0].repositories.is_empty());

    // The repositories arrive later (here from the hand-written file); replace updates the profile.
    let outcome = importer
        .apply(&golden("synthetic-full.json"), OnConflict::Replace, &cancel)
        .unwrap();
    assert_eq!(outcome.plan.counts().repositories_added, 3);
    assert_eq!(
        outcome.plan.counts().profiles_replaced,
        1,
        "Alpha now resolves its repositories"
    );
    assert!(outcome.backup.is_some());
    let library = store.load().unwrap();
    assert_eq!(library.repositories().len(), 3);
    assert_eq!(library.profiles().len(), 3);
    let alpha = library
        .profiles()
        .iter()
        .find(|p| p.name.as_str() == "Alpha")
        .unwrap();
    assert_eq!(alpha.repositories.len(), 2);
    let everything = library
        .profiles()
        .iter()
        .find(|p| p.name.as_str() == "Everything")
        .unwrap();
    assert_eq!(
        everything.repositories.len(),
        3,
        "the unknown repository is left out"
    );
    assert!(library.repository(&alpha.repositories[0]).is_some());
    assert!(outcome.plan.warnings.len() >= 3);

    let again = importer
        .apply(&golden("synthetic-full.json"), OnConflict::Replace, &cancel)
        .unwrap();
    assert!(!again.changed);
    assert_eq!(importer.journal().unwrap().len(), 2);
    fs::remove_dir_all(dir).unwrap();
}
