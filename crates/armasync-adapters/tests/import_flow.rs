// SPDX-License-Identifier: AGPL-3.0-only
//! End-to-end tests of the neutral import: preview, apply, repeat, backup, abort, damaged targets, hostile input.
//! All data is synthetic; the import source and the existing library are checked for unintended changes.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::{Path, PathBuf};

use armasync_adapters::LibraryStore;
use armasync_adapters::import::{Action, Importer, OnConflict, parse_import};
use armasync_core::{CancelToken, ErrorKind};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "armasync-import-{name}-{}-{}",
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

const SOURCE: &str = r#"{
  "kind": "armasync.import",
  "schema_version": 1,
  "source": { "tool": "test-exporter", "tool_version": "1", "format": "synthetic" },
  "repositories": [
    { "key": "Example HTTP", "name": "Example HTTP", "url": "https://repo.example.invalid/arma", "needs_credentials": true },
    { "key": "Example SFTP", "name": "Example SFTP", "url": "sftp://sftp.example.invalid:2222/data" }
  ],
  "profiles": [
    { "name": "Alpha", "repositories": ["Example SFTP", "Example HTTP", "Not exported"] },
    { "name": "Empty", "repositories": [] }
  ],
  "notes": [ { "code": "skipped-encrypted-repository", "subject": "Hidden", "count": 1 } ]
}"#;

struct Setup {
    root: PathBuf,
    config: PathBuf,
    source: PathBuf,
}

fn setup(name: &str) -> Setup {
    let root = scratch(name);
    let config = root.join("config");
    let source = root.join("export.json");
    fs::write(&source, SOURCE).unwrap();
    Setup {
        root,
        config,
        source,
    }
}

fn names(dir: &Path) -> Vec<String> {
    if !dir.exists() {
        return Vec::new();
    }
    let mut v: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    v.sort();
    v
}

#[test]
fn preview_reports_the_plan_and_writes_nothing() {
    let s = setup("preview");
    let importer = Importer::new(&s.config);
    let preview = importer.preview(&s.source, OnConflict::KeepBoth).unwrap();
    let counts = preview.plan.counts();
    assert_eq!((counts.repositories_added, counts.profiles_added), (2, 2));
    assert!(!preview.already_imported);
    assert!(
        !s.config.exists(),
        "a preview must not even create the config directory"
    );
    assert_eq!(preview.plan.profiles[0].unresolved, ["Not exported"]);
    assert_eq!(preview.plan.notes[0].code, "skipped-encrypted-repository");
    assert!(
        preview
            .plan
            .repositories
            .iter()
            .all(|i| i.action == Action::Add)
    );
    fs::remove_dir_all(s.root).unwrap();
}

#[test]
fn apply_writes_library_and_journal_and_a_repeat_changes_nothing() {
    let s = setup("apply");
    let source_before = fs::read(&s.source).unwrap();
    let importer = Importer::new(&s.config);
    let outcome = importer
        .apply(&s.source, OnConflict::KeepBoth, &CancelToken::new())
        .unwrap();
    assert!(outcome.changed);
    assert!(outcome.backup.is_none(), "there was no library to back up");
    assert!(outcome.journal_warning.is_none());
    let library = LibraryStore::new(s.config.join("library.json"))
        .load()
        .unwrap();
    assert_eq!(library.repositories().len(), 2);
    assert_eq!(library.profiles().len(), 2);
    assert_eq!(library.repositories()[0].id.as_str(), "example-http");
    assert!(library.repositories()[0].credential.is_some());
    assert_eq!(library.profiles()[0].repositories.len(), 2);
    assert_eq!(names(&s.config), ["import-journal.json", "library.json"]);
    let journal = importer.journal().unwrap();
    assert_eq!(journal.len(), 1);
    assert_eq!(journal[0].repositories_added, 2);
    assert_eq!(journal[0].source_sha256, outcome.source_sha256);

    // The journal holds counts and a hash only: no names, addresses or credentials.
    let journal_text = fs::read_to_string(s.config.join("import-journal.json")).unwrap();
    for forbidden in ["Example", "example.invalid", "sftp", "password", "Alpha"] {
        assert!(!journal_text.contains(forbidden), "{forbidden}");
    }

    // Repeat: nothing changes, no new backup, no new journal entry, and the preview knows the file.
    let library_bytes = fs::read(s.config.join("library.json")).unwrap();
    let preview = importer.preview(&s.source, OnConflict::KeepBoth).unwrap();
    assert!(preview.already_imported);
    let again = importer
        .apply(&s.source, OnConflict::KeepBoth, &CancelToken::new())
        .unwrap();
    assert!(!again.changed && again.backup.is_none());
    assert_eq!(
        fs::read(s.config.join("library.json")).unwrap(),
        library_bytes
    );
    assert_eq!(importer.journal().unwrap().len(), 1);
    assert_eq!(names(&s.config), ["import-journal.json", "library.json"]);

    assert_eq!(
        fs::read(&s.source).unwrap(),
        source_before,
        "the source is never modified"
    );
    fs::remove_dir_all(s.root).unwrap();
}

