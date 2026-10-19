// SPDX-License-Identifier: AGPL-3.0-only
//! Tests of the HTTP(S) adapter against local loopback servers (plain HTTP and TLS with a freshly generated,
//! self-signed test certificate). No real server and no real data are used.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use armasync_adapters::http::{HttpConfig, HttpRepository, redirect_target};
use armasync_core::{CancelToken, ErrorKind, RelativePath, RepositorySource, TRANSFER_CHUNK};

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/legacy-formats/generated/valid/server/small/.a3s")
            .join(name),
    )
    .unwrap()
}

fn path(text: &str) -> RelativePath {
    RelativePath::parse(text).unwrap()
}

fn byte(i: usize) -> u8 {
    (i.wrapping_mul(31) % 251) as u8
}

/// What the server saw of the last requests.
#[derive(Default)]
struct Seen {
    requests: Vec<(String, Vec<(String, String)>)>,
}

struct Server {
    addr: SocketAddr,
    stop: Arc<AtomicBool>,
    seen: Arc<Mutex<Seen>>,
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(self.addr);
    }
}

fn respond(stream: &mut impl Write, status: &str, headers: &[(&str, String)], body: &[u8]) {
    let mut head = format!("HTTP/1.1 {status}\r\nConnection: close\r\n");
    for (k, v) in headers {
        head.push_str(&format!("{k}: {v}\r\n"));
    }
    head.push_str("\r\n");
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(body);
    let _ = stream.flush();
}

/// Serves the routes of the test repository on `stream` (plain or TLS).
fn handle(stream: &mut impl Write, target: &str) {
    let ok_body: Vec<u8> = (0..200_000).map(byte).collect();
    match target {
        "/repo/.a3s/sync" => respond(
            stream,
            "200 OK",
            &[("Content-Length", fixture("sync").len().to_string())],
            &fixture("sync"),
        ),
        "/repo/.a3s/serverinfo" => respond(
            stream,
            "200 OK",
            &[("Content-Length", fixture("serverinfo").len().to_string())],
            &fixture("serverinfo"),
        ),
        "/repo/data/ok.bin" => respond(
            stream,
            "200 OK",
            &[("Content-Length", "200000".into())],
            &ok_body,
        ),
        "/repo/data/short.bin" => respond(
            stream,
            "200 OK",
            &[("Content-Length", "200000".into())],
            &ok_body[..60_000],
        ),
        "/repo/data/nolen.bin" => respond(stream, "200 OK", &[], &ok_body[..150_000]),
        "/repo/data/mismatch.bin" => respond(
            stream,
            "200 OK",
            &[("Content-Length", "5".into())],
            b"hello",
        ),
        "/repo/data/gz.bin" => respond(
            stream,
            "200 OK",
            &[
                ("Content-Length", "5".into()),
                ("Content-Encoding", "gzip".into()),
            ],
            b"hello",
        ),
        "/repo/auth.bin" => respond(
            stream,
            "401 Unauthorized",
            &[("Content-Length", "0".into())],
            b"",
        ),
        "/repo/boom.bin" => respond(
            stream,
            "500 Internal Server Error",
            &[("Content-Length", "0".into())],
            b"",
        ),
        "/repo/redir.bin" => respond(
            stream,
            "302 Found",
            &[
                ("Location", "/repo/data/ok.bin".into()),
                ("Content-Length", "0".into()),
            ],
            b"",
        ),
        "/repo/loop.bin" => respond(
            stream,
            "302 Found",
            &[
                ("Location", "/repo/loop.bin".into()),
                ("Content-Length", "0".into()),
            ],
            b"",
        ),
        "/repo/away.bin" => respond(
            stream,
            "302 Found",
            &[
                ("Location", "http://127.0.0.1:9/x".into()),
                ("Content-Length", "0".into()),
            ],
            b"",
        ),
        "/repo/stall.bin" => {
            let _ = stream.write_all(
                b"HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Length: 100000\r\n\r\n",
            );
            let _ = stream.write_all(&ok_body[..1000]);
            let _ = stream.flush();
            thread::sleep(Duration::from_secs(4));
        }
        "/repo/slow.bin" => {
            let _ = stream.write_all(
                b"HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Length: 2000000\r\n\r\n",
            );
            for chunk in vec![0u8; 2_000_000].chunks(TRANSFER_CHUNK) {
                if stream.write_all(chunk).is_err() {
                    break;
                }
                let _ = stream.flush();
                thread::sleep(Duration::from_millis(20));
            }
        }
        t if t.starts_with("/repo/@m/a%20b%23c") => {
            respond(stream, "200 OK", &[("Content-Length", "3".into())], b"abc")
        }
        _ => respond(
            stream,
            "404 Not Found",
            &[("Content-Length", "0".into())],
            b"",
        ),
    }
}

