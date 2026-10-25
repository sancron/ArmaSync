// SPDX-License-Identifier: AGPL-3.0-only
//! Loopback WebDAV-like test server shared by the WebDAV and upload tests. Plain HTTP or TLS with a freshly generated,
//! self-signed certificate for 127.0.0.1. Synthetic credentials only.
#![allow(clippy::unwrap_used, clippy::expect_used, dead_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use armasync_adapters::http::HttpConfig;
use armasync_adapters::webdav::BasicCredentials;
use armasync_core::RelativePath;

pub const USER: &str = "Aladdin";
pub const PASSWORD: &str = "open sesame";
/// `Basic` plus base64("Aladdin:open sesame"), the example of RFC 7617.
pub const EXPECTED_AUTH: &str = "Basic QWxhZGRpbjpvcGVuIHNlc2FtZQ==";

#[derive(Default)]
struct State {
    files: BTreeMap<String, Vec<u8>>,
    dirs: BTreeSet<String>,
    seen: Vec<Seen>,
    /// Path whose `PUT` is answered with `500` and not stored (fault injection).
    fail_put: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Seen {
    pub method: String,
    pub target: String,
    pub authorization: Option<String>,
}

#[derive(Clone)]
struct Options {
    require_auth: bool,
    /// Absolute target of the `/dav/redirect/cross` route.
    cross_target: String,
}

pub struct Server {
    pub addr: SocketAddr,
    stop: Arc<AtomicBool>,
    state: Arc<Mutex<State>>,
    cert_der: Vec<u8>,
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(self.addr);
    }
}

impl Server {
    pub fn seen(&self) -> Vec<Seen> {
        self.state.lock().unwrap().seen.clone()
    }

    pub fn file(&self, name: &str) -> Option<Vec<u8>> {
        self.state.lock().unwrap().files.get(name).cloned()
    }

    /// Stores a file directly, as if an earlier run had left it there.
    pub fn set_file(&self, name: &str, bytes: &[u8]) {
        self.state
            .lock()
            .unwrap()
            .files
            .insert(name.to_owned(), bytes.to_vec());
    }

    /// Answers every `PUT` to `name` with `500` from now on (fault injection).
    pub fn fail_puts_to(&self, name: &str) {
        self.state.lock().unwrap().fail_put = Some(name.to_owned());
    }

    /// Stops the fault injection.
    pub fn clear_failures(&self) {
        self.state.lock().unwrap().fail_put = None;
    }

    /// Names of all stored files, sorted.
    pub fn file_names(&self) -> Vec<String> {
        self.state.lock().unwrap().files.keys().cloned().collect()
    }

    /// Number of `PUT` requests the server has seen.
    pub fn puts(&self) -> usize {
        self.seen().iter().filter(|s| s.method == "PUT").count()
    }

    pub fn folder(&self, name: &str) {
        self.state.lock().unwrap().dirs.insert(name.to_owned());
    }

    pub fn https(&self) -> String {
        format!("https://{}/dav", self.addr)
    }

    pub fn http(&self) -> String {
        format!("http://{}/dav", self.addr)
    }

    /// Configuration that trusts only this server's test certificate.
    pub fn trust(&self) -> HttpConfig {
        HttpConfig {
            trusted_roots_der: Some(vec![self.cert_der.clone()]),
            ..HttpConfig::default()
        }
    }
}

fn tls_config() -> (Arc<rustls::ServerConfig>, Vec<u8>) {
    use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
    let certified = rcgen::generate_simple_self_signed(vec!["127.0.0.1".to_owned()]).unwrap();
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
    (Arc::new(config), cert_der)
}