#[test]
fn apply_backs_up_the_existing_library_byte_for_byte() {
    let s = setup("backup");
    let importer = Importer::new(&s.config);
    importer
        .apply(&s.source, OnConflict::KeepBoth, &CancelToken::new())
        .unwrap();
    // A second, different document is added to the now existing library.
    let second = s.root.join("second.json");
    fs::write(
        &second,
        SOURCE
            .replace("Example HTTP", "Another")
            .replace("repo.example.invalid", "other.example.invalid"),
    )
    .unwrap();
    let before = fs::read(s.config.join("library.json")).unwrap();
    let outcome = importer
        .apply(&second, OnConflict::KeepBoth, &CancelToken::new())
        .unwrap();
    let backup = outcome.backup.expect("backup of the existing library");
    assert_eq!(fs::read(&backup).unwrap(), before);
    assert!(
        backup
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("library.pre-import-")
    );
    assert_eq!(importer.journal().unwrap().len(), 2);
    fs::remove_dir_all(s.root).unwrap();
}

#[test]
fn cancelled_import_leaves_everything_untouched() {
    let s = setup("cancel");
    let importer = Importer::new(&s.config);
    importer
        .apply(&s.source, OnConflict::KeepBoth, &CancelToken::new())
        .unwrap();
    let second = s.root.join("second.json");
    fs::write(
        &second,
        SOURCE
            .replace("repo.example.invalid", "other.example.invalid")
            .replace("sftp.example.invalid", "x.example.invalid"),
    )
    .unwrap();
    let files_before = names(&s.config);
    let library_before = fs::read(s.config.join("library.json")).unwrap();
    let journal_before = fs::read(s.config.join("import-journal.json")).unwrap();
    let cancel = CancelToken::new();
    cancel.cancel();
    let error = importer
        .apply(&second, OnConflict::KeepBoth, &cancel)
        .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Cancelled);
    assert_eq!(
        names(&s.config),
        files_before,
        "no backup, no temporary files"
    );
    assert_eq!(
        fs::read(s.config.join("library.json")).unwrap(),
        library_before
    );
    assert_eq!(
        fs::read(s.config.join("import-journal.json")).unwrap(),
        journal_before
    );
    fs::remove_dir_all(s.root).unwrap();
}

