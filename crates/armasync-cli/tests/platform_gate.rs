// SPDX-License-Identifier: AGPL-3.0-only
//! The platform gate of `armasync sync run` (RB-01, `docs/migration/RELEASE_BLOCKERS.md`): in a normal build —
//! which includes every release build — there is **no** option that lets a user change a destination on a platform
//! whose synchronisation is not released. The override exists only in builds made with the cargo feature
//! `unreleased-platform-override` (validation builds).
//!
//! These tests compile and run on every platform; which ones are active depends on the platform and the feature.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

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

fn scratch(name: &str) -> PathBuf {
    let base = std::env::temp_dir().join(format!(
        "armasync-gate-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
    ));
    let mut dir = base.clone();
    let mut attempt = 0u32;
    while fs::create_dir(&dir).is_err() {
        attempt += 1;
        assert!(attempt < 1000);
        dir = PathBuf::from(format!("{}-{attempt}", base.display()));
    }
    dir
}

fn path(p: &Path) -> &str {
    p.to_str().unwrap()
}

/// A library with one repository whose address is a port nobody serves: a gate that works must refuse **before**
/// any network access, so nothing needs to listen.
fn library(root: &Path) -> PathBuf {
    let import = root.join("import.json");
    fs::write(
        &import,
        r#"{"kind":"armasync.import","schema_version":1,
        "source":{"tool":"test","tool_version":"1","format":"synthetic"},
        "repositories":[{"key":"Example","name":"Example","url":"http://127.0.0.1:9/repo"}],"profiles":[]}"#,
    )
    .unwrap();
    let config = root.join("config");
    let out = armasync(&[
        "import",
        path(&import),
        "--apply",
        "--config",
        path(&config),
    ]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    config
}

#[cfg(not(feature = "unreleased-platform-override"))]
#[test]
fn a_normal_build_does_not_offer_the_override_at_all() {
    // The option is unknown: a usage error, and the help does not mention it.
    let out = armasync(&[
        "sync",
        "run",
        "x",
        "--destination",
        ".",
        "--allow-unreleased-platform",
    ]);
    assert_eq!(out.status.code(), Some(2), "{}", text(&out.stderr));
    assert!(
        text(&out.stderr).contains("unexpected argument"),
        "{}",
        text(&out.stderr)
    );
    for args in [["sync", "run", "--help"], ["sync", "plan", "--help"]] {
        let help = armasync(&args);
        assert!(
            !text(&help.stdout).contains("unreleased"),
            "{}",
            text(&help.stdout)
        );
    }
    // neither is it reachable through the environment
    let out = Command::new(env!("CARGO_BIN_EXE_armasync"))
        .args(["sync", "run", "x", "--destination", "."])
        .env("ARMASYNC_ALLOW_UNRELEASED_PLATFORM", "1")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(!text(&out.stderr).contains("not released"));
}

#[cfg(feature = "unreleased-platform-override")]
#[test]
fn a_validation_build_documents_the_override_as_validation_only() {
    let help = armasync(&["sync", "run", "--help"]);
    let shown = text(&help.stdout);
    assert!(shown.contains("--allow-unreleased-platform"), "{shown}");
    assert!(shown.contains("unreleased-platform-override"), "{shown}");
}

/// On a platform that is not released (everything but Windows) a normal build refuses `sync run` before any
/// network access and changes nothing; `plan` and `--dry-run` stay available.
#[cfg(all(not(windows), not(feature = "unreleased-platform-override")))]
#[test]
fn a_normal_build_refuses_to_change_anything_on_an_unreleased_platform() {
    let root = scratch("refuse");
    let config = library(&root);
    let dest = root.join("dest");
    fs::create_dir_all(&dest).unwrap();
    let out = armasync(&[
        "sync",
        "run",
        "example",
        "--destination",
        path(&dest),
        "--config",
        path(&config),
    ]);
    assert_eq!(out.status.code(), Some(8), "{}", text(&out.stderr));
    assert!(
        text(&out.stderr).contains("noch nicht freigegeben"),
        "{}",
        text(&out.stderr)
    );
    assert_eq!(
        fs::read_dir(&dest).unwrap().count(),
        0,
        "nothing was created, not even the lock file"
    );
    let json = armasync(&[
        "--json",
        "sync",
        "run",
        "example",
        "--destination",
        path(&dest),
        "--config",
        path(&config),
    ]);
    let value: serde_json::Value = serde_json::from_str(&text(&json.stdout)).unwrap();
    assert_eq!(value["error"]["kind"], "unsupported");
    fs::remove_dir_all(root).unwrap();
}

/// Same guarantee on every platform for the plan modes: they never change anything and never need the override.
/// (The repository address is unreachable, so the plan fails with a network error — the point is that the gate
/// does not block them.)
#[test]
fn plan_and_dry_run_are_never_blocked_by_the_gate() {
    let root = scratch("plan");
    let config = library(&root);
    let dest = root.join("dest");
    fs::create_dir_all(&dest).unwrap();
    for sub in [vec!["sync", "plan"], vec!["sync", "run", "--dry-run"]] {
        let mut args = sub.clone();
        args.extend([
            "example",
            "--destination",
            path(&dest),
            "--config",
            path(&config),
        ]);
        let out = armasync(&args);
        assert_eq!(out.status.code(), Some(4), "{sub:?}: {}", text(&out.stderr));
        assert!(
            !text(&out.stderr).contains("noch nicht freigegeben"),
            "{sub:?}"
        );
    }
    assert_eq!(fs::read_dir(&dest).unwrap().count(), 0);
    fs::remove_dir_all(root).unwrap();
}