fn serve() -> Server {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let seen = Arc::new(Mutex::new(Seen::default()));
    let (stop2, seen2) = (stop.clone(), seen.clone());
    thread::spawn(move || {
        for incoming in listener.incoming() {
            if stop2.load(Ordering::SeqCst) {
                break;
            }
            let Ok(mut stream) = incoming else { continue };
            let seen = seen2.clone();
            thread::spawn(move || {
                if let Some((target, headers)) = read_request(&mut stream) {
                    seen.lock()
                        .unwrap()
                        .requests
                        .push((target.clone(), headers));
                    handle(&mut stream, &target);
                }
            });
        }
    });
    Server { addr, stop, seen }
}

fn repository(server: &Server) -> HttpRepository {
    HttpRepository::new(
        &format!("http://{}/repo", server.addr),
        HttpConfig::default(),
    )
    .unwrap()
}

fn fetch(
    repo: &HttpRepository,
    name: &str,
    size: u64,
) -> (Result<u64, armasync_core::Error>, Vec<u8>) {
    let mut out = Vec::new();
    let result = repo.fetch(
        &path(name),
        size,
        &mut out,
        &CancelToken::new(),
        &mut |_| {},
    );
    (result, out)
}

#[test]
fn streams_a_file_in_bounded_chunks_with_progress() {
    let server = serve();
    let repo = repository(&server);
    let mut out = Vec::new();
    let mut steps = Vec::new();
    let written = repo
        .fetch(
            &path("data/ok.bin"),
            200_000,
            &mut out,
            &CancelToken::new(),
            &mut |n| steps.push(n),
        )
        .unwrap();
    assert_eq!(written, 200_000);
    assert_eq!(out, (0..200_000).map(byte).collect::<Vec<_>>());
    assert_eq!(steps.iter().sum::<u64>(), 200_000);
    assert!(steps.iter().all(|n| *n <= TRANSFER_CHUNK as u64));
}

#[test]
fn requests_are_plain_identity_gets_without_credentials() {
    let server = serve();
    let repo = repository(&server);
    fetch(&repo, "data/ok.bin", 200_000).0.unwrap();
    let seen = server.seen.lock().unwrap();
    let (target, headers) = &seen.requests[0];
    assert_eq!(target, "/repo/data/ok.bin");
    let get = |name: &str| {
        headers
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    };
    assert_eq!(get("accept-encoding"), Some("identity"));
    assert_eq!(get("user-agent"), Some("ArmaSync"));
    assert!(get("authorization").is_none() && get("cookie").is_none());
}

#[test]
fn path_segments_are_percent_encoded() {
    let server = serve();
    let repo = repository(&server);
    // Path validation allows spaces and '#'; the request must not turn them into URL syntax.
    let (result, out) = fetch(&repo, "@m/a b#c.pbo", 3);
    assert_eq!(result.unwrap(), 3);
    assert_eq!(out, b"abc");
    assert_eq!(
        server.seen.lock().unwrap().requests[0].0,
        "/repo/@m/a%20b%23c.pbo"
    );
}

#[test]
fn manifest_comes_from_the_published_metadata() {
    let server = serve();
    let repo = repository(&server);
    let manifest = repo.manifest().unwrap();
    assert_eq!(manifest.revision(), "3");
    assert_eq!(manifest.entries().len(), 2);
    let metadata = repo.metadata().unwrap();
    assert_eq!(metadata.unsupported.len(), 1);
    assert_eq!(metadata.protected.len(), 1);
}

#[test]
fn announced_size_must_match_the_file_list_and_the_body() {
    let server = serve();
    let repo = repository(&server);
    // Content-Length differs from the expected size: refused before any byte is written.
    let (result, out) = fetch(&repo, "data/mismatch.bin", 10);
    assert_eq!(result.unwrap_err().kind(), ErrorKind::Integrity);
    assert!(out.is_empty());
    // no Content-Length, more data than expected: exactly the expected amount is written, then Integrity
    let (result, out) = fetch(&repo, "data/nolen.bin", 100_000);
    assert_eq!(result.unwrap_err().kind(), ErrorKind::Integrity);
    assert_eq!(out.len(), 100_000);
    // no Content-Length, less data than expected
    let (result, _) = fetch(&repo, "data/nolen.bin", 200_000);
    assert_eq!(result.unwrap_err().kind(), ErrorKind::Integrity);
}