#[test]
fn failing_commit_keeps_the_library_and_reports_the_failure() {
    let s = setup("failcommit");
    let importer = Importer::new(&s.config);
    importer
        .apply(&s.source, OnConflict::KeepBoth, &CancelToken::new())
        .unwrap();
    let second = s.root.join("second.json");
    fs::write(
        &second,
        SOURCE
            .replace("repo.example.invalid", "other.example.invalid")
            .replace("sftp.example.invalid", "x.example.invalid"),
    )
    .unwrap();
    // A directory in the place of the store's backup file makes the commit fail.
    fs::create_dir(s.config.join("library.json.bak")).unwrap();
    let before = fs::read(s.config.join("library.json")).unwrap();
    let error = importer
        .apply(&second, OnConflict::KeepBoth, &CancelToken::new())
        .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::FileSystem);
    assert_eq!(fs::read(s.config.join("library.json")).unwrap(), before);
    assert_eq!(
        importer.journal().unwrap().len(),
        1,
        "no journal entry for a failed import"
    );
    assert!(
        !names(&s.config).iter().any(|n| n.starts_with('.')),
        "temporary files are removed"
    );
    fs::remove_dir_all(s.root).unwrap();
}

#[test]
fn damaged_library_is_never_overwritten() {
    let s = setup("damaged");
    fs::create_dir_all(&s.config).unwrap();
    fs::write(s.config.join("library.json"), b"{ damaged").unwrap();
    let importer = Importer::new(&s.config);
    for result in [
        importer
            .preview(&s.source, OnConflict::KeepBoth)
            .map(|_| ()),
        importer
            .apply(&s.source, OnConflict::KeepBoth, &CancelToken::new())
            .map(|_| ()),
    ] {
        assert_eq!(result.unwrap_err().kind(), ErrorKind::Format);
    }
    assert_eq!(
        fs::read(s.config.join("library.json")).unwrap(),
        b"{ damaged"
    );
    assert_eq!(names(&s.config), ["library.json"]);
    fs::remove_dir_all(s.root).unwrap();
}

#[test]
fn damaged_journal_is_reported_but_does_not_undo_the_import() {
    let s = setup("badjournal");
    fs::create_dir_all(&s.config).unwrap();
    fs::write(s.config.join("import-journal.json"), b"garbage").unwrap();
    let importer = Importer::new(&s.config);
    let outcome = importer
        .apply(&s.source, OnConflict::KeepBoth, &CancelToken::new())
        .unwrap();
    assert!(outcome.changed);
    assert!(outcome.journal_warning.is_some());
    assert_eq!(
        fs::read(s.config.join("import-journal.json")).unwrap(),
        b"garbage"
    );
    assert_eq!(
        LibraryStore::new(s.config.join("library.json"))
            .load()
            .unwrap()
            .repositories()
            .len(),
        2
    );
    fs::remove_dir_all(s.root).unwrap();
}

#[test]
fn conflict_policy_fail_changes_nothing() {
    let s = setup("conflict");
    let importer = Importer::new(&s.config);
    importer
        .apply(&s.source, OnConflict::KeepBoth, &CancelToken::new())
        .unwrap();
    let other = s.root.join("other.json");
    fs::write(
        &other,
        SOURCE.replace("repo.example.invalid", "other.example.invalid"),
    )
    .unwrap();
    let before = names(&s.config);
    let library_before = fs::read(s.config.join("library.json")).unwrap();
    let error = importer
        .apply(&other, OnConflict::Fail, &CancelToken::new())
        .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Config);
    assert_eq!(names(&s.config), before);
    assert_eq!(
        fs::read(s.config.join("library.json")).unwrap(),
        library_before
    );
    fs::remove_dir_all(s.root).unwrap();
}

#[test]
fn missing_or_oversized_sources_are_reported() {
    let s = setup("source");
    let importer = Importer::new(&s.config);
    let missing = importer
        .preview(&s.root.join("nope.json"), OnConflict::KeepBoth)
        .unwrap_err();
    assert_eq!(missing.kind(), ErrorKind::Usage);
    let huge = s.root.join("huge.json");
    fs::write(&huge, vec![b' '; 4 * 1024 * 1024 + 1]).unwrap();
    assert_eq!(
        importer
            .preview(&huge, OnConflict::KeepBoth)
            .unwrap_err()
            .kind(),
        ErrorKind::Format
    );
    assert!(!s.config.exists());
    fs::remove_dir_all(s.root).unwrap();
}

