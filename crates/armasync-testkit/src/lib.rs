// SPDX-License-Identifier: AGPL-3.0-only
//! Test support (not shipped): a loopback repository server publishing the MIG-001 `small` metadata with payloads
//! regenerated from the documented generator, misbehaviour switches, a self-signed TLS variant and a server that
//! hangs up. Used by the CLI, workflow and GUI tests; no public server and no user data are involved.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    missing_debug_implementations
)]

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

fn handle(stream: &mut impl Write, target: &str, behavior: &Behavior) {
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

/// Starts a plain HTTP repository at `/repo` publishing the MIG-001 `small` metadata.
#[must_use]
pub fn serve() -> Server {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let behavior = Arc::new(Mutex::new(Behavior::default()));
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
                    requests.lock().unwrap().push((target.clone(), headers));
                    let current = behavior.lock().unwrap().clone();
                    handle(&mut stream, &target, &current);
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
                    handle(&mut tls, &target, &Behavior::default());
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
