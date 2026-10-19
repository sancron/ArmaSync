// SPDX-License-Identifier: AGPL-3.0-only
//! Black-box tests of `armasync repository import` (MIG-022) and of the scope notices that both import commands
//! print: preview vs apply, exit codes, JSON envelope, no secrets anywhere, no prompts. All data is synthetic.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const SECRETS: [&str; 3] = [
    "fixture-user",
    "fixture-password-not-a-secret",
    "fixture-server-password-not-a-secret",
];

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "armasync-cli-repo-import-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
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

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn autoconfig() -> PathBuf {
    root().join("fixtures/legacy-formats/generated/valid/server/small/.a3s/autoconfig")
}

fn path(p: &Path) -> &str {
    p.to_str().unwrap()
}

fn assert_no_secret(output: &Output) {
    for secret in SECRETS {
        assert!(!text(&output.stdout).contains(secret), "{secret} on stdout");
        assert!(!text(&output.stderr).contains(secret), "{secret} on stderr");
    }
}

#[test]
fn preview_is_the_default_and_writes_nothing() {
    let dir = scratch("preview");
    let config = dir.join("config");
    let out = armasync(&[
        "repository",
        "import",
        path(&autoconfig()),
        "--config",
        path(&config),
    ]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    let stdout = text(&out.stdout);
    assert!(stdout.contains("Preview only"), "{stdout}");
    assert!(
        stdout.contains("repository Example HTTP -> example-http [add]"),
        "{stdout}"
    );
    assert!(stdout.contains("needs credentials"), "{stdout}");
    assert!(stdout.contains("What this import covers:"), "{stdout}");
    assert!(
        stdout.contains("does not replace your own repository files"),
        "{stdout}"
    );
    assert!(!config.exists());
    assert_no_secret(&out);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn apply_json_envelope_is_secret_free_and_repeat_is_a_noop() {
    let dir = scratch("apply");
    let config = dir.join("config");
    let file = autoconfig();
    let args = [
        "--json",
        "--no-input",
        "repository",
        "import",
        path(&file),
        "--apply",
        "--config",
        path(&config),
    ];
    let out = armasync(&args);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    assert_no_secret(&out);
    let value: serde_json::Value = serde_json::from_str(&text(&out.stdout)).unwrap();
    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["command"], "repository-import");
    assert_eq!(value["status"], "ok");
    assert_eq!(value["result"]["mode"], "apply");
    assert_eq!(value["result"]["changed"], true);
    assert_eq!(value["result"]["counts"]["repositories_added"], 1);
    assert_eq!(
        value["result"]["source"]["format"],
        "legacy-server-autoconfig"
    );
    let codes: Vec<_> = value["result"]["scope_notices"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["code"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(
        codes,
        [
            "server-metadata-scope",
            "server-metadata-not-local-settings",
            "server-favorite-servers-skipped"
        ]
    );
    for notice in value["result"]["scope_notices"].as_array().unwrap() {
        assert!(notice["text"].as_str().unwrap().len() > 20);
        assert!(notice["text_de"].as_str().unwrap().len() > 20);
        assert!(["info", "limitation"].contains(&notice["severity"].as_str().unwrap()));
    }
    for entry in fs::read_dir(&config).unwrap() {
        let content = fs::read_to_string(entry.unwrap().path()).unwrap();
        for secret in SECRETS {
            assert!(!content.contains(secret), "{secret} stored");
        }
    }
    let library = fs::read_to_string(config.join("library.json")).unwrap();
    assert!(library.contains("https://repo.example.invalid"));

    let again = armasync(&args);
    assert_eq!(again.status.code(), Some(0));
    let value: serde_json::Value = serde_json::from_str(&text(&again.stdout)).unwrap();
    assert_eq!(value["result"]["changed"], false);
    assert_eq!(value["result"]["counts"]["repositories_unchanged"], 1);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn quiet_suppresses_the_notices() {
    let dir = scratch("quiet");
    let out = armasync(&[
        "--quiet",
        "repository",
        "import",
        path(&autoconfig()),
        "--config",
        path(&dir.join("config")),
    ]);
    assert_eq!(out.status.code(), Some(0));
    assert!(out.stdout.is_empty(), "{}", text(&out.stdout));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn wrong_and_hostile_files_fail_with_format_exit_code_and_no_writes() {
    let dir = scratch("hostile");
    let config = dir.join("config");
    let neutral = root().join("fixtures/neutral-import/synthetic-full.json");
    let out = armasync(&[
        "repository",
        "import",
        path(&neutral),
        "--config",
        path(&config),
    ]);
    assert_eq!(out.status.code(), Some(7));
    assert!(
        text(&out.stderr).contains("armasync import"),
        "{}",
        text(&out.stderr)
    );
    let invalid = root().join("fixtures/legacy-formats/generated/invalid");
    for entry in fs::read_dir(invalid).unwrap() {
        let file = entry.unwrap().path();
        if file.extension().is_some_and(|e| e == "zip") {
            continue;
        }
        let out = armasync(&[
            "repository",
            "import",
            path(&file),
            "--apply",
            "--config",
            path(&config),
        ]);
        assert_eq!(out.status.code(), Some(7), "{}", file.display());
        assert!(out.stdout.is_empty());
    }
    assert!(!config.exists());
    let missing = armasync(&[
        "repository",
        "import",
        path(&dir.join("nope")),
        "--config",
        path(&config),
    ]);
    assert_eq!(missing.status.code(), Some(2));
    let json = armasync(&["--json", "repository", "import", path(&neutral)]);
    assert_eq!(json.status.code(), Some(7));
    let value: serde_json::Value = serde_json::from_str(&text(&json.stdout)).unwrap();
    assert_eq!(value["status"], "error");
    assert_eq!(value["command"], "repository-import");
    assert_eq!(value["error"]["kind"], "format");
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn help_states_what_the_command_does_not_do() {
    let out = armasync(&["repository", "import", "--help"]);
    assert_eq!(out.status.code(), Some(0));
    let stdout = text(&out.stdout);
    for expected in ["--apply", "--on-conflict", "never passwords", "does not"] {
        assert!(stdout.contains(expected), "{expected} missing in {stdout}");
    }
}

#[test]
fn local_import_explains_its_limits_in_text_and_json() {
    let dir = scratch("local");
    let file = root().join("fixtures/neutral-import/from-legacy-local.json");
    let config = dir.join("config");
    let out = armasync(&["import", path(&file), "--config", path(&config)]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    let stdout = text(&out.stdout);
    assert!(stdout.contains("What this import covers:"), "{stdout}");
    assert!(
        stdout.contains("are encrypted and were not read"),
        "{stdout}"
    );
    assert!(stdout.contains("not in ArmaSync yet"), "{stdout}");
    assert!(!config.exists());

    let json = armasync(&["--json", "import", path(&file), "--config", path(&config)]);
    let value: serde_json::Value = serde_json::from_str(&text(&json.stdout)).unwrap();
    assert_eq!(value["command"], "import");
    let codes: Vec<_> = value["result"]["scope_notices"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["code"].as_str().unwrap())
        .collect();
    assert_eq!(
        codes,
        [
            "local-import-scope",
            "encrypted-repositories-not-read",
            "profile-repositories-missing"
        ]
    );
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn imports_without_a_known_source_make_no_claims() {
    let dir = scratch("synthetic");
    let file = dir.join("e.json");
    fs::write(
        &file,
        r#"{"kind":"armasync.import","schema_version":1,
        "source":{"tool":"test","tool_version":"1","format":"synthetic"},
        "repositories":[{"key":"A","name":"A","url":"https://a.example.invalid/r"}],"profiles":[]}"#,
    )
    .unwrap();
    let out = armasync(&[
        "--json",
        "import",
        path(&file),
        "--config",
        path(&dir.join("c")),
    ]);
    let value: serde_json::Value = serde_json::from_str(&text(&out.stdout)).unwrap();
    assert!(
        value["result"]["scope_notices"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    fs::remove_dir_all(dir).unwrap();
}
