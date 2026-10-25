// SPDX-License-Identifier: AGPL-3.0-only
//! Process-level tests of `armasync upload` (MIG-042, step 4a; ADR-0014): the built binary talks to the loopback
//! WebDAV server of the test kit over anonymous HTTP. No test root and no TLS bypass are involved; the server is
//! reached on 127.0.0.1 only. Exit status, output and the published pointer are checked.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

use armasync_adapters::Sha256Hasher;
use armasync_core::{HashAlgorithm, Hasher};
use armasync_testkit::webdav::{Server, start};
use serde_json::Value;

fn source(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("armasync-process-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(root.join("a.txt"), b"alpha content").unwrap();
    fs::write(root.join("docs/b.txt"), b"beta").unwrap();
    fs::write(root.join(".armasync-lock"), b"").unwrap();
    root
}

fn armasync(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_armasync"))
        .args(args)
        .output()
        .expect("the armasync binary starts")
}

fn target(server: &Server) -> String {
    format!("http://{}/dav", server.addr)
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).unwrap()
}

/// The `manifest_sha256` value named in a pointer.
fn pointer_field(pointer: &Value, field: &str) -> String {
    pointer[field].as_str().unwrap().to_owned()
}

#[test]
fn an_anonymous_publish_through_the_binary_exits_0_and_publishes_a_verifiable_pointer() {
    let root = source("publish");
    let server = start(false, false, "");
    let output = armasync(&[
        "--json",
        "upload",
        "publish",
        "--source",
        root.to_str().unwrap(),
        "--target",
        &target(&server),
        "--yes",
    ]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}\nstdout: {}",
        text(&output.stderr),
        text(&output.stdout)
    );
    assert!(output.stderr.is_empty(), "{}", text(&output.stderr));

    // The output is one versioned envelope with the result of the publication.
    let envelope: Value = serde_json::from_str(text(&output.stdout).trim()).unwrap();
    assert_eq!(envelope["status"], "ok", "{envelope}");
    assert_eq!(envelope["command"], "upload-publish");
    let result = &envelope["result"];
    assert_eq!(result["pointer_changed"], true, "{envelope}");
    assert_eq!(result["files_uploaded"], 2, "{envelope}");
    assert_eq!(result["excluded"][0]["path"], ".armasync-lock");
    assert_eq!(result["excluded"][0]["reason"], "internal-data");
    let revision = result["revision"].as_str().unwrap().to_owned();
    assert!(revision.starts_with("r-"), "{revision}");

    // The published pointer names exactly this revision and the hash of its manifest.
    let pointer_bytes = server.file("current.json").expect("pointer published");
    let pointer: Value = serde_json::from_slice(&pointer_bytes).unwrap();
    assert_eq!(pointer["schema"], "armasync.current");
    assert_eq!(pointer["revision"], revision.as_str());
    let manifest = server
        .file(&format!("revisions/{revision}/manifest.json"))
        .expect("manifest published");
    assert_eq!(
        pointer_field(&pointer, "manifest_sha256"),
        Sha256Hasher.hash(HashAlgorithm::Sha256, &manifest).hex()
    );
    // The files are there, byte for byte, and nothing internal reached the server.
    assert_eq!(
        server
            .file(&format!("revisions/{revision}/files/a.txt"))
            .unwrap(),
        b"alpha content"
    );
    assert_eq!(
        server
            .file(&format!("revisions/{revision}/files/docs/b.txt"))
            .unwrap(),
        b"beta"
    );
    assert!(server.file_names().iter().all(|n| !n.contains(".armasync")));
    fs::remove_dir_all(root).ok();
}

#[test]
fn a_second_publish_through_the_binary_changes_nothing_and_says_so() {
    let root = source("again");
    let server = start(false, false, "");
    let args = [
        "--json",
        "upload",
        "publish",
        "--source",
        root.to_str().unwrap(),
        "--target",
        &target(&server),
        "--yes",
    ];
    assert_eq!(armasync(&args).status.code(), Some(0));
    let pointer_before = server.file("current.json").unwrap();
    let puts_before = server.puts();

    let output = armasync(&args);
    assert_eq!(output.status.code(), Some(0));
    let envelope: Value = serde_json::from_str(text(&output.stdout).trim()).unwrap();
    assert_eq!(envelope["result"]["pointer_changed"], false, "{envelope}");
    assert_eq!(
        envelope["result"]["revision_was_complete"], true,
        "{envelope}"
    );
    assert_eq!(server.puts(), puts_before, "nothing may be written");
    assert_eq!(server.file("current.json").unwrap(), pointer_before);
    fs::remove_dir_all(root).ok();
}

#[test]
fn the_text_preview_through_the_binary_lists_exclusions_and_writes_nothing() {
    let root = source("preview");
    let server = start(false, false, "");
    let output = armasync(&[
        "upload",
        "plan",
        "--source",
        root.to_str().unwrap(),
        "--target",
        &target(&server),
        "--exclude",
        "docs",
    ]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let out = text(&output.stdout);
    assert!(out.starts_with("Preview: 1 files"), "{out}");
    assert!(
        out.contains("excluded: .armasync-lock (internal ArmaSync data)"),
        "{out}"
    );
    assert!(
        out.contains("excluded: docs/b.txt (excluded by 'docs')"),
        "{out}"
    );
    assert_eq!(server.puts(), 0, "a preview writes nothing");
    assert!(server.file("current.json").is_none());
    fs::remove_dir_all(root).ok();
}

#[test]
fn a_publish_without_yes_through_the_binary_exits_3_and_writes_nothing() {
    let root = source("no-yes");
    let server = start(false, false, "");
    let output = armasync(&[
        "upload",
        "publish",
        "--source",
        root.to_str().unwrap(),
        "--target",
        &target(&server),
    ]);
    assert_eq!(output.status.code(), Some(3));
    assert!(
        text(&output.stderr).contains("--yes"),
        "{}",
        text(&output.stderr)
    );
    assert!(output.stdout.is_empty());
    assert!(
        server.seen().is_empty(),
        "no connection is made without --yes"
    );
    fs::remove_dir_all(root).ok();
}

#[test]
fn the_pointer_stays_unchanged_when_the_source_cannot_be_read() {
    let server = start(false, false, "");
    let output = armasync(&[
        "upload",
        "publish",
        "--source",
        "X:/definitely/not/here-armasync-process",
        "--target",
        &target(&server),
        "--yes",
    ]);
    assert_eq!(output.status.code(), Some(5), "{}", text(&output.stderr));
    assert!(server.file("current.json").is_none());
}