fn header(extra: &str) -> String {
    format!(
        r#"{{"kind":"armasync.import","schema_version":1,"source":{{"tool":"t","tool_version":"1","format":"f"}}{extra}}}"#
    )
}

fn parse(text: &str) -> Result<(), armasync_core::Error> {
    parse_import(Path::new("t.json"), text.as_bytes()).map(|_| ())
}

#[test]
fn hostile_or_invalid_documents_are_rejected_without_echoing_secrets() {
    let repo = |extra: &str| {
        header(&format!(
            r#","repositories":[{{"key":"k","name":"n","url":"https://a.example/"{extra}}}]"#
        ))
    };
    let cases: Vec<(&str, String)> = vec![
        ("password field", repo(r#","password":"hunter2-SECRET""#)),
        ("login field", repo(r#","login":"someone""#)),
        (
            "token field in nested object",
            header(r#","notes":[{"code":"x","extra":{"api_token":"abc"}}]"#),
        ),
        (
            "credentials in url",
            header(
                r#","repositories":[{"key":"k","name":"n","url":"https://user:pw-SECRET@a.example/"}]"#,
            ),
        ),
        (
            "query in url",
            header(
                r#","repositories":[{"key":"k","name":"n","url":"https://a.example/?t=SECRET"}]"#,
            ),
        ),
        (
            "file url",
            header(r#","repositories":[{"key":"k","name":"n","url":"file:///etc/passwd"}]"#),
        ),
        ("unknown field", header(r#","extra":1"#)),
        ("unknown repository field", repo(r#","path":"/x""#)),
        (
            "duplicate keys",
            header(
                r#","repositories":[{"key":"k","name":"a","url":"https://a.example/"},{"key":"k","name":"b","url":"https://b.example/"}]"#,
            ),
        ),
        (
            "duplicate profile names",
            header(r#","profiles":[{"name":"p"},{"name":"p"}]"#),
        ),
        (
            "profile lists a key twice",
            header(r#","profiles":[{"name":"p","repositories":["a","a"]}]"#),
        ),
        ("empty name", header(r#","profiles":[{"name":""}]"#)),
        (
            "control character in key",
            header(r#","repositories":[{"key":"a\u0000b","name":"n","url":"https://a.example/"}]"#),
        ),
        ("bad note code", header(r#","notes":[{"code":"Bad Code"}]"#)),
        (
            "newer schema",
            r#"{"kind":"armasync.import","schema_version":2,"future":true}"#.to_owned(),
        ),
        (
            "wrong kind",
            r#"{"kind":"armasync.library","schema_version":1}"#.to_owned(),
        ),
        ("no version", r#"{"kind":"armasync.import"}"#.to_owned()),
        ("not json", "nonsense".to_owned()),
        (
            "source missing",
            r#"{"kind":"armasync.import","schema_version":1}"#.to_owned(),
        ),
        (
            "source text too long",
            format!(
                r#"{{"kind":"armasync.import","schema_version":1,"source":{{"tool":"{}","tool_version":"1","format":"f"}}}}"#,
                "x".repeat(500)
            ),
        ),
    ];
    for (name, text) in cases {
        let error = parse(&text).expect_err(name);
        assert_eq!(error.kind(), ErrorKind::Format, "{name}");
        let message = error.to_string();
        assert!(
            !message.contains("SECRET")
                && !message.contains("hunter2")
                && !message.contains("someone"),
            "{name}: {message}"
        );
    }
}

#[test]
fn deeply_nested_input_is_rejected_and_needs_credentials_flag_is_allowed() {
    let deep = format!("{}1{}", "[".repeat(100_000), "]".repeat(100_000));
    assert_eq!(parse(&deep).unwrap_err().kind(), ErrorKind::Format);
    let ok = header(
        r#","repositories":[{"key":"k","name":"n","url":"https://a.example/","needs_credentials":true}]"#,
    );
    parse(&ok).unwrap();
}
