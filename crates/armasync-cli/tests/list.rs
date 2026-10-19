// SPDX-License-Identifier: AGPL-3.0-only
//! Black-box tests of `armasync repository list` and `armasync profile list`: read-only, JSON envelope, empty and
//! damaged libraries, no credentials. All data is synthetic.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "armasync-cli-list-{name}-{}-{}",
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

fn armasync(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_armasync"))
        .args(args)
        .stdin(Stdio::null())
        .output()
        .expect("armasync binary must start")
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn path(p: &Path) -> &str {
    p.to_str().unwrap()
}

/// Imports the golden neutral file so the library holds repositories and profiles.
fn populated(name: &str) -> (PathBuf, PathBuf) {
    let dir = scratch(name);
    let config = dir.join("config");
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/neutral-import/synthetic-full.json");
    let out = armasync(&["import", path(&file), "--apply", "--config", path(&config)]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    (dir, config)
}

#[test]
fn lists_repositories_and_profiles_as_text_and_json() {
    let (dir, config) = populated("full");
    let out = armasync(&["repository", "list", "--config", path(&config)]);
    assert_eq!(out.status.code(), Some(0));
    let stdout = text(&out.stdout);
    assert!(stdout.contains("https://"), "{stdout}");
    assert!(stdout.contains("credentials required"), "{stdout}");

    let out = armasync(&["--json", "repository", "list", "--config", path(&config)]);
    let value: serde_json::Value = serde_json::from_str(&text(&out.stdout)).unwrap();
    assert_eq!(value["command"], "repository-list");
    assert_eq!(value["status"], "ok");
    let repositories = value["result"]["repositories"].as_array().unwrap();
    assert!(!repositories.is_empty());
    for r in repositories {
        for key in ["id", "name", "url", "unencrypted", "credentials_required"] {
            assert!(r.get(key).is_some(), "{key}");
        }
    }

    let out = armasync(&["--json", "profile", "list", "--config", path(&config)]);
    let value: serde_json::Value = serde_json::from_str(&text(&out.stdout)).unwrap();
    assert_eq!(value["command"], "profile-list");
    assert!(!value["result"]["profiles"].as_array().unwrap().is_empty());
    let out = armasync(&["profile", "list", "--config", path(&config)]);
    assert!(text(&out.stdout).contains("repositories:"));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn listing_is_read_only_and_empty_library_is_not_an_error() {
    let dir = scratch("empty");
    let config = dir.join("config");
    let out = armasync(&["repository", "list", "--config", path(&config)]);
    assert_eq!(out.status.code(), Some(0));
    assert!(text(&out.stdout).contains("No repositories yet"));
    let out = armasync(&["--json", "profile", "list", "--config", path(&config)]);
    let value: serde_json::Value = serde_json::from_str(&text(&out.stdout)).unwrap();
    assert!(value["result"]["profiles"].as_array().unwrap().is_empty());
    assert!(!config.exists(), "listing must not create anything");

    let (populated_dir, populated_config) = populated("readonly");
    let before = fs::read(populated_config.join("library.json")).unwrap();
    let entries_before = fs::read_dir(&populated_config).unwrap().count();
    armasync(&["repository", "list", "--config", path(&populated_config)]);
    assert_eq!(
        fs::read(populated_config.join("library.json")).unwrap(),
        before
    );
    assert_eq!(
        fs::read_dir(&populated_config).unwrap().count(),
        entries_before
    );
    fs::remove_dir_all(dir).unwrap();
    fs::remove_dir_all(populated_dir).unwrap();
}

#[test]
fn damaged_library_is_a_format_error_and_untouched() {
    let dir = scratch("damaged");
    let config = dir.join("config");
    fs::create_dir_all(&config).unwrap();
    fs::write(config.join("library.json"), b"{ not json").unwrap();
    let out = armasync(&["repository", "list", "--config", path(&config)]);
    assert_eq!(out.status.code(), Some(7));
    assert!(out.stdout.is_empty());
    let json = armasync(&["--json", "profile", "list", "--config", path(&config)]);
    assert_eq!(json.status.code(), Some(7));
    let value: serde_json::Value = serde_json::from_str(&text(&json.stdout)).unwrap();
    assert_eq!(value["status"], "error");
    assert_eq!(value["error"]["kind"], "format");
    assert_eq!(
        fs::read(config.join("library.json")).unwrap(),
        b"{ not json"
    );
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn quiet_prints_nothing() {
    let (dir, config) = populated("quiet");
    let out = armasync(&["--quiet", "repository", "list", "--config", path(&config)]);
    assert_eq!(out.status.code(), Some(0));
    assert!(out.stdout.is_empty());
    fs::remove_dir_all(dir).unwrap();
}