#[test]
fn a_connection_that_drops_midway_is_a_network_error_with_a_partial_prefix() {
    let server = serve();
    let repo = repository(&server);
    let (result, out) = fetch(&repo, "data/short.bin", 200_000);
    assert_eq!(result.unwrap_err().kind(), ErrorKind::Network);
    assert!(out.len() < 200_000, "the caller must discard this prefix");
}

#[test]
fn status_codes_map_to_network_errors_without_secrets() {
    let server = serve();
    let repo = repository(&server);
    for (name, expected) in [
        ("missing.bin", "not found"),
        ("auth.bin", "refused access"),
        ("boom.bin", "status 500"),
    ] {
        let (result, out) = fetch(&repo, name, 1);
        let error = result.unwrap_err();
        assert_eq!(error.kind(), ErrorKind::Network, "{name}");
        assert!(error.message().contains(expected), "{name}: {error}");
        assert!(out.is_empty());
    }
    let (result, _) = fetch(&repo, "data/gz.bin", 5);
    assert_eq!(result.unwrap_err().kind(), ErrorKind::Network);
}

#[test]
fn redirects_are_limited_and_stay_on_the_same_server() {
    let server = serve();
    let repo = repository(&server);
    let (result, out) = fetch(&repo, "redir.bin", 200_000);
    assert_eq!(result.unwrap(), 200_000);
    assert_eq!(out.len(), 200_000);
    let (result, _) = fetch(&repo, "loop.bin", 1);
    assert!(result.unwrap_err().message().contains("too many redirects"));
    let (result, _) = fetch(&repo, "away.bin", 1);
    assert!(
        result
            .unwrap_err()
            .message()
            .contains("redirect was refused")
    );
}

#[test]
fn redirect_rules() {
    let ok = |from: &str, to: &str| redirect_target(from, to).unwrap();
    assert_eq!(ok("http://h:8080/a/b", "/x"), "http://h:8080/x");
    assert_eq!(ok("https://h/a", "https://h/b"), "https://h/b");
    assert_eq!(
        ok("http://h/a", "https://h/b"),
        "https://h/b",
        "upgrade of the same host"
    );
    assert_eq!(ok("http://h:80/a", "https://h:443/b"), "https://h:443/b");
    for (from, to) in [
        ("https://h/a", "http://h/b"),
        ("https://h/a", "https://other/b"),
        ("http://h/a", "https://other/b"),
        ("https://h/a", "ftp://h/b"),
        ("https://h/a", "b/c"),
        ("https://h/a", "//evil/b"),
        ("https://h/a", "https://user:pw@h/b"),
        ("https://h/a", "https://h/b c"),
    ] {
        assert!(redirect_target(from, to).is_err(), "{from} -> {to}");
    }
}

#[test]
fn a_stalled_transfer_ends_with_a_timeout_instead_of_hanging() {
    let server = serve();
    let config = HttpConfig {
        min_body_timeout: Duration::from_secs(1),
        min_bytes_per_second: 1_000_000,
        ..HttpConfig::default()
    };
    let repo = HttpRepository::new(&format!("http://{}/repo", server.addr), config).unwrap();
    let started = Instant::now();
    let (result, out) = fetch(&repo, "stall.bin", 100_000);
    assert_eq!(result.unwrap_err().kind(), ErrorKind::Network);
    assert!(
        started.elapsed() < Duration::from_secs(6),
        "{:?}",
        started.elapsed()
    );
    assert!(out.len() <= 1000);
}

#[test]
fn cancellation_stops_a_running_transfer_and_is_checked_before_connecting() {
    let server = serve();
    let repo = repository(&server);
    let cancel = CancelToken::new();
    let mut out = Vec::new();
    let started = Instant::now();
    let error = repo
        .fetch(&path("slow.bin"), 2_000_000, &mut out, &cancel, &mut |_| {
            cancel.cancel()
        })
        .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Cancelled);
    assert!(
        out.len() <= 2 * TRANSFER_CHUNK,
        "stopped within a chunk or two: {}",
        out.len()
    );
    assert!(started.elapsed() < Duration::from_secs(3));
    // already cancelled: no request is made at all
    let before = server.seen.lock().unwrap().requests.len();
    let error = repo
        .fetch(
            &path("data/ok.bin"),
            200_000,
            &mut Vec::new(),
            &cancel,
            &mut |_| {},
        )
        .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Cancelled);
    assert_eq!(server.seen.lock().unwrap().requests.len(), before);
}

