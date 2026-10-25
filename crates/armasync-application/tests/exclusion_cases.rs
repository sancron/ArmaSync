// SPDX-License-Identifier: AGPL-3.0-only
//! Upload exclusions (MIG-042, step 3b; ADR-0013): the preview shows every excluded file with its reason, internal
//! and partial files and explicitly excluded paths never reach a plan or a revision, and unused rules are reported.
//! In-memory source and target; the loopback tests of the adapter crate cover the real folder and server.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::{BTreeMap, BTreeSet};
use std::io::{Cursor, Read};
use std::sync::Mutex;

use armasync_adapters::Sha256Hasher;
use armasync_application::exclusion::partition;
use armasync_application::revision;
use armasync_application::upload::{self, UploadStep};
use armasync_core::exclusion::{ExclusionReason, ExclusionRules, is_internal};
use armasync_core::{
    CancelToken, Error, ErrorKind, HashAlgorithm, Hasher, InventoryEntry, LocalSource,
    OpenedSource, RelativePath, RemoteTarget, SourceFile,
};

fn path(text: &str) -> RelativePath {
    RelativePath::parse(text).unwrap()
}

/// A source folder in memory. Like the adapter, `files` never lists internal names; `inventory` lists everything.
struct FakeSource {
    contents: BTreeMap<String, Vec<u8>>,
}

impl FakeSource {
    fn with(files: &[(&str, &[u8])]) -> Self {
        Self {
            contents: files
                .iter()
                .map(|(name, bytes)| ((*name).to_owned(), bytes.to_vec()))
                .collect(),
        }
    }
}

impl LocalSource for FakeSource {
    fn files(&self, _cancel: &CancelToken) -> Result<Vec<SourceFile>, Error> {
        Ok(self
            .contents
            .iter()
            .filter(|(name, _)| !is_internal(&path(name)))
            .map(|(name, bytes)| SourceFile {
                path: path(name),
                size: bytes.len() as u64,
                hash: Sha256Hasher.hash(HashAlgorithm::Sha256, bytes),
            })
            .collect())
    }

    fn inventory(&self, _cancel: &CancelToken) -> Result<Vec<InventoryEntry>, Error> {
        Ok(self
            .contents
            .iter()
            .map(|(name, bytes)| InventoryEntry {
                path: path(name),
                size: bytes.len() as u64,
            })
            .collect())
    }

    fn open(&self, path: &RelativePath) -> Result<OpenedSource, Error> {
        let bytes = self
            .contents
            .get(path.as_str())
            .ok_or_else(|| Error::new(ErrorKind::FileSystem, "missing"))?;
        Ok(OpenedSource {
            size: bytes.len() as u64,
            content: Box::new(Cursor::new(bytes.clone())),
        })
    }
}

/// A target folder in memory that records every write.
#[derive(Default)]
struct MemoryTarget {
    files: Mutex<BTreeMap<String, Vec<u8>>>,
    folders: Mutex<BTreeSet<String>>,
    writes: Mutex<Vec<String>>,
}

impl RemoteTarget for MemoryTarget {
    fn exists(&self, path: &RelativePath) -> Result<bool, Error> {
        Ok(self.folders.lock().unwrap().contains(path.as_str())
            || self.files.lock().unwrap().contains_key(path.as_str()))
    }

    fn make_folder(&self, path: &RelativePath) -> Result<(), Error> {
        self.writes.lock().unwrap().push(format!("mkdir {path}"));
        self.folders
            .lock()
            .unwrap()
            .insert(path.as_str().to_owned());
        Ok(())
    }

    fn upload(
        &self,
        path: &RelativePath,
        mut source: OpenedSource,
        _cancel: &CancelToken,
    ) -> Result<(), Error> {
        let mut bytes = Vec::new();
        source.content.read_to_end(&mut bytes).unwrap();
        self.writes.lock().unwrap().push(format!("put {path}"));
        self.files
            .lock()
            .unwrap()
            .insert(path.as_str().to_owned(), bytes);
        Ok(())
    }

    fn fetch(&self, path: &RelativePath, _limit: u64) -> Result<Option<Vec<u8>>, Error> {
        Ok(self.files.lock().unwrap().get(path.as_str()).cloned())
    }
}

fn sample() -> FakeSource {
    FakeSource::with(&[
        ("a.txt", b"alpha"),
        (".armasync-lock", b""),
        ("mods/.armasync-partial-7.data", b"partial"),
        ("cache/.armasync-x/inner.bin", b"internal"),
        ("docs/secret.txt", b"private"),
        ("docs/open.txt", b"open"),
        ("tmp/one.txt", b"temp"),
    ])
}

fn rules() -> ExclusionRules {
    ExclusionRules::new([path("docs/secret.txt"), path("tmp"), path("nothing-here")])
}

