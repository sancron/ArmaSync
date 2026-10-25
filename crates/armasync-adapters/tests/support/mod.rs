//! Test helpers for the adapter tests that need adapter types (ADR-0010, ADR-0011).
#![allow(clippy::unwrap_used, clippy::expect_used, dead_code, unused_imports)]

pub use armasync_testkit::webdav::*;

use armasync_adapters::http::HttpConfig;
use armasync_adapters::webdav::BasicCredentials;
use armasync_core::RelativePath;

/// The configuration that trusts only this server's test certificate.
pub fn trust(server: &Server) -> HttpConfig {
    HttpConfig {
        trusted_roots_der: Some(vec![server.cert_der()]),
        ..HttpConfig::default()
    }
}

pub fn path(text: &str) -> RelativePath {
    RelativePath::parse(text).unwrap()
}

pub fn credentials() -> BasicCredentials {
    BasicCredentials::new(USER, PASSWORD).unwrap()
}