#[test]
fn a_failing_sink_is_a_file_system_error() {
    struct Full;
    impl Write for Full {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("disk full"))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let server = serve();
    let repo = repository(&server);
    let error = repo
        .fetch(
            &path("data/ok.bin"),
            200_000,
            &mut Full,
            &CancelToken::new(),
            &mut |_| {},
        )
        .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::FileSystem);
}

/// A server that owns its port for the whole test and closes every connection at once. (Binding a port, freeing it
/// and then connecting is not deterministic: another test's server can be handed the same port in between.)
fn hang_up_server() -> (SocketAddr, Arc<AtomicBool>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let stop2 = stop.clone();
    thread::spawn(move || {
        for incoming in listener.incoming() {
            if stop2.load(Ordering::SeqCst) {
                break;
            }
            drop(incoming);
        }
    });
    (addr, stop)
}

#[test]
fn other_schemes_and_unreachable_servers_are_clear_errors() {
    for base in ["ftp://h/r", "sftp://h/r", "file:///x", "h/r"] {
        assert_eq!(
            HttpRepository::new(base, HttpConfig::default())
                .unwrap_err()
                .kind(),
            ErrorKind::Unsupported,
            "{base}"
        );
    }
    let (addr, stop) = hang_up_server();
    let repo = HttpRepository::new(&format!("http://{addr}/repo"), HttpConfig::default()).unwrap();
    let (result, _) = fetch(&repo, "data/ok.bin", 1);
    assert_eq!(result.unwrap_err().kind(), ErrorKind::Network);
    assert_eq!(repo.manifest().unwrap_err().kind(), ErrorKind::Network);
    stop.store(true, Ordering::SeqCst);
    let _ = TcpStream::connect(addr);
}

#[test]
fn read_is_for_small_files_only() {
    let server = serve();
    let repo = repository(&server);
    assert_eq!(repo.read(&path("data/mismatch.bin")).unwrap(), b"hello");
}

// ---- TLS ------------------------------------------------------------------------------------------------------

struct TlsServer {
    server: Server,
    cert_der: Vec<u8>,
}

fn serve_tls(subject: &str) -> TlsServer {
    use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
    let certified = rcgen::generate_simple_self_signed(vec![subject.to_owned()]).unwrap();
    let cert_der = certified.cert.der().to_vec();
    let key = PrivatePkcs8KeyDer::from(certified.signing_key.serialize_der());
    let config = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .unwrap()
    .with_no_client_auth()
    .with_single_cert(
        vec![CertificateDer::from(cert_der.clone())],
        PrivateKeyDer::from(key),
    )
    .unwrap();
    let config = Arc::new(config);
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let seen = Arc::new(Mutex::new(Seen::default()));
    let (stop2, seen2) = (stop.clone(), seen.clone());
    thread::spawn(move || {
        for incoming in listener.incoming() {
            if stop2.load(Ordering::SeqCst) {
                break;
            }
            let Ok(stream) = incoming else { continue };
            let (config, seen) = (config.clone(), seen2.clone());
            thread::spawn(move || {
                let Ok(connection) = rustls::ServerConnection::new(config) else {
                    return;
                };
                let mut tls = rustls::StreamOwned::new(connection, stream);
                if let Some((target, headers)) = read_request(&mut tls) {
                    seen.lock()
                        .unwrap()
                        .requests
                        .push((target.clone(), headers));
                    handle(&mut tls, &target);
                    tls.conn.send_close_notify();
                    let _ = tls.flush();
                }
            });
        }
    });
    TlsServer {
        server: Server { addr, stop, seen },
        cert_der,
    }
}

fn read_request<S: Read>(stream: &mut S) -> Option<(String, Vec<(String, String)>)> {
    let mut data = Vec::new();
    let mut buf = [0u8; 1024];
    while !data.windows(4).any(|w| w == b"\r\n\r\n") {
        let n = stream.read(&mut buf).ok()?;
        if n == 0 {
            return None;
        }
        data.extend_from_slice(&buf[..n]);
    }
    parse_head(&data)
}

fn parse_head(data: &[u8]) -> Option<(String, Vec<(String, String)>)> {
    let text = String::from_utf8_lossy(data).into_owned();
    let mut lines = text.split("\r\n");
    let target = lines.next()?.split(' ').nth(1)?.to_owned();
    let headers = lines
        .take_while(|l| !l.is_empty())
        .filter_map(|l| l.split_once(": "))
        .map(|(k, v)| (k.to_ascii_lowercase(), v.to_owned()))
        .collect();
    Some((target, headers))
}

