// SPDX-License-Identifier: AGPL-3.0-only
//! The `upload` commands of the CLI (MIG-042, step 4; ADR-0014): exit codes, the JSON envelope, the refusals that
//! happen before any connection, cancellation, and that no password reaches the output. The network is never used
//! successfully here: the unreachable target is a closed loopback port. Synthetic data only.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::PathBuf;

use armasync_cli::{ExitStatus, run, run_with_cancel};
use armasync_core::CancelToken;
use serde_json::Value;

/// A loopback address with nothing listening: connections are refused at once.
const CLOSED: &str = "https://127.0.0.1:1/dav";

fn source(name: &str) -> PathBuf {
    let root =
        std::env::temp_dir().join(format!("armasync-cli-upload-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(root.join("a.txt"), b"alpha").unwrap();
    fs::write(root.join("docs/b.txt"), b"beta").unwrap();
    root
}

fn call(args: &[&str]) -> (ExitStatus, String, String) {
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let status = run(args.iter().copied(), &mut out, &mut err);
    (
        status,
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}

fn json_error(out: &str) -> Value {
    let value: Value = serde_json::from_str(out.trim()).expect("one JSON envelope");
    assert_eq!(value["status"], "error", "{value}");
    value
}

#[test]
fn publish_without_yes_is_refused_before_any_connection() {
    let root = source("no-yes");
    let (status, out, err) = call(&[
        "armasync",
        "--json",
        "upload",
        "publish",
        "--source",
        root.to_str().unwrap(),
        "--target",
        CLOSED,
        "--user",
        "u",
    ]);
    assert_eq!(status, ExitStatus::Config);
    assert_eq!(status.code(), 3);
    let value = json_error(&out);
    assert_eq!(value["command"], "upload-publish");
    assert_eq!(value["error"]["kind"], "config");
    assert!(
        value["error"]["message"]
            .as_str()
            .unwrap()
            .contains("--yes")
    );
    assert!(err.is_empty());
    fs::remove_dir_all(root).ok();
}

#[test]
fn an_address_with_credentials_is_refused_with_exit_code_3() {
    let root = source("userinfo");
    let (status, out, _) = call(&[
        "armasync",
        "--json",
        "upload",
        "plan",
        "--source",
        root.to_str().unwrap(),
        "--target",
        "https://u:p@127.0.0.1:1/dav",
    ]);
    assert_eq!(status.code(), 3);
    let value = json_error(&out);
    assert_eq!(value["command"], "upload-plan");
    assert!(!out.contains(":p@"), "{out}");
    fs::remove_dir_all(root).ok();
}

#[test]
fn a_user_without_the_password_variable_is_refused_with_exit_code_3() {
    let root = source("no-password");
    let variable = format!("ARMASYNC_TEST_NO_PASSWORD_{}", std::process::id());
    let (status, _, err) = call(&[
        "armasync",
        "upload",
        "plan",
        "--source",
        root.to_str().unwrap(),
        "--target",
        CLOSED,
        "--user",
        "u",
        "--password-env",
        &variable,
    ]);
    assert_eq!(status.code(), 3, "{err}");
    assert!(err.contains("password"), "{err}");
    fs::remove_dir_all(root).ok();
}

#[test]
fn an_unsafe_exclusion_is_a_usage_error_with_exit_code_2() {
    let root = source("bad-exclude");
    let (status, _, err) = call(&[
        "armasync",
        "upload",
        "plan",
        "--source",
        root.to_str().unwrap(),
        "--target",
        CLOSED,
        "--exclude",
        "../escape",
    ]);
    assert_eq!(status.code(), 2);
    assert!(err.contains("../escape"), "{err}");
    fs::remove_dir_all(root).ok();
}

#[test]
fn a_missing_source_folder_is_a_file_system_error_with_exit_code_5() {
    let (status, _, err) = call(&[
        "armasync",
        "upload",
        "plan",
        "--source",
        "X:/definitely/not/here-armasync",
        "--target",
        CLOSED,
    ]);
    assert_eq!(status.code(), 5, "{err}");
}

#[test]
fn an_unreachable_target_is_a_network_error_with_exit_code_4() {
    let root = source("unreachable");
    let (status, out, _) = call(&[
        "armasync",
        "--json",
        "upload",
        "plan",
        "--source",
        root.to_str().unwrap(),
        "--target",
        CLOSED,
    ]);
    assert_eq!(status.code(), 4, "{out}");
    let value = json_error(&out);
    assert_eq!(value["error"]["kind"], "network");
    fs::remove_dir_all(root).ok();
}

#[test]
fn a_cancelled_plan_exits_with_130_and_says_so() {
    let root = source("cancel");
    let cancel = CancelToken::new();
    cancel.cancel();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let status = run_with_cancel(
        [
            "armasync",
            "--json",
            "upload",
            "plan",
            "--source",
            root.to_str().unwrap(),
            "--target",
            CLOSED,
        ],
        &mut out,
        &mut err,
        &cancel,
    );
    assert_eq!(status.code(), 130);
    let value = json_error(&String::from_utf8(out).unwrap());
    assert_eq!(value["error"]["kind"], "cancelled");
    fs::remove_dir_all(root).ok();
}

#[test]
fn the_help_lists_plan_and_publish() {
    let (status, out, _) = call(&["armasync", "upload", "--help"]);
    assert_eq!(status, ExitStatus::Success);
    assert!(out.contains("plan") && out.contains("publish"), "{out}");
}
