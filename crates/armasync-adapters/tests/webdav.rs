// SPDX-License-Identifier: AGPL-3.0-only
//! Tests of the WebDAV client (OPTIONS, PROPFIND, MKCOL, PUT, DELETE) against local loopback servers: plain HTTP and
//! TLS with a freshly generated, self-signed test certificate. Synthetic credentials only; no real server and no real
//! data.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;
use support::trust;

use std::fs::File;
use std::io::Cursor;
use std::time::{Duration, Instant};

use armasync_adapters::http::HttpConfig;
use armasync_adapters::webdav::{BasicCredentials, WebDavClient};
use armasync_core::{CancelToken, ErrorKind, OpenedSource, RemoteTarget};
use support::{EXPECTED_AUTH, USER, credentials, path, payload, scratch_file, start};

// ---- schemes and credentials ----------------------------------------------------------------------------------

#[test]
fn credentials_over_plain_http_are_refused_before_any_connection() {
    let server = start(false, false, "");
    let error =
        WebDavClient::new(&server.http(), HttpConfig::default(), Some(credentials())).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Config);
    assert!(server.seen().is_empty());
}

#[test]
fn other_schemes_are_unsupported() {
    let error = WebDavClient::new("ftp://127.0.0.1/dav", HttpConfig::default(), None).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Unsupported);
}

#[test]
fn credentials_must_be_complete_and_are_never_printed() {
    assert!(BasicCredentials::new("", "x").is_err());
    assert!(BasicCredentials::new("u", "").is_err());
    assert!(BasicCredentials::new("a:b", "x").is_err());
    assert!(BasicCredentials::new("a\nb", "x").is_err());
    let debug = format!(
        "{:?}",
        BasicCredentials::new("user", "topsecret-value").unwrap()
    );
    assert!(!debug.contains("topsecret-value"), "{debug}");
}

#[test]
fn the_client_debug_output_never_shows_the_password() {
    let client = WebDavClient::new(
        "https://127.0.0.1:9/dav",
        HttpConfig::default(),
        Some(BasicCredentials::new("user", "topsecret-value").unwrap()),
    )
    .unwrap();
    let debug = format!("{client:?}");
    assert!(!debug.contains("topsecret-value"), "{debug}");
}

#[test]
fn an_untrusted_certificate_is_rejected_with_the_default_trust_store() {
    let server = start(true, true, "");
    let client =
        WebDavClient::new(&server.https(), HttpConfig::default(), Some(credentials())).unwrap();
    let error = client.capabilities().unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Network);
    assert!(
        error.message().contains("secure connection failed"),
        "{error}"
    );
}

// ---- capabilities and anonymous access ------------------------------------------------------------------------

#[test]
fn options_declare_class_1_anonymously_over_plain_http() {
    let server = start(false, false, "");
    let client = WebDavClient::new(&server.http(), HttpConfig::default(), None).unwrap();
    let capabilities = client.capabilities().unwrap();
    assert!(!capabilities.locking);
    assert!(capabilities.allow.iter().any(|m| m == "PUT"));
    assert!(server.seen().iter().all(|s| s.authorization.is_none()));
}

#[test]
fn anonymous_folder_creation_and_upload_over_plain_http_send_no_authorization() {
    let server = start(false, false, "");
    let client = WebDavClient::new(&server.http(), HttpConfig::default(), None).unwrap();
    client.make_collection(&path("docs")).unwrap();
    let mut file = File::open(scratch_file("anon.txt", b"hello")).unwrap();
    client.put(&path("docs/hello.txt"), &mut file).unwrap();
    assert_eq!(server.file("docs/hello.txt").unwrap(), b"hello");
    let seen = server.seen();
    assert!(seen.iter().any(|s| s.method == "MKCOL"), "{seen:?}");
    assert!(seen.iter().all(|s| s.authorization.is_none()));
}

// ---- authenticated round trip over TLS ------------------------------------------------------------------------