/// Starts a WebDAV-like server on loopback. `require_auth` demands [`EXPECTED_AUTH`] on every request.
pub fn start(tls: bool, require_auth: bool, cross_target: &str) -> Server {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let (config, cert_der) = if tls {
        let (config, cert) = tls_config();
        (Some(config), cert)
    } else {
        (None, Vec::new())
    };
    let stop = Arc::new(AtomicBool::new(false));
    let state = Arc::new(Mutex::new(State::default()));
    let options = Options {
        require_auth,
        cross_target: cross_target.to_owned(),
    };
    let (stop2, state2) = (stop.clone(), state.clone());
    thread::spawn(move || {
        for incoming in listener.incoming() {
            if stop2.load(Ordering::SeqCst) {
                break;
            }
            let Ok(mut stream) = incoming else { continue };
            let (config, state, options) = (config.clone(), state2.clone(), options.clone());
            thread::spawn(move || match config {
                None => handle(&mut stream, &state, &options),
                Some(config) => {
                    let Ok(connection) = rustls::ServerConnection::new(config) else {
                        return;
                    };
                    let mut tls = rustls::StreamOwned::new(connection, stream);
                    handle(&mut tls, &state, &options);
                    tls.conn.send_close_notify();
                    let _ = tls.flush();
                }
            });
        }
    });
    Server {
        addr,
        stop,
        state,
        cert_der,
    }
}

struct Request {
    method: String,
    target: String,
    authorization: Option<String>,
    body: Vec<u8>,
}

fn read_request<S: Read + Write>(stream: &mut S) -> Option<Request> {
    let mut data = Vec::new();
    let mut buf = [0u8; 4096];
    let head_end = loop {
        if let Some(pos) = data.windows(4).position(|w| w == b"\r\n\r\n") {
            break pos + 4;
        }
        let n = stream.read(&mut buf).ok()?;
        if n == 0 {
            return None;
        }
        data.extend_from_slice(&buf[..n]);
    };
    let head = String::from_utf8_lossy(&data[..head_end]).into_owned();
    let mut lines = head.split("\r\n");
    let mut first = lines.next()?.split(' ');
    let method = first.next()?.to_owned();
    let target = first.next()?.to_owned();
    let headers: Vec<(String, String)> = lines
        .take_while(|l| !l.is_empty())
        .filter_map(|l| l.split_once(": "))
        .map(|(k, v)| (k.to_ascii_lowercase(), v.to_owned()))
        .collect();
    let header = |name: &str| {
        headers
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.clone())
    };
    let length: usize = header("content-length")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    if header("expect").is_some_and(|v| v.eq_ignore_ascii_case("100-continue")) {
        stream.write_all(b"HTTP/1.1 100 Continue\r\n\r\n").ok()?;
    }
    let mut body = data[head_end..].to_vec();
    while body.len() < length {
        let n = stream.read(&mut buf).ok()?;
        if n == 0 {
            return None;
        }
        body.extend_from_slice(&buf[..n]);
    }
    body.truncate(length);
    Some(Request {
        method,
        target,
        authorization: header("authorization"),
        body,
    })
}

fn respond(stream: &mut impl Write, status: &str, headers: &[(&str, String)], body: &[u8]) {
    let mut head = format!(
        "HTTP/1.1 {status}\r\nConnection: close\r\nContent-Length: {}\r\n",
        body.len()
    );
    for (name, value) in headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str("\r\n");
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(body);
    let _ = stream.flush();
}

