// SPDX-License-Identifier: AGPL-3.0-only
//! Resume over HTTP (MIG-032R, ADR-0009) against the loopback test server. The server serves synthetic files with
//! a configurable `ETag`, range behaviour and a cut connection. No real server and no real data are used.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;
use std::io::Read;

use armasync_adapters::http::{HttpConfig, HttpRepository};
use armasync_core::{ErrorKind, RelativePath, RepositorySource};
use armasync_testkit::{Published, RangeBehavior, Server, serve_published};

const NAME: &str = "a.pbo";
const STRONG: &str = "\"v1\"";

fn content() -> Vec<u8> {
    (0..300_000).map(|i| (i % 253) as u8).collect()
}

fn publish(etag: Option<&str>, ranges: RangeBehavior, cut_after: Option<usize>) -> Server {
    let mut files = BTreeMap::new();
    files.insert(NAME.to_owned(), content());
    serve_published(Published {
        files,
        etag: etag.map(str::to_owned),
        ranges,
        cut_after,
        ..Published::default()
    })
}

fn repository(server: &Server) -> HttpRepository {
    HttpRepository::new(
        &format!("http://{}/repo", server.addr),
        HttpConfig::default(),
    )
    .unwrap()
}

fn name() -> RelativePath {
    RelativePath::parse(NAME).unwrap()
}

fn size() -> u64 {
    content().len() as u64
}

fn read_all(body: &mut dyn Read) -> Vec<u8> {
    let mut data = Vec::new();
    body.read_to_end(&mut data).unwrap();
    data
}

/// The headers the server saw in its last request for the file (lower-case names).
fn last_file_headers(server: &Server) -> Vec<(String, String)> {
    server
        .requests
        .lock()
        .unwrap()
        .iter()
        .rev()
        .find(|(target, _)| target.ends_with(NAME))
        .map(|(_, headers)| headers.clone())
        .unwrap_or_default()
}

fn header<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.as_str())
}

#[test]
fn a_whole_file_answer_carries_its_strong_validator() {
    let server = publish(Some(STRONG), RangeBehavior::Honor, None);
    let repo = repository(&server);
    let mut opened = repo.open(&name(), size(), 0, None).unwrap();
    assert_eq!(opened.start, 0);
    assert_eq!(opened.validator.as_deref(), Some(STRONG));
    assert_eq!(read_all(&mut *opened.body), content());
    assert!(
        header(&last_file_headers(&server), "range").is_none(),
        "no range without an offset"
    );
}

#[test]
fn a_matching_if_range_continues_with_206_and_only_the_rest_is_sent() {
    let server = publish(Some(STRONG), RangeBehavior::Honor, None);
    let repo = repository(&server);
    let offset = 120_000;
    let mut opened = repo.open(&name(), size(), offset, Some(STRONG)).unwrap();
    assert_eq!(opened.start, offset, "the answer continues at the offset");
    assert_eq!(opened.validator.as_deref(), Some(STRONG));
    assert_eq!(read_all(&mut *opened.body), content()[offset as usize..]);

    let headers = last_file_headers(&server);
    assert_eq!(header(&headers, "range"), Some("bytes=120000-"));
    assert_eq!(header(&headers, "if-range"), Some(STRONG));
}

#[test]
fn a_changed_validator_restarts_with_the_whole_file_and_the_new_validator() {
    let server = publish(Some("\"v2\""), RangeBehavior::Honor, None);
    let repo = repository(&server);
    let mut opened = repo.open(&name(), size(), 120_000, Some(STRONG)).unwrap();
    assert_eq!(
        opened.start, 0,
        "the old validator does not match: no resume"
    );
    assert_eq!(opened.validator.as_deref(), Some("\"v2\""));
    assert_eq!(read_all(&mut *opened.body), content());
}

#[test]
fn a_server_that_ignores_ranges_answers_with_the_whole_file() {
    let server = publish(Some(STRONG), RangeBehavior::Ignore, None);
    let repo = repository(&server);
    let mut opened = repo.open(&name(), size(), 120_000, Some(STRONG)).unwrap();
    assert_eq!(opened.start, 0);
    assert_eq!(read_all(&mut *opened.body), content());
}

#[test]
fn a_wrong_content_range_is_refused_before_any_byte_is_used() {
    let server = publish(Some(STRONG), RangeBehavior::WrongContentRange, None);
    let repo = repository(&server);
    let error = repo
        .open(&name(), size(), 120_000, Some(STRONG))
        .err()
        .unwrap();
    assert_eq!(error.kind(), ErrorKind::Integrity);
}

#[test]
fn a_weak_validator_is_never_sent_as_if_range_and_never_stored() {
    let weak = "W/\"v1\"";
    let server = publish(Some(weak), RangeBehavior::Honor, None);
    let repo = repository(&server);
    let mut opened = repo.open(&name(), size(), 0, None).unwrap();
    assert!(opened.validator.is_none(), "a weak ETag is not a validator");
    assert_eq!(read_all(&mut *opened.body), content());

    let opened = repo.open(&name(), size(), 120_000, Some(weak)).unwrap();
    assert_eq!(opened.start, 0);
    assert!(
        header(&last_file_headers(&server), "range").is_none(),
        "a weak validator must not start a range request"
    );
}

#[test]
fn a_resume_beyond_the_end_is_refused_as_integrity() {
    let server = publish(Some(STRONG), RangeBehavior::Honor, None);
    let repo = repository(&server);
    // The announced size is larger than the file: the server cannot answer the range (416).
    let error = repo
        .open(&name(), size() + 500, size() + 100, Some(STRONG))
        .err()
        .unwrap();
    assert_eq!(error.kind(), ErrorKind::Integrity);
}

#[test]
fn a_connection_cut_during_a_resumed_body_is_a_network_error() {
    let server = publish(Some(STRONG), RangeBehavior::Honor, Some(5_000));
    let repo = repository(&server);
    let mut opened = repo.open(&name(), size(), 120_000, Some(STRONG)).unwrap();
    assert_eq!(opened.start, 120_000);
    let mut received = Vec::new();
    let outcome = opened.body.read_to_end(&mut received);
    assert!(
        outcome.is_err(),
        "a body that ends before its length is an error"
    );
    assert!(received.len() <= 5_000);
}

#[test]
fn a_server_without_an_etag_answers_with_the_whole_file_and_no_validator() {
    // The request is sent with the stored validator, but the server has none to compare: whole file, no validator.
    let server = publish(None, RangeBehavior::Honor, None);
    let repo = repository(&server);
    let opened = repo.open(&name(), size(), 120_000, Some(STRONG)).unwrap();
    assert_eq!(opened.start, 0);
    assert!(opened.validator.is_none());
    assert_eq!(
        header(&last_file_headers(&server), "if-range"),
        Some(STRONG)
    );
}