#[test]
fn basic_auth_round_trip_over_https_carries_the_header_on_every_request() {
    let server = start(true, true, "");
    let client = WebDavClient::new(&server.https(), trust(&server), Some(credentials())).unwrap();
    client.capabilities().unwrap();
    client.make_collection(&path("docs")).unwrap();
    assert!(client.exists(&path("docs")).unwrap());
    let content = payload();
    let mut file = File::open(scratch_file("payload.bin", &content)).unwrap();
    client.put(&path("docs/payload.bin"), &mut file).unwrap();
    assert_eq!(server.file("docs/payload.bin").unwrap(), content);
    assert!(client.exists(&path("docs/payload.bin")).unwrap());
    assert!(client.delete(&path("docs/payload.bin")).unwrap());
    assert!(!client.delete(&path("docs/payload.bin")).unwrap());
    assert!(!client.exists(&path("docs/payload.bin")).unwrap());
    let seen = server.seen();
    let methods: Vec<&str> = seen.iter().map(|s| s.method.as_str()).collect();
    assert_eq!(
        methods,
        [
            "OPTIONS", "MKCOL", "PROPFIND", "PUT", "PROPFIND", "DELETE", "DELETE", "PROPFIND"
        ],
        "{seen:?}"
    );
    assert!(
        seen.iter()
            .all(|s| s.authorization.as_deref() == Some(EXPECTED_AUTH)),
        "{seen:?}"
    );
}

#[test]
fn a_wrong_password_is_refused_without_echoing_it() {
    let server = start(true, true, "");
    let wrong = BasicCredentials::new(USER, "wrong-secret-value").unwrap();
    let client = WebDavClient::new(&server.https(), trust(&server), Some(wrong)).unwrap();
    let error = client.capabilities().unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Network);
    assert!(
        error.message().contains("check the user name and password"),
        "{error}"
    );
    assert!(!error.message().contains("wrong-secret-value"));
    assert!(!server.seen().is_empty());
}

// ---- folders ------------------------------------------------------------------------------------------------

#[test]
fn make_collection_creates_a_folder_and_accepts_an_existing_one() {
    let server = start(true, true, "");
    let client = WebDavClient::new(&server.https(), trust(&server), Some(credentials())).unwrap();
    client.make_collection(&path("docs")).unwrap();
    client.make_collection(&path("docs")).unwrap();
    assert!(client.exists(&path("docs")).unwrap());
    assert!(!client.exists(&path("other")).unwrap());
}

#[test]
fn make_collection_without_a_parent_folder_is_a_file_system_error() {
    let server = start(true, true, "");
    let client = WebDavClient::new(&server.https(), trust(&server), Some(credentials())).unwrap();
    let error = client.make_collection(&path("missing/sub")).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::FileSystem, "{error}");
}

// ---- redirects ------------------------------------------------------------------------------------------------

#[test]
fn write_requests_do_not_follow_redirects() {
    let server = start(true, true, "");
    server.folder("docs");
    let client = WebDavClient::new(&server.https(), trust(&server), Some(credentials())).unwrap();
    let mut file = File::open(scratch_file("redirected.bin", b"data")).unwrap();
    let error = client
        .put(&path("redirect/same/docs/x.bin"), &mut file)
        .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Network);
    assert!(error.message().contains("not followed"), "{error}");
    assert!(server.file("docs/x.bin").is_none());
    let seen = server.seen();
    assert!(
        seen.iter().all(|s| s.target != "/dav/docs/x.bin"),
        "{seen:?}"
    );
}

#[test]
fn a_redirected_folder_creation_is_not_followed() {
    let server = start(true, true, "");
    let client = WebDavClient::new(&server.https(), trust(&server), Some(credentials())).unwrap();
    let error = client
        .make_collection(&path("redirect/same/newdir"))
        .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Network);
    assert!(error.message().contains("not followed"), "{error}");
    assert!(server.seen().iter().all(|s| s.target != "/dav/newdir"));
}