#[test]
fn an_untrusted_certificate_is_rejected_by_default() {
    let tls = serve_tls("127.0.0.1");
    let repo = HttpRepository::new(
        &format!("https://{}/repo", tls.server.addr),
        HttpConfig::default(),
    )
    .unwrap();
    let (result, out) = fetch(&repo, "data/ok.bin", 200_000);
    let error = result.unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Network);
    assert!(
        error.message().contains("secure connection failed"),
        "{error}"
    );
    assert!(out.is_empty());
    assert!(
        tls.server.seen.lock().unwrap().requests.is_empty(),
        "no request is sent over an untrusted connection"
    );
}

#[test]
fn an_injected_root_makes_exactly_that_certificate_trusted() {
    let tls = serve_tls("127.0.0.1");
    let config = HttpConfig {
        trusted_roots_der: Some(vec![tls.cert_der.clone()]),
        ..HttpConfig::default()
    };
    let repo = HttpRepository::new(&format!("https://{}/repo", tls.server.addr), config).unwrap();
    let (result, out) = fetch(&repo, "data/ok.bin", 200_000);
    assert_eq!(result.unwrap(), 200_000);
    assert_eq!(out.len(), 200_000);
    assert_eq!(repo.manifest().unwrap().revision(), "3");
}

#[test]
fn a_trusted_certificate_for_another_name_is_still_rejected() {
    let tls = serve_tls("not-this-host.invalid");
    let config = HttpConfig {
        trusted_roots_der: Some(vec![tls.cert_der.clone()]),
        ..HttpConfig::default()
    };
    let repo = HttpRepository::new(&format!("https://{}/repo", tls.server.addr), config).unwrap();
    let (result, _) = fetch(&repo, "data/ok.bin", 200_000);
    let error = result.unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Network);
    assert!(
        error.message().contains("secure connection failed"),
        "{error}"
    );
}

#[test]
fn a_different_trusted_root_does_not_trust_the_server_certificate() {
    let tls = serve_tls("127.0.0.1");
    let other = rcgen::generate_simple_self_signed(vec!["127.0.0.1".to_owned()]).unwrap();
    let config = HttpConfig {
        trusted_roots_der: Some(vec![other.cert.der().to_vec()]),
        ..HttpConfig::default()
    };
    let repo = HttpRepository::new(&format!("https://{}/repo", tls.server.addr), config).unwrap();
    assert_eq!(
        fetch(&repo, "data/ok.bin", 200_000).0.unwrap_err().kind(),
        ErrorKind::Network
    );
    assert!(
        HttpRepository::new(
            "https://h/r",
            HttpConfig {
                trusted_roots_der: Some(vec![]),
                ..HttpConfig::default()
            }
        )
        .is_err()
    );
}

#[test]
fn https_against_a_plain_server_fails_cleanly() {
    let server = serve();
    let repo = HttpRepository::new(
        &format!("https://{}/repo", server.addr),
        HttpConfig::default(),
    )
    .unwrap();
    let (result, out) = fetch(&repo, "data/ok.bin", 200_000);
    assert_eq!(result.unwrap_err().kind(), ErrorKind::Network);
    assert!(out.is_empty());
}

/// Manual evidence with the operating system's trust store and real certificates (needs internet access; not part
/// of the normal run): `cargo test -p armasync-adapters --test http_adapter -- --ignored`. Only public test hosts
/// are contacted and nothing is sent but a plain GET.
#[test]
#[ignore = "needs internet access"]
fn system_trust_store_accepts_a_valid_certificate_and_rejects_bad_ones() {
    let get = |host: &str| {
        let repo = HttpRepository::new(&format!("https://{host}"), HttpConfig::default()).unwrap();
        repo.metadata().map(|_| ()).unwrap_err()
    };
    // A valid certificate: the handshake succeeds and the answer is a normal HTTP error (no such file list).
    let ok = get("example.com");
    assert!(!ok.message().contains("secure connection failed"), "{ok}");
    for host in [
        "expired.badssl.com",
        "self-signed.badssl.com",
        "wrong.host.badssl.com",
        "untrusted-root.badssl.com",
    ] {
        let error = get(host);
        assert!(
            error.message().contains("secure connection failed"),
            "{host}: {error}"
        );
    }
}
