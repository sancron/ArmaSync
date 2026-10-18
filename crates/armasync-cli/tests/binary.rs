// SPDX-License-Identifier: AGPL-3.0-only
//! Black-box tests of the `armasync` executable: exit codes, stdout/stderr separation, no waiting for input.
#![allow(clippy::expect_used)] // test code: a failing spawn must fail the test loudly

use std::process::{Command, Stdio};

fn armasync(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_armasync"))
        .args(args)
        .stdin(Stdio::null())
        .output()
        .expect("armasync binary must start")
}

#[test]
fn version_succeeds_with_empty_stderr() {
    let output = armasync(&["version"]);
    assert_eq!(output.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&output.stdout).contains("ArmaSync"));
    assert!(output.stderr.is_empty());
}

#[test]
fn json_goes_to_stdout_only() {
    let output = armasync(&["--json", "--no-input", "version"]);
    assert_eq!(output.status.code(), Some(0));
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.trim_start().starts_with('{') && text.contains("\"schema_version\":1"));
    assert!(output.stderr.is_empty());
}

#[test]
fn usage_error_exits_with_code_2_and_writes_nothing_to_stdout() {
    let output = armasync(&["--definitely-unknown-flag"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(!output.stderr.is_empty());
}
