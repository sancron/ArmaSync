// SPDX-License-Identifier: AGPL-3.0-only
//! Test support (not shipped): a loopback repository server publishing the MIG-001 `small` metadata with payloads
//! regenerated from the documented generator, misbehaviour switches, a self-signed TLS variant and a server that
//! hangs up. Used by the CLI, workflow and GUI tests; no public server and no user data are involved.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    missing_debug_implementations
)]

pub mod synthetic;

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use std::fs;

pub const PBO: &str = "@example_core/addons/example_core.pbo";
pub const BIKEY: &str = "@example_core/keys/example.bikey";
pub const ZIP: &str = "@example_core/addons/example_core.pbo.zip";

/// The documented MIG-001 payload generator (`fixtures/legacy-formats/README.md`).
pub fn payload(length: usize, seed: u64) -> Vec<u8> {
    let mut x = 0x9E37_79B9_7F4A_7C15u64 ^ seed;
    (0..length)
        .map(|_| {
            x = x
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (x >> 56) as u8
        })
        .collect()
}

pub fn pbo() -> Vec<u8> {
    payload(8193, 3)
}

pub fn bikey() -> Vec<u8> {
    payload(20, 5)
}

pub fn fixture(name: &str) -> Vec<u8> {
    fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/legacy-formats/generated/valid/server/small/.a3s")
            .join(name),
    )
    .unwrap()
}

/// How the test repository misbehaves (all off by default).
#[derive(Clone, Default)]
pub struct Behavior {
    /// Serve the PBO with its last byte flipped (same length, wrong checksum).
    pub corrupt_pbo: bool,
    /// Answer the PBO request with this HTTP status and no body.
    pub pbo_status: Option<u16>,
    /// Announce the full PBO but close the connection after 4000 bytes.
    pub short_pbo: bool,
    /// Send 1000 bytes of the PBO and then stall for 40 s.
    pub slow_pbo: bool,
    /// Send the PBO in 40 small pieces with 50 ms pauses (about two seconds): a transfer that is slow but keeps
    /// flowing, so cancellation can be observed between pieces.
    pub trickle_pbo: bool,
    /// Like `trickle_pbo`, but 500 ms between the pieces (about twenty seconds): time enough to send Ctrl+C from a
    /// helper process.
    pub very_slow_pbo: bool,
    /// Replaces the MIG-001 metadata and payloads by a synthetic repository (see [`Published`]) when set.
    pub published: Option<Published>,
}

/// A synthetic repository served instead of the MIG-001 fixtures (MIG-030E tests): its own `.a3s/sync` stream and
/// any files below `/repo` (for example packed copies `<name>.zip`). All content is invented for the tests.
#[derive(Clone, Default)]
pub struct Published {
    /// The gzip-compressed `.a3s/sync` stream.
    pub sync: Vec<u8>,
    /// Files below `/repo`, by their path (no leading slash).
    pub files: BTreeMap<String, Vec<u8>>,
    /// Send every file in 16 KiB pieces with 50 ms between them (about 3 s for 1 MiB): a transfer that keeps flowing,
    /// so a cancellation can be requested while it runs (MIG-032B).
    pub trickle: bool,
    /// The `ETag` sent with every file answer (`None`: no validator). Tests set e.g. `"\"v1\""` for a strong one.
    pub etag: Option<String>,
    /// How the server treats `Range` requests (MIG-032R).
    pub ranges: RangeBehavior,
    /// Announce the full answer but close the connection after this many payload bytes (a dropped transfer).
    pub cut_after: Option<usize>,
}

/// How a server answers `Range` requests (MIG-032R, ADR-0009).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RangeBehavior {
    /// `206` with the requested bytes when `If-Range` names the current `ETag`; otherwise the whole file (`200`).
    #[default]
    Honor,
    /// Always the whole file with `200`, as if `Range` were not supported.
    Ignore,
    /// Honours the range but shifts the start in `Content-Range` by one byte (a wrong answer).
    WrongContentRange,
}

/// Requests seen by a server: (target, lower-case headers).
pub type Seen = Arc<Mutex<Vec<(String, Vec<(String, String)>)>>>;

/// A loopback repository server (stops when dropped).
pub struct Server {
    /// Address (127.0.0.1 and a free port).
    pub addr: SocketAddr,
    stop: Arc<AtomicBool>,
    /// Behaviour switches, changeable while running.
    pub behavior: Arc<Mutex<Behavior>>,
    /// Everything the server was asked.
    pub requests: Seen,
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(self.addr);
    }
}

impl Server {
    /// Number of requests whose target ends with `suffix`.
    #[must_use]
    pub fn requested(&self, suffix: &str) -> usize {
        self.requests
            .lock()
            .unwrap()
            .iter()
            .filter(|(t, _)| t.ends_with(suffix))
            .count()
    }

    /// Number of requests.
    #[must_use]
    pub fn total(&self) -> usize {
        self.requests.lock().unwrap().len()
    }
}

