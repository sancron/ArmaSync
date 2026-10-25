// SPDX-License-Identifier: AGPL-3.0-only
//! Tests of the manual upload server (MIG-042, A11Y): loopback only, the WebDAV requests of an upload, path and size
//! limits, no test-only routes, and that stopping releases the port. Raw HTTP over loopback; synthetic data only.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::thread;
use std::time::{Duration, Instant};

use armasync_testkit::webdav::{ManualLimits, Server, start_manual};

/// Sends one request with a `Content-Length` header and returns the status code and the body.
fn request(addr: SocketAddr, method: &str, target: &str, body: &[u8]) -> (u16, Vec<u8>) {
    let head = format!(
        "{method} {target} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: {}\r\n\r\n",
        body.len()
    );
    send(addr, head.as_bytes(), body)
}

/// Sends raw bytes and reads the answer. A refused request may end the connection early; the answer is still read.
fn send(addr: SocketAddr, head: &[u8], body: &[u8]) -> (u16, Vec<u8>) {
    let mut stream = TcpStream::connect(addr).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    let _ = stream.write_all(head);
    let _ = stream.write_all(body);
    let mut response = Vec::new();
    let _ = stream.read_to_end(&mut response);
    let text = String::from_utf8_lossy(&response).into_owned();
    let status = text
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or(0);
    let body = response
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .map_or(Vec::new(), |pos| response[pos + 4..].to_vec());
    (status, body)
}

fn small_limits() -> ManualLimits {
    ManualLimits {
        max_file_bytes: 16,
        max_total_bytes: 10,
        max_files: 2,
    }
}

#[test]
fn the_server_binds_to_loopback_on_a_dynamic_port() {
    let server = start_manual(ManualLimits::default());
    assert_eq!(server.addr.ip(), Ipv4Addr::LOCALHOST);
    assert_ne!(server.addr.port(), 0);
}

#[test]
fn the_requests_of_an_upload_round_trip() {
    let server = start_manual(ManualLimits::default());
    let addr = server.addr;

    let (status, _) = request(addr, "OPTIONS", "/dav", b"");
    assert_eq!(status, 200, "OPTIONS");
    assert_eq!(
        request(addr, "PROPFIND", "/dav/", b"").0,
        207,
        "root exists"
    );
    assert_eq!(request(addr, "MKCOL", "/dav/docs", b"").0, 201, "MKCOL");
    assert_eq!(
        request(addr, "PUT", "/dav/docs/b.txt", b"beta").0,
        201,
        "PUT"
    );
    assert_eq!(request(addr, "PROPFIND", "/dav/docs/b.txt", b"").0, 207);
    assert_eq!(
        request(addr, "GET", "/dav/docs/b.txt", b""),
        (200, b"beta".to_vec())
    );
    assert_eq!(request(addr, "DELETE", "/dav/docs/b.txt", b"").0, 204);
    assert_eq!(request(addr, "GET", "/dav/docs/b.txt", b"").0, 404);
    assert_eq!(request(addr, "PROPFIND", "/dav/docs/b.txt", b"").0, 404);
}

#[test]
fn percent_encoded_names_are_decoded_and_stored_by_their_real_name() {
    let server = start_manual(ManualLimits::default());
    assert_eq!(
        request(server.addr, "PUT", "/dav/Meine%20Datei.txt", b"x").0,
        201
    );
    assert_eq!(server.file("Meine Datei.txt").unwrap(), b"x");
}

#[test]
fn traversal_separators_and_control_characters_are_refused_and_store_nothing() {
    let server = start_manual(ManualLimits::default());
    let refused = [
        "/dav/../escape.txt",
        "/dav/a/../escape.txt",
        "/dav/a/%2e%2e/escape.txt",
        "/dav/%2E%2E/escape.txt",
        "/dav/a%2F%2Fb.txt",
        "/dav/a%5Cb.txt",
        "/dav/c%3Ad.txt",
        "/dav/e%00f.txt",
        "/dav/g.txt?query=1",
        "/dav/h.txt%23fragment",
        "/dav/bad%zz.txt",
    ];
    for target in refused {
        let (status, _) = request(server.addr, "PUT", target, b"data");
        assert_eq!(status, 400, "{target} must be refused");
    }
    assert!(server.file_names().is_empty(), "{:?}", server.file_names());
}

#[test]
fn an_oversized_body_is_refused_before_it_is_read_or_stored() {
    let server = start_manual(small_limits());
    // Only the head is sent: the announced 17 bytes exceed the 16-byte limit and nothing is read.
    let head = b"PUT /dav/big.txt HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: 17\r\n\r\n";
    assert_eq!(send(server.addr, head, b"").0, 413);
    assert!(server.file_names().is_empty());
}

#[test]
fn the_number_of_files_is_limited() {
    let server = start_manual(small_limits());
    assert_eq!(request(server.addr, "PUT", "/dav/one", b"1").0, 201);
    assert_eq!(request(server.addr, "PUT", "/dav/two", b"2").0, 201);
    assert_eq!(request(server.addr, "PUT", "/dav/three", b"3").0, 507);
    assert_eq!(server.file_names(), ["one", "two"]);
}

#[test]
fn the_total_size_is_limited_and_a_replacement_counts_only_once() {
    let server = start_manual(small_limits());
    assert_eq!(request(server.addr, "PUT", "/dav/a", b"123456").0, 201);
    // Six more bytes would make twelve: over the ten-byte total.
    assert_eq!(request(server.addr, "PUT", "/dav/b", b"abcdef").0, 507);
    // Replacing `a` with four bytes gives four in total, which is allowed.
    assert_eq!(request(server.addr, "PUT", "/dav/a", b"1234").0, 204);
    assert_eq!(server.file("a").unwrap(), b"1234");
}

#[test]
fn test_only_routes_and_faults_do_not_exist_on_the_manual_server() {
    let server = start_manual(ManualLimits::default());
    let started = Instant::now();
    // The adapter tests use `/dav/hang` (a three-second delay) and redirects; here they are plain, unknown names.
    let (status, _) = request(server.addr, "GET", "/dav/hang", b"");
    assert_eq!(status, 404);
    assert!(started.elapsed() < Duration::from_secs(2), "no delay route");
    assert_eq!(
        request(server.addr, "GET", "/dav/redirect/cross", b"").0,
        404
    );
    // A file named like a fault switch is stored normally.
    assert_eq!(
        request(server.addr, "PUT", "/dav/locked-full.txt", b"x").0,
        201
    );
}

#[test]
fn stopping_releases_the_port() {
    let server: Server = start_manual(ManualLimits::default());
    let addr = server.addr;
    assert_eq!(request(addr, "OPTIONS", "/dav", b"").0, 200);
    drop(server);
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if TcpStream::connect(addr).is_err() {
            break;
        }
        assert!(Instant::now() < deadline, "the port is still in use");
        thread::sleep(Duration::from_millis(20));
    }
}
