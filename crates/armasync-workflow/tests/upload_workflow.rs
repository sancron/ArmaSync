// SPDX-License-Identifier: AGPL-3.0-only
//! The shared upload wiring (MIG-042, step 4; ADR-0014) against the loopback WebDAV server of the test kit: preview,
//! publication, cancellation and the refusals that must happen before any connection. Synthetic data and credentials.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::{Path, PathBuf};

use armasync_adapters::http::HttpConfig;
use armasync_core::{CancelToken, ErrorKind};
use armasync_testkit::webdav::{EXPECTED_AUTH, PASSWORD, Server, USER, start};
use armasync_workflow::upload::{UploadRequest, check_target, preview, publish};

fn tree(name: &str) -> PathBuf {
    let root =
        std::env::temp_dir().join(format!("armasync-wf-upload-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    for (relative, bytes) in [
        ("a.txt", b"alpha".as_slice()),
        ("docs/open.txt", b"open"),
        ("docs/secret.txt", b"private"),
        (".armasync-lock", b""),
        ("tmp/one.txt", b"temp"),
    ] {
        let target = root.join(relative);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(target, bytes).unwrap();
    }
    root
}

/// Trust configuration for the test server's self-signed certificate; the production default stays the platform store.
fn trusting(server: &Server) -> HttpConfig {
    HttpConfig {
        trusted_roots_der: Some(vec![server.cert_der()]),
        ..HttpConfig::default()
    }
}

fn request(root: &Path, target: &str, server: Option<&Server>) -> UploadRequest {
    UploadRequest {
        source: root.to_path_buf(),
        target: target.to_owned(),
        user: Some(USER.to_owned()),
        password: Some(PASSWORD.to_owned()),
        exclusions: vec!["docs/secret.txt".to_owned(), "tmp".to_owned()],
        http: server.map_or_else(HttpConfig::default, trusting),
    }
}

#[test]
fn the_preview_shows_included_and_excluded_files_and_changes_nothing() {
    let root = tree("preview");
    let server = start(true, true, "");
    let plan = preview(
        &request(&root, &server.https(), Some(&server)),
        &CancelToken::new(),
    )
    .unwrap();
    assert_eq!(plan.files, 2, "{plan:?}");
    let excluded: Vec<(&str, &str)> = plan
        .exclusions
        .iter()
        .map(|e| (e.path.as_str(), e.reason.code()))
        .collect();
    assert_eq!(
        excluded,
        [
            (".armasync-lock", "internal-data"),
            ("docs/secret.txt", "user-rule"),
            ("tmp/one.txt", "user-rule"),
        ]
    );
    assert!(
        server
            .seen()
            .iter()
            .all(|s| s.method != "PUT" && s.method != "MKCOL"),
        "a preview writes nothing"
    );
    fs::remove_dir_all(root).ok();
}

#[test]
fn a_publication_with_credentials_publishes_only_the_included_files() {
    let root = tree("publish");
    let server = start(true, true, "");
    let report = publish(
        &request(&root, &server.https(), Some(&server)),
        &CancelToken::new(),
    )
    .unwrap();
    assert!(report.pointer_changed);
    assert_eq!(report.files_uploaded, 2);
    assert!(server.file("current.json").is_some());
    let names = server.file_names();
    assert!(
        names
            .iter()
            .all(|n| !n.contains(".armasync") && !n.contains("secret") && !n.contains("tmp/")),
        "{names:?}"
    );
    // Every request that touches the target carries the credentials, because the target is HTTPS.
    assert!(
        server
            .seen()
            .iter()
            .all(|s| s.authorization.as_deref() == Some(EXPECTED_AUTH))
    );
    fs::remove_dir_all(root).ok();
}

#[test]
fn a_cancelled_publication_leaves_no_pointer() {
    let root = tree("cancel");
    let server = start(true, true, "");
    let cancel = CancelToken::new();
    cancel.cancel();
    let error = publish(&request(&root, &server.https(), Some(&server)), &cancel).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Cancelled, "{error}");
    assert!(server.file("current.json").is_none());
    assert!(server.seen().iter().all(|s| s.method != "PUT"));
    fs::remove_dir_all(root).ok();
}

#[test]
fn an_address_with_credentials_is_refused_before_any_connection() {
    let root = tree("userinfo");
    let server = start(true, true, "");
    let target = format!("https://{USER}:{PASSWORD}@{}/dav", server.addr);
    let error = preview(&request(&root, &target, Some(&server)), &CancelToken::new()).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Config, "{error}");
    assert!(!error.message().contains(PASSWORD), "{error}");
    assert!(server.seen().is_empty());
    fs::remove_dir_all(root).ok();
}

#[test]
fn query_and_fragment_are_refused_in_the_address() {
    assert_eq!(
        check_target("https://h.example/dav?x=1")
            .unwrap_err()
            .kind(),
        ErrorKind::Config
    );
    assert_eq!(
        check_target("https://h.example/dav#frag")
            .unwrap_err()
            .kind(),
        ErrorKind::Config
    );
    assert!(check_target("https://h.example/dav/folder").is_ok());
}

#[test]
fn credentials_over_plain_http_are_refused_before_any_connection() {
    let root = tree("plain-http");
    let server = start(false, false, "");
    let target = format!("http://{}/dav", server.addr);
    let error = publish(&request(&root, &target, None), &CancelToken::new()).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Config, "{error}");
    assert!(server.seen().is_empty());
    fs::remove_dir_all(root).ok();
}

#[test]
fn a_user_without_a_password_and_a_password_without_a_user_are_refused() {
    let root = tree("half-credentials");
    let server = start(true, true, "");
    let mut only_user = request(&root, &server.https(), Some(&server));
    only_user.password = None;
    assert_eq!(
        preview(&only_user, &CancelToken::new()).unwrap_err().kind(),
        ErrorKind::Config
    );
    let mut only_password = request(&root, &server.https(), Some(&server));
    only_password.user = None;
    assert_eq!(
        preview(&only_password, &CancelToken::new())
            .unwrap_err()
            .kind(),
        ErrorKind::Config
    );
    assert!(server.seen().is_empty());
    fs::remove_dir_all(root).ok();
}

#[test]
fn an_unsafe_exclusion_is_a_usage_error_naming_it() {
    let root = tree("bad-exclusion");
    let server = start(true, true, "");
    let mut bad = request(&root, &server.https(), Some(&server));
    bad.exclusions = vec!["../escape".to_owned()];
    let error = preview(&bad, &CancelToken::new()).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Usage);
    assert!(error.message().contains("../escape"), "{error}");
    fs::remove_dir_all(root).ok();
}

#[test]
fn the_request_debug_output_hides_the_password() {
    let root = tree("debug");
    let debug = format!("{:?}", request(&root, "https://h.example/dav", None));
    assert!(!debug.contains(PASSWORD), "{debug}");
    assert!(debug.contains("<redacted>"), "{debug}");
    fs::remove_dir_all(root).ok();
}