fn handle<S: Read + Write>(stream: &mut S, state: &Mutex<State>, options: &Options) {
    let Some(request) = read_request(stream) else {
        return;
    };
    state.lock().unwrap().seen.push(Seen {
        method: request.method.clone(),
        target: request.target.clone(),
        authorization: request.authorization.clone(),
    });
    if options.require_auth && request.authorization.as_deref() != Some(EXPECTED_AUTH) {
        let challenge = vec![("WWW-Authenticate", "Basic realm=\"test\"".to_owned())];
        return respond(stream, "401 Unauthorized", &challenge, b"");
    }
    let target = request.target.as_str();
    if target.starts_with("/dav/hang") {
        // Never answers in time: the client's timeout must end the request.
        thread::sleep(Duration::from_secs(3));
        return respond(stream, "503 Service Unavailable", &[], b"");
    }
    if let Some(rest) = target.strip_prefix("/dav/redirect/same") {
        let location = vec![("Location", format!("/dav/{}", rest.trim_start_matches('/')))];
        return respond(stream, "302 Found", &location, b"");
    }
    if target.starts_with("/dav/redirect/cross") {
        let location = vec![("Location", options.cross_target.clone())];
        return respond(stream, "302 Found", &location, b"");
    }
    let Some(rel) = target.strip_prefix("/dav") else {
        return respond(stream, "404 Not Found", &[], b"");
    };
    let rel = rel.trim_start_matches('/');
    let mut st = state.lock().unwrap();
    let parent = rel.rsplit_once('/').map_or("", |(parent, _)| parent);
    let parent_missing = !parent.is_empty() && !st.dirs.contains(parent);
    let exists = rel.is_empty() || st.dirs.contains(rel) || st.files.contains_key(rel);
    match request.method.as_str() {
        "OPTIONS" => {
            let headers = vec![
                ("DAV", "1".to_owned()),
                (
                    "Allow",
                    "OPTIONS, PROPFIND, MKCOL, PUT, DELETE, GET".to_owned(),
                ),
            ];
            respond(stream, "200 OK", &headers, b"");
        }
        "PROPFIND" => {
            if exists {
                respond(stream, "207 Multi-Status", &[], b"");
            } else {
                respond(stream, "404 Not Found", &[], b"");
            }
        }
        "MKCOL" => {
            if exists {
                respond(stream, "405 Method Not Allowed", &[], b"");
            } else if parent_missing {
                respond(stream, "409 Conflict", &[], b"");
            } else {
                st.dirs.insert(rel.to_owned());
                respond(stream, "201 Created", &[], b"");
            }
        }
        "PUT" => {
            if st.fail_put.as_deref() == Some(rel) {
                respond(stream, "500 Internal Server Error", &[], b"");
            } else if rel.contains("full") {
                respond(stream, "507 Insufficient Storage", &[], b"");
            } else if rel.contains("locked") {
                respond(stream, "423 Locked", &[], b"");
            } else if parent_missing || st.dirs.contains(rel) {
                respond(stream, "409 Conflict", &[], b"");
            } else {
                let existed = st.files.insert(rel.to_owned(), request.body).is_some();
                let status = if existed {
                    "204 No Content"
                } else {
                    "201 Created"
                };
                respond(stream, status, &[], b"");
            }
        }
        "DELETE" => {
            if st.files.remove(rel).is_some() {
                respond(stream, "204 No Content", &[], b"");
            } else if st.dirs.remove(rel) {
                let prefix = format!("{rel}/");
                st.files.retain(|name, _| !name.starts_with(&prefix));
                st.dirs.retain(|name| !name.starts_with(&prefix));
                respond(stream, "204 No Content", &[], b"");
            } else {
                respond(stream, "404 Not Found", &[], b"");
            }
        }
        "GET" => match st.files.get(rel) {
            Some(bytes) => respond(stream, "200 OK", &[], bytes),
            None => respond(stream, "404 Not Found", &[], b""),
        },
        _ => respond(stream, "405 Method Not Allowed", &[], b""),
    }
}

pub fn path(text: &str) -> RelativePath {
    RelativePath::parse(text).unwrap()
}

pub fn credentials() -> BasicCredentials {
    BasicCredentials::new(USER, PASSWORD).unwrap()
}

/// A file in the system temp directory; the name is unique per process.
pub fn scratch_file(name: &str, bytes: &[u8]) -> PathBuf {
    let file = std::env::temp_dir().join(format!("armasync-webdav-{}-{name}", std::process::id()));
    std::fs::write(&file, bytes).unwrap();
    file
}

pub fn payload() -> Vec<u8> {
    (0..200_000)
        .map(|i: usize| (i.wrapping_mul(31) % 251) as u8)
        .collect()
}