#[test]
fn same_origin_redirect_keeps_the_credentials_for_read_requests() {
    let server = start(true, true, "");
    let base = format!("https://{}/dav/redirect/same", server.addr);
    let client = WebDavClient::new(&base, trust(&server), Some(credentials())).unwrap();
    client.capabilities().unwrap();
    let seen = server.seen();
    let followed = seen
        .iter()
        .find(|s| s.target == "/dav/" && s.method == "OPTIONS")
        .expect("the redirect target was requested");
    assert_eq!(followed.authorization.as_deref(), Some(EXPECTED_AUTH));
}

#[test]
fn a_cross_origin_redirect_never_receives_credentials() {
    let other = start(true, false, "");
    let target = format!("https://{}/dav/docs", other.addr);
    let server = start(true, true, &target);
    let base = format!("https://{}/dav/redirect/cross", server.addr);
    let client = WebDavClient::new(&base, trust(&server), Some(credentials())).unwrap();
    let error = client.exists(&path("docs")).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Network);
    assert!(error.message().contains("refused"), "{error}");
    assert!(other.seen().is_empty(), "{:?}", other.seen());
    assert_eq!(server.seen().len(), 1);
}

#[test]
fn a_redirect_from_https_to_http_is_refused_and_the_plain_target_is_never_contacted() {
    let plain = start(false, false, "");
    let target = format!("http://{}/dav/docs", plain.addr);
    let server = start(true, true, &target);
    let base = format!("https://{}/dav/redirect/cross", server.addr);
    let client = WebDavClient::new(&base, trust(&server), Some(credentials())).unwrap();
    let error = client.capabilities().unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Network);
    assert!(plain.seen().is_empty(), "{:?}", plain.seen());
}

// ---- failures, timeouts and cancellation ----------------------------------------------------------------------

#[test]
fn full_disk_locked_and_missing_parent_map_to_file_system_errors() {
    let server = start(true, true, "");
    server.folder("docs");
    let client = WebDavClient::new(&server.https(), trust(&server), Some(credentials())).unwrap();
    let mut file = File::open(scratch_file("failure.bin", b"x")).unwrap();
    for target in ["docs/full.bin", "docs/locked.bin", "missing/x.bin"] {
        let error = client.put(&path(target), &mut file).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::FileSystem, "{target}: {error}");
    }
}

fn short_limits(server: &support::Server) -> HttpConfig {
    HttpConfig {
        response_timeout: Duration::from_secs(1),
        min_body_timeout: Duration::from_secs(1),
        ..trust(server)
    }
}

#[test]
fn a_server_that_does_not_answer_times_out_within_the_limit() {
    let server = start(true, true, "");
    let client =
        WebDavClient::new(&server.https(), short_limits(&server), Some(credentials())).unwrap();
    let started = Instant::now();
    let error = client.exists(&path("hang")).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Network);
    assert!(
        error.message().contains("did not respond in time"),
        "{error}"
    );
    assert!(
        started.elapsed() < Duration::from_secs(3),
        "{:?}",
        started.elapsed()
    );
}

#[test]
fn an_upload_to_a_silent_server_times_out_within_the_limit() {
    let server = start(true, true, "");
    let client =
        WebDavClient::new(&server.https(), short_limits(&server), Some(credentials())).unwrap();
    let source = OpenedSource {
        size: 3,
        content: Box::new(Cursor::new(b"abc".to_vec())),
    };
    let started = Instant::now();
    let error = client
        .upload(&path("hang/x.txt"), source, &CancelToken::new())
        .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Network, "{error}");
    assert!(
        started.elapsed() < Duration::from_secs(3),
        "{:?}",
        started.elapsed()
    );
}

#[test]
fn an_upload_started_after_cancellation_is_reported_as_cancelled_and_stores_nothing() {
    let server = start(true, true, "");
    server.folder("docs");
    let client = WebDavClient::new(&server.https(), trust(&server), Some(credentials())).unwrap();
    let cancel = CancelToken::new();
    cancel.cancel();
    let source = OpenedSource {
        size: 3,
        content: Box::new(Cursor::new(b"abc".to_vec())),
    };
    let error = client
        .upload(&path("docs/cancelled.txt"), source, &cancel)
        .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Cancelled, "{error}");
    assert!(server.file("docs/cancelled.txt").is_none());
}