fn read_head<S: Read>(stream: &mut S) -> Option<(String, Vec<(String, String)>)> {
    let mut data = Vec::new();
    let mut buf = [0u8; 1024];
    while !data.windows(4).any(|w| w == b"\r\n\r\n") {
        let n = stream.read(&mut buf).ok()?;
        if n == 0 {
            return None;
        }
        data.extend_from_slice(&buf[..n]);
    }
    let text = String::from_utf8_lossy(&data).into_owned();
    let mut lines = text.split("\r\n");
    let target = lines.next()?.split(' ').nth(1)?.to_owned();
    let headers = lines
        .take_while(|l| !l.is_empty())
        .filter_map(|l| l.split_once(": "))
        .map(|(k, v)| (k.to_ascii_lowercase(), v.to_owned()))
        .collect();
    Some((target, headers))
}

fn reply(stream: &mut impl Write, status: &str, body: &[u8]) {
    let head = format!(
        "HTTP/1.1 {status}\r\nConnection: close\r\nContent-Length: {}\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(body);
    let _ = stream.flush();
}

fn handle(
    stream: &mut impl Write,
    target: &str,
    headers: &[(String, String)],
    behavior: &Behavior,
) {
    if let Some(published) = &behavior.published {
        return handle_published(stream, target, headers, published);
    }
    match target {
        "/repo/.a3s/sync" => reply(stream, "200 OK", &fixture("sync")),
        "/repo/.a3s/serverinfo" => reply(stream, "200 OK", &fixture("serverinfo")),
        t if t == format!("/repo/{BIKEY}") => reply(stream, "200 OK", &bikey()),
        t if t == format!("/repo/{PBO}") => {
            let mut body = pbo();
            if let Some(code) = behavior.pbo_status {
                reply(stream, &format!("{code} Error"), b"");
            } else if behavior.slow_pbo {
                let _ = stream.write_all(
                    b"HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Length: 8193\r\n\r\n",
                );
                let _ = stream.write_all(&body[..1000]);
                let _ = stream.flush();
                thread::sleep(Duration::from_secs(40));
            } else if behavior.trickle_pbo || behavior.very_slow_pbo {
                let head = "HTTP/1.1 200 OK
Connection: close
Content-Length: 8193

";
                let _ = stream.write_all(head.as_bytes());
                for piece in body.chunks(205) {
                    if stream.write_all(piece).is_err() {
                        break;
                    }
                    let _ = stream.flush();
                    thread::sleep(Duration::from_millis(if behavior.very_slow_pbo {
                        500
                    } else {
                        50
                    }));
                }
            } else if behavior.short_pbo {
                let head = "HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Length: 8193\r\n\r\n";
                let _ = stream.write_all(head.as_bytes());
                let _ = stream.write_all(&body[..4000]);
                let _ = stream.flush();
            } else {
                if behavior.corrupt_pbo {
                    let last = body.len() - 1;
                    body[last] ^= 0xFF;
                }
                reply(stream, "200 OK", &body);
            }
        }
        _ => reply(stream, "404 Not Found", b""),
    }
}

/// Piece size of a trickled published file.
const TRICKLE_PIECE: usize = 16 * 1024;

fn handle_published(
    stream: &mut impl Write,
    target: &str,
    headers: &[(String, String)],
    published: &Published,
) {
    match target {
        "/repo/.a3s/sync" => reply(stream, "200 OK", &published.sync),
        "/repo/.a3s/serverinfo" => reply(stream, "200 OK", &fixture("serverinfo")),
        other => match other
            .strip_prefix("/repo/")
            .and_then(|path| published.files.get(path))
        {
            Some(body) => serve_file(stream, body, headers, published),
            None => reply(stream, "404 Not Found", b""),
        },
    }
}

/// Answers a file request like a server with validators and ranges (MIG-032R, ADR-0009):
/// * `Range: bytes=<n>-` together with an `If-Range` that equals the current `ETag` gives `206` with the rest of the
///   file (`Content-Range`), unless the behaviour ignores or distorts ranges;
/// * an `If-Range` that does not match, or no range, gives the whole file with `200`;
/// * a start behind the end gives `416`;
/// * `cut_after` announces the full answer and closes after that many payload bytes.
fn serve_file(
    stream: &mut impl Write,
    body: &[u8],
    headers: &[(String, String)],
    published: &Published,
) {
    let header = |name: &str| {
        headers
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    };
    let etag = published.etag.as_deref();
    let start = header("range")
        .and_then(|range| range.strip_prefix("bytes="))
        .and_then(|range| range.strip_suffix('-'))
        .and_then(|first| first.parse::<usize>().ok());
    let if_range_matches = etag.is_some() && header("if-range") == etag;
    let resumed = match start {
        Some(start) if published.ranges != RangeBehavior::Ignore && if_range_matches => Some(start),
        _ => None,
    };
    let etag_line = etag.map_or(String::new(), |tag| format!("ETag: {tag}\r\n"));
    let (status, content_range, payload): (&str, Option<String>, &[u8]) = match resumed {
        Some(start) if start >= body.len() => {
            let head = format!(
                "HTTP/1.1 416 Range Not Satisfiable\r\nConnection: close\r\nContent-Length: 0\r\nContent-Range: bytes */{}\r\n{etag_line}\r\n",
                body.len()
            );
            let _ = stream.write_all(head.as_bytes());
            return;
        }
        Some(start) => {
            let shown = if published.ranges == RangeBehavior::WrongContentRange {
                start + 1
            } else {
                start
            };
            (
                "206 Partial Content",
                Some(format!("bytes {shown}-{}/{}", body.len() - 1, body.len())),
                &body[start..],
            )
        }
        None => ("200 OK", None, body),
    };
    let announced = payload.len();
    let sent = published
        .cut_after
        .map_or(payload, |cut| &payload[..cut.min(payload.len())]);
    let content_range_line =
        content_range.map_or(String::new(), |range| format!("Content-Range: {range}\r\n"));
    let head = format!(
        "HTTP/1.1 {status}\r\nConnection: close\r\nContent-Length: {announced}\r\n{content_range_line}{etag_line}\r\n"
    );
    if stream.write_all(head.as_bytes()).is_err() {
        return;
    }
    if published.trickle {
        for piece in sent.chunks(TRICKLE_PIECE) {
            if stream.write_all(piece).is_err() {
                return;
            }
            let _ = stream.flush();
            thread::sleep(Duration::from_millis(50));
        }
    } else {
        let _ = stream.write_all(sent);
    }
    let _ = stream.flush();
}

/// Starts a plain HTTP repository at `/repo` publishing the MIG-001 `small` metadata.
#[must_use]
pub fn serve() -> Server {
    serve_with(Behavior::default())
}

/// Starts a plain HTTP repository at `/repo` publishing the given synthetic repository.
#[must_use]
pub fn serve_published(published: Published) -> Server {
    serve_with(Behavior {
        published: Some(published),
        ..Behavior::default()
    })
}

fn serve_with(initial: Behavior) -> Server {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let behavior = Arc::new(Mutex::new(initial));
    let requests = Arc::new(Mutex::new(Vec::new()));
    let (stop2, behavior2, requests2) = (stop.clone(), behavior.clone(), requests.clone());
    thread::spawn(move || {
        for incoming in listener.incoming() {
            if stop2.load(Ordering::SeqCst) {
                break;
            }
            let Ok(mut stream) = incoming else { continue };
            let (behavior, requests) = (behavior2.clone(), requests2.clone());
            thread::spawn(move || {
                if let Some((target, headers)) = read_head(&mut stream) {
                    requests
                        .lock()
                        .unwrap()
                        .push((target.clone(), headers.clone()));
                    let current = behavior.lock().unwrap().clone();
                    handle(&mut stream, &target, &headers, &current);
                }
            });
        }
    });
    Server {
        addr,
        stop,
        behavior,
        requests,
    }
}

/// Starts an HTTPS server with a freshly generated **self-signed** certificate for 127.0.0.1 (a client that
/// verifies certificates must reject it).
#[must_use]
pub fn serve_tls() -> Server {
    use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
    let certified = rcgen::generate_simple_self_signed(vec!["127.0.0.1".to_owned()]).unwrap();
    let key = PrivatePkcs8KeyDer::from(certified.signing_key.serialize_der());
    let config = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .unwrap()
    .with_no_client_auth()
    .with_single_cert(
        vec![CertificateDer::from(certified.cert.der().to_vec())],
        PrivateKeyDer::from(key),
    )
    .unwrap();
    let config = Arc::new(config);
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let behavior = Arc::new(Mutex::new(Behavior::default()));
    let requests = Arc::new(Mutex::new(Vec::new()));
    let (stop2, requests2) = (stop.clone(), requests.clone());
    thread::spawn(move || {
        for incoming in listener.incoming() {
            if stop2.load(Ordering::SeqCst) {
                break;
            }
            let Ok(stream) = incoming else { continue };
            let (config, requests) = (config.clone(), requests2.clone());
            thread::spawn(move || {
                let Ok(connection) = rustls::ServerConnection::new(config) else {
                    return;
                };
                let mut tls = rustls::StreamOwned::new(connection, stream);
                if let Some((target, headers)) = read_head(&mut tls) {
                    requests.lock().unwrap().push((target.clone(), headers));
                    handle(&mut tls, &target, &[], &Behavior::default());
                    tls.conn.send_close_notify();
                    let _ = tls.flush();
                }
            });
        }
    });
    Server {
        addr,
        stop,
        behavior,
        requests,
    }
}

/// A server that owns its port for the whole test and closes every connection at once. (Binding a port, freeing it
/// and then connecting is not deterministic: another test's server can be handed the same port in between.)
pub fn hang_up_server() -> (SocketAddr, Arc<AtomicBool>) {
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

/// Loopback WebDAV-like server for the upload tests (ADR-0010, ADR-0011).
pub mod webdav;
