// SPDX-License-Identifier: AGPL-3.0-only
//! Black-box tests of `armasync import`: preview vs apply, exit codes, JSON envelope, no secrets, no prompts.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "armasync-cli-import-{name}-{}-{}",
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

fn export(dir: &Path, name: &str, host: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(
        &path,
        format!(
            r#"{{"kind":"armasync.import","schema_version":1,
            "source":{{"tool":"test","tool_version":"1","format":"synthetic"}},
            "repositories":[{{"key":"Main","name":"Main","url":"https://{host}/repo","needs_credentials":true}}],
            "profiles":[{{"name":"Alpha","repositories":["Main","Missing"]}}]}}"#
        ),
    )
    .unwrap();
    path
}

#[test]
fn preview_is_the_default_and_writes_nothing() {
    let dir = scratch("preview");
    let file = export(&dir, "e.json", "a.example.invalid");
    let config = dir.join("config");
    let out = armasync(&[
        "import",
        file.to_str().unwrap(),
        "--config",
        config.to_str().unwrap(),
    ]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    let stdout = text(&out.stdout);
    assert!(stdout.contains("Preview only"), "{stdout}");
    assert!(stdout.contains("repository Main -> main [add]"), "{stdout}");
    assert!(stdout.contains("warning:"), "{stdout}");
    assert!(!config.exists());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn apply_json_envelope_then_repeat_is_a_noop() {
    let dir = scratch("apply");
    let file = export(&dir, "e.json", "a.example.invalid");
    let config = dir.join("config");
    let args = [
        "--json",
        "--no-input",
        "import",
        file.to_str().unwrap(),
        "--apply",
        "--config",
        config.to_str().unwrap(),
    ];
    let out = armasync(&args);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    let value: serde_json::Value = serde_json::from_str(&text(&out.stdout)).unwrap();
    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["status"], "ok");
    assert_eq!(value["result"]["mode"], "apply");
    assert_eq!(value["result"]["changed"], true);
    assert_eq!(value["result"]["counts"]["repositories_added"], 1);
    assert_eq!(
        value["result"]["profiles"][0]["unresolved_repository_keys"][0],
        "Missing"
    );
    assert!(config.join("library.json").is_file());
    assert!(config.join("import-journal.json").is_file());
    let library_before = fs::read(config.join("library.json")).unwrap();

    let again = armasync(&args);
    assert_eq!(again.status.code(), Some(0));
    let value: serde_json::Value = serde_json::from_str(&text(&again.stdout)).unwrap();
    assert_eq!(value["result"]["changed"], false);
    assert_eq!(
        fs::read(config.join("library.json")).unwrap(),
        library_before
    );

    let human = armasync(&[
        "import",
        file.to_str().unwrap(),
        "--apply",
        "--config",
        config.to_str().unwrap(),
    ]);
    assert!(text(&human.stdout).contains("Nothing to import"));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn secret_bearing_import_files_are_refused_without_echoing_the_secret() {
    let dir = scratch("secret");
    let file = dir.join("bad.json");
    fs::write(
        &file,
        r#"{"kind":"armasync.import","schema_version":1,"source":{"tool":"t","tool_version":"1","format":"f"},
            "repositories":[{"key":"k","name":"n","url":"https://a.example/","password":"hunter2-SECRET"}]}"#,
    )
    .unwrap();
    let config = dir.join("config");
    let out = armasync(&[
        "--json",
        "import",
        file.to_str().unwrap(),
        "--apply",
        "--config",
        config.to_str().unwrap(),
    ]);
    assert_eq!(out.status.code(), Some(7));
    let (stdout, stderr) = (text(&out.stdout), text(&out.stderr));
    assert!(!stdout.contains("SECRET") && !stderr.contains("SECRET"));
    let value: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(value["status"], "error");
    assert_eq!(value["error"]["kind"], "format");
    assert!(!config.exists());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn usage_and_conflict_errors_use_the_documented_exit_codes() {
    let dir = scratch("codes");
    let file = export(&dir, "e.json", "a.example.invalid");
    let config = dir.join("config");
    let config_arg = config.to_str().unwrap();
    // missing file
    let missing = armasync(&[
        "import",
        dir.join("nope.json").to_str().unwrap(),
        "--config",
        config_arg,
    ]);
    assert_eq!(missing.status.code(), Some(2));
    // unknown policy value
    let bad = armasync(&[
        "import",
        file.to_str().unwrap(),
        "--on-conflict",
        "skip",
        "--config",
        config_arg,
    ]);
    assert_eq!(bad.status.code(), Some(2));
    assert!(bad.stdout.is_empty());
    // conflict with --on-conflict fail
    assert_eq!(
        armasync(&[
            "import",
            file.to_str().unwrap(),
            "--apply",
            "--config",
            config_arg
        ])
        .status
        .code(),
        Some(0)
    );
    let other = export(&dir, "other.json", "b.example.invalid");
    let before = fs::read(config.join("library.json")).unwrap();
    let conflict = armasync(&[
        "import",
        other.to_str().unwrap(),
        "--apply",
        "--on-conflict",
        "fail",
        "--config",
        config_arg,
    ]);
    assert_eq!(conflict.status.code(), Some(3));
    assert_eq!(fs::read(config.join("library.json")).unwrap(), before);
    // keep-both adds next to the existing entries
    let both = armasync(&[
        "--json",
        "import",
        other.to_str().unwrap(),
        "--apply",
        "--config",
        config_arg,
    ]);
    assert_eq!(both.status.code(), Some(0));
    let value: serde_json::Value = serde_json::from_str(&text(&both.stdout)).unwrap();
    assert_eq!(value["result"]["repositories"][0]["id"], "main-2");
    fs::remove_dir_all(dir).unwrap();
}
