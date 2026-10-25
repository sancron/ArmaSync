// SPDX-License-Identifier: AGPL-3.0-only
//! Upload exclusions on a real folder and the loopback WebDAV server (MIG-042, step 3b; ADR-0013). The preview lists
//! every excluded file with its reason, nothing internal or excluded reaches the server, and the source folder is not
//! changed by listing it. Synthetic data and credentials only.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;
use support::trust;

use std::fs;
use std::path::{Path, PathBuf};

use armasync_adapters::Sha256Hasher;
use armasync_adapters::localfs::LocalDirectory;
use armasync_adapters::webdav::WebDavClient;
use armasync_application::revision;
use armasync_application::upload::{self, UploadStep};
use armasync_core::CancelToken;
use armasync_core::RelativePath;
use armasync_core::exclusion::{ExclusionReason, ExclusionRules};
use support::{credentials, start};

fn path(text: &str) -> RelativePath {
    RelativePath::parse(text).unwrap()
}

fn tree(name: &str) -> PathBuf {
    let root =
        std::env::temp_dir().join(format!("armasync-exclusion-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let files: [(&str, &[u8]); 7] = [
        ("a.txt", b"alpha"),
        ("docs/open.txt", b"open"),
        ("docs/secret.txt", b"private"),
        (".armasync-lock", b""),
        ("mods/.armasync-partial-1.data", b"partial"),
        ("cache/.armasync-x/inner.bin", b"internal"),
        ("tmp/one.txt", b"temp"),
    ];
    for (relative, bytes) in files {
        let target = root.join(relative);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(target, bytes).unwrap();
    }
    root
}

fn rules() -> ExclusionRules {
    ExclusionRules::new([path("docs/secret.txt"), path("tmp")])
}

/// Every file below `root`, relative and sorted, to show that listing changes nothing.
fn listing(root: &Path) -> Vec<String> {
    fn walk(dir: &Path, prefix: &str, out: &mut Vec<String>) {
        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let name = format!("{prefix}{}", entry.file_name().to_string_lossy());
            if entry.file_type().unwrap().is_dir() {
                walk(&entry.path(), &format!("{name}/"), out);
            } else {
                out.push(name);
            }
        }
    }
    let mut out = Vec::new();
    walk(root, "", &mut out);
    out.sort();
    out
}

#[test]
fn the_preview_shows_every_exclusion_and_only_the_rest_reaches_the_server() {
    let root = tree("preview");
    let source = LocalDirectory::new(&root).unwrap();
    let server = start(true, true, "");
    let target = WebDavClient::new(&server.https(), trust(&server), Some(credentials())).unwrap();
    let cancel = CancelToken::new();

    let plan = upload::plan_with_rules(&source, &target, &rules(), &cancel).unwrap();
    let shown: Vec<(&str, &str)> = plan
        .exclusions
        .iter()
        .map(|e| (e.path.as_str(), e.reason.code()))
        .collect();
    assert_eq!(
        shown,
        [
            (".armasync-lock", "internal-data"),
            ("cache/.armasync-x/inner.bin", "internal-data"),
            ("docs/secret.txt", "user-rule"),
            ("mods/.armasync-partial-1.data", "partial-file"),
            ("tmp/one.txt", "user-rule"),
        ]
    );
    assert_eq!(
        plan.exclusions
            .iter()
            .find(|e| e.path.as_str() == "docs/secret.txt")
            .unwrap()
            .reason,
        ExclusionReason::UserRule(path("docs/secret.txt"))
    );
    // Only a.txt and docs/open.txt are uploaded; the cache folder needs no folder on the server.
    assert_eq!(plan.files, 2);
    assert!(plan.steps.iter().all(|s| !matches!(
        s,
        UploadStep::MakeFolder { path } if path.as_str() != "docs"
    )));

    upload::apply(&plan, &source, &target, &cancel).unwrap();
    assert_eq!(
        server.file_names(),
        ["a.txt", "docs/open.txt"],
        "nothing internal or excluded may reach the server"
    );
    fs::remove_dir_all(root).ok();
}

#[test]
fn listing_and_previewing_a_folder_changes_nothing_in_it() {
    let root = tree("unchanged");
    // A staging leftover that an older scan removed: the preview must keep it.
    fs::write(root.join(".armasync-stale-1.tmp"), b"left over").unwrap();
    let before = listing(&root);
    let source = LocalDirectory::new(&root).unwrap();
    let server = start(true, true, "");
    let target = WebDavClient::new(&server.https(), trust(&server), Some(credentials())).unwrap();
    let plan = upload::plan_with_rules(&source, &target, &rules(), &CancelToken::new()).unwrap();
    assert!(
        plan.exclusions
            .iter()
            .any(|e| e.path.as_str() == ".armasync-stale-1.tmp")
    );
    assert_eq!(listing(&root), before);
    fs::remove_dir_all(root).ok();
}

#[test]
fn a_published_revision_contains_no_excluded_file() {
    let root = tree("revision");
    let source = LocalDirectory::new(&root).unwrap();
    let server = start(true, true, "");
    let target = WebDavClient::new(&server.https(), trust(&server), Some(credentials())).unwrap();
    let report = revision::publish_with_rules(
        &source,
        &target,
        &Sha256Hasher,
        &rules(),
        &CancelToken::new(),
    )
    .unwrap();
    assert_eq!(report.files_uploaded, 2);
    let names = server.file_names();
    assert!(
        names
            .iter()
            .all(|n| !n.contains(".armasync") && !n.contains("secret") && !n.contains("tmp/")),
        "{names:?}"
    );
    let manifest = String::from_utf8(
        server
            .file(&format!("revisions/{}/manifest.json", report.revision))
            .unwrap(),
    )
    .unwrap();
    assert!(
        !manifest.contains("secret") && !manifest.contains(".armasync"),
        "{manifest}"
    );
    fs::remove_dir_all(root).ok();
}

#[test]
fn a_rule_that_matches_nothing_is_reported_in_the_preview() {
    let root = tree("unused");
    let source = LocalDirectory::new(&root).unwrap();
    let server = start(true, true, "");
    let target = WebDavClient::new(&server.https(), trust(&server), Some(credentials())).unwrap();
    let rules = ExclusionRules::new([path("does-not-exist")]);
    let plan = upload::plan_with_rules(&source, &target, &rules, &CancelToken::new()).unwrap();
    assert_eq!(plan.unused_rules, [path("does-not-exist")]);
    assert_eq!(
        plan.files, 4,
        "without the rule the user files are uploaded"
    );
    fs::remove_dir_all(root).ok();
}