#[test]
fn the_preview_lists_every_excluded_file_with_its_reason() {
    let split = partition(&sample(), &rules(), &CancelToken::new()).unwrap();
    let shown: Vec<(&str, &str)> = split
        .excluded
        .iter()
        .map(|e| (e.path.as_str(), e.reason.code()))
        .collect();
    assert_eq!(
        shown,
        [
            (".armasync-lock", "internal-data"),
            ("cache/.armasync-x/inner.bin", "internal-data"),
            ("docs/secret.txt", "user-rule"),
            ("mods/.armasync-partial-7.data", "partial-file"),
            ("tmp/one.txt", "user-rule"),
        ]
    );
    // The reason of a user rule names the rule that matched.
    let secret = split
        .excluded
        .iter()
        .find(|e| e.path.as_str() == "docs/secret.txt")
        .unwrap();
    assert_eq!(
        secret.reason,
        ExclusionReason::UserRule(path("docs/secret.txt"))
    );
    // Sizes are shown as well.
    assert_eq!(secret.size, 7);
}

#[test]
fn included_and_excluded_files_together_make_up_the_whole_source() {
    let source = sample();
    let split = partition(&source, &rules(), &CancelToken::new()).unwrap();
    let included: Vec<&str> = split.included.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(included, ["a.txt", "docs/open.txt"]);
    let total = source.contents.len();
    assert_eq!(included.len() + split.excluded.len(), total);
}

#[test]
fn unused_rules_are_reported_and_not_silently_ignored() {
    let split = partition(&sample(), &rules(), &CancelToken::new()).unwrap();
    assert_eq!(split.unused_rules, [path("nothing-here")]);
}

#[test]
fn a_plan_creates_no_folder_for_an_excluded_only_folder() {
    let source = sample();
    let target = MemoryTarget::default();
    let plan = upload::plan_with_rules(&source, &target, &rules(), &CancelToken::new()).unwrap();
    let steps: Vec<&UploadStep> = plan.steps.iter().collect();
    assert!(
        plan.steps
            .iter()
            .all(|step| !matches!(step, UploadStep::MakeFolder { path } if path.as_str() == "tmp" || path.as_str() == "cache" || path.as_str() == "mods")),
        "{steps:?}"
    );
    assert_eq!(plan.files, 2);
    assert_eq!(plan.folders, 1, "only docs is needed");
    assert_eq!(plan.exclusions.len(), 5);
    assert_eq!(plan.unused_rules, [path("nothing-here")]);
}

#[test]
fn an_apply_writes_only_the_included_files() {
    let source = sample();
    let target = MemoryTarget::default();
    let cancel = CancelToken::new();
    let plan = upload::plan_with_rules(&source, &target, &rules(), &cancel).unwrap();
    upload::apply(&plan, &source, &target, &cancel).unwrap();
    let writes = target.writes.lock().unwrap().clone();
    assert_eq!(writes, ["mkdir docs", "put a.txt", "put docs/open.txt"]);
    let stored: Vec<String> = target.files.lock().unwrap().keys().cloned().collect();
    assert!(
        stored.iter().all(|name| !is_internal(&path(name))),
        "{stored:?}"
    );
    assert!(!stored.contains(&"docs/secret.txt".to_owned()));
    assert!(!stored.contains(&"tmp/one.txt".to_owned()));
}

#[test]
fn the_revision_and_its_manifest_never_contain_an_excluded_path() {
    let source = sample();
    let target = MemoryTarget::default();
    let report = revision::publish_with_rules(
        &source,
        &target,
        &Sha256Hasher,
        &rules(),
        &CancelToken::new(),
    )
    .unwrap();
    assert_eq!(report.files_uploaded, 2);
    let manifest = target
        .files
        .lock()
        .unwrap()
        .get(&format!("revisions/{}/manifest.json", report.revision))
        .cloned()
        .unwrap();
    let text = String::from_utf8(manifest).unwrap();
    for excluded in [".armasync", "secret.txt", "tmp/one.txt", "docs/open.txt"] {
        assert_eq!(
            text.contains(excluded),
            excluded == "docs/open.txt",
            "{excluded} in {text}"
        );
    }
    assert_eq!(report.exclusions.len(), 5);
    assert_eq!(report.unused_rules, [path("nothing-here")]);
}

#[test]
fn a_source_with_only_excluded_files_publishes_an_empty_revision() {
    let source = FakeSource::with(&[(".armasync-lock", b""), ("tmp/x.txt", b"x")]);
    let target = MemoryTarget::default();
    let report = revision::publish_with_rules(
        &source,
        &target,
        &Sha256Hasher,
        &ExclusionRules::new([path("tmp")]),
        &CancelToken::new(),
    )
    .unwrap();
    assert_eq!(report.files_uploaded, 0);
    assert_eq!(report.exclusions.len(), 2);
}

#[test]
fn a_rule_cannot_escape_the_source_root() {
    // Rules are validated paths: a rule that would leave the root cannot be built at all.
    assert!(RelativePath::parse("../secret.txt").is_err());
    assert!(RelativePath::parse("/etc/passwd").is_err());
}

#[test]
fn duplicate_rules_are_collapsed_into_one() {
    let rules = ExclusionRules::new([path("tmp"), path("tmp")]);
    assert_eq!(rules.rules().len(), 1);
}
